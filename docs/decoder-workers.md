# Isolated animation decoder workers

Implemented on 2026-10-11 in `crates/wallpicker/src/decoding/`. This is a headless
foundation; owned first-frame batch preparation is implemented, and the daemon starts workers
only for animated requests while `animation.enabled` is on (one admission pool for its
lifetime, provisional limits in `cli/daemon/playback.rs`). Static requests start none.

## Memory and admission

The child's cap is `address_space_bytes + frame_allowance × RGBA frame bytes`. The daemon uses
64 MiB plus four frames (97 MiB at 1080p, 196 MiB at 4K). Measured on 2026-10-12 with the
release qualification helper: a 3840×2160, 60-frame GIF peaked at about 111 MiB RSS (21 frames
a second with hashing), against 29 MiB at 1080p. A fixed 128 MiB cap aborted the daemon's
debug-build 4K decoder (SIGABRT in the child; playback froze on its last frame, as designed).

`image-webp 0.2.4` documents allocations that ignore its memory limit. Rather than treating
its limit as a guarantee, each worker execs this wallpicker binary's private `__decode`
entry point. Before exec, the parent installs equal soft/hard `RLIMIT_AS` limits and disables
core dumps. The limit covers the child's executable, libraries, stack, canvases and codec
scratch. Limit exhaustion may return a codec error or terminate the child; either is reported
to the consumer without invoking the codec in the daemon. There is no in-process fallback.

Linux enforces `RLIMIT_AS` on address-space growth, including `mmap` and `brk`; this is a
virtual address-space limit, not the ineffective Linux `RLIMIT_RSS` control.
See [getrlimit(2)](https://man7.org/linux/man-pages/man2/getrlimit.2.html).
Tests verify both the child's installed limits and an actual over-budget `mmap` rejection.
The direct `wallpicker_core::media` API still offers only best-effort codec allocation limits.

A shared `Budget` reserves the following before starting a worker:

```text
child address-space cap + 3 × source width × source height × 4 + 1 MiB control allowance
```

Admission limits both bytes and workers atomically. The control allowance covers the
256 KiB pump stack, request/error storage and small protocol/channel bookkeeping. GPU
textures, transition surfaces, kernel socket/page-table memory and unrelated daemon
allocations need separate accounting. This reservation is not a whole-daemon RSS limit;
the parent's allocator may retain freed blocks.

Three leased slots cover **all** parent frame allocations: receiving, queued and retained
by consumers. Only two frames can queue. Keeping frames stalls decoding when no slot is
available; it cannot grow an unbounded history. Frames expose borrowed pixels and may be
shared through `Arc<DecodedFrame>`. Copies explicitly made by consumers are their own
responsibility. Reservations remain charged after worker cancellation until every retained
frame is dropped, including the worker count (a deliberately conservative policy).

There are no shipping worker defaults yet. Tests use a 128 MiB child cap and a 256 MiB,
one-worker pool. Larger media can need a larger cap or be rejected; these test values are
not an advertised maximum resolution or a desktop playback configuration.

## Protocol and lifetime

The parent sends a length-bounded request over a private Unix socket with the original
path, inspected identity and media limits. The child validates identity before decoding;
the existing decoder verifies it while advancing, and the parent checks it before handing
each complete frame to the consumer. These checks detect replacement/modification but
do not create an immutable snapshot against concurrent writes.

One command byte permits one frame. Replies contain a fixed header and source-sized RGBA
pixels, or a bounded error. The parent validates dimensions before allocating. A total
response deadline covers each frame, including startup/probing for the first response.
There is no whole-animation collection, output-sized cache, resize, disk write or readback.
Socket transport does copy RGBA bytes; measure its CPU/bandwidth cost before selecting
playback defaults or considering shared memory.

Each result carries the caller's generation and a monotonically increasing sequence.
The future renderer owner must check generations and associate workers with application
ID plus source identity. A worker itself does not know which outputs are still current.
It repeats from a fresh decoder at EOF regardless of finite file loop metadata; original
frame delays remain intact and the media API's provisional 10 ms minimum remains available.
There is no presentation clock or catch-up scheduler in this layer.

Pause stops new decode requests; at most one in-flight frame may complete. Cancellation
disconnects queue waiters, wakes slot waiters, shuts down the socket, kills/reaps the child,
and joins the pump. No detached workers accumulate during replacement. Dropping a worker
performs the same cleanup; retained frames stay valid. This cleanup is synchronous, so
renderer integration must account for reaping/joining without putting decode work on the
event loop. A terminal decode error is delivered once after any already-queued valid frames.

The child also receives `SIGKILL` if the spawning pump thread dies. It is spawned on that
long-lived thread because Linux's parent-death signal follows the creating **thread**.
The pre-exec setup checks for parent death between fork and signal installation, and a
pump-side RAII guard reaps on unwinding.
See [PR_SET_PDEATHSIG(2const)](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html).

## First-frame preparation and handoff

`Worker::prepare` decodes exactly one frame and pauses before publishing it. The caller
receives the first frame and the still-live worker together; only an explicit resume permits
the second frame. This keeps later payload failures out of first-frame preflight and holds
the incoming frame steady while presentation/transition ownership is established.

`wallpicker::preparation::Preparer` validates a complete typed batch before starting workers.
It fully preflights static originals/prepared files and checks animated container metadata,
request identity and source geometry. Equal file identities within an application share a
worker/first frame; each output retains its own geometry. Separate applications allocate
new sessions and globally distinct generations. Admission or first-frame failure drops all
earlier sessions before returning an error. Static-only batches require no decoder budget.

`PreparedBatch` owns the workers and leased first frames until consumed or dropped. Its
`handoff()` checks every source alias and prepared static path again, then transfers the
batch, output-to-session indices and sessions. `Session::into_parts()` lets the
presentation owner release the first frame after transition handoff rather than pinning it
throughout playback. A separately retained first frame keeps its reservation even after the
worker is dropped. File identity remains a change detector, not an immutable snapshot.

The daemon's static preflight now uses this same ownership contract. The qualified Wayland
host connects animated presentation, output generations, shared clocks and GPU lifetime.
`PreparedAdjustment` validates geometry and file identity without admitting another worker;
the renderer must match an existing live session. Public daemon acceptance, restoration,
and public feature flows remain integration work; qualified transitions/fallback now work,
but the public capability stays disabled.

## Qualification

The normal suite covers real GIF/WebP exec and transport, repeated finite-loop sources,
encoded timing, generation tags, retained-frame backpressure, pause/resume, source changes,
late corrupt payloads, admission failure before spawning, process-limit failure, malformed
response lengths, concurrent reservation, rapid replacement, forced stalls/timeouts,
cancellation during reads/queue waits and complete reaping/reservation release.

Run the explicit RSS soak separately:

```sh
cargo test -p wallpicker --test decoding_worker --offline \
  soak::long_loops_keep_child_rss_stable -- --ignored --nocapture
```

It warms up for 100 frames, then decodes 1,000 two-frame loops per codec at 320×180. It
samples child and parent RSS every 50 frames and checks for sustained growth; admission
must return to zero after teardown. Debug-build measurements are recorded below. These
small solid-color fixtures qualify repeated teardown/compositing behavior, not real-world
throughput, worst-case scratch usage or shipping budgets.

Measured on 2026-10-11 with the 128 MiB child limit:

| Codec | Child RSS after warmup | Sampled peak | Final child RSS | Parent RSS warmup / peak | 1,000 loops |
| --- | ---: | ---: | ---: | ---: | ---: |
| GIF | 16.32 MiB | 16.71 MiB | 16.32 MiB | 7.20 / 7.20 MiB | 14.38 s |
| Lossless WebP | 17.42 MiB | 17.43 MiB | 17.42 MiB | 7.20 / 7.20 MiB | 16.66 s |

Three real wallpaper-sized GIFs now pass release-build qualification at 1920×1080 and
1925×1075, including complete-loop pixel/timing checks, RSS sampling, held-frame backpressure
and replacement/cancellation cleanup. See [wallpaper GIF measurements](wallpaper-gif-qualification.md)
for results and the opt-in `qualify_animation` example; inputs remain outside the repository.

Desktop-sized lossy/lossless WebP, alpha/disposal-heavy content, concurrent sources, longer
replacement runs and eventual GPU/transition overlap still require measurement. The OS cap
remains enforced for those inputs even when decoding cannot fit.
