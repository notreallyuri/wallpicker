# Animated wallpaper implementation plan

Status: media API, decoder fixtures, source-aware static requests, isolated bounded workers
and owned first-frame preflight implemented. Persistent GPU presentation primitives pass
headless EGL checks. Source ownership and clock/output scheduling decisions pass headless
tests. A calloop adapter now drives real timers/notifications and bounded off-loop cleanup;
the Wayland host now passes playback and incoming transition checks with hardware EGL on private virtual
outputs. Live geometry adjustments reuse source sessions and textures without resetting
playback. Incoming transitions freeze frame zero through the shared endpoint barrier.
Representative-frame fallback now passes private Wayland checks with software EGL rejected.
The daemon plays animations behind the experimental `animation.enabled` setting, with manual
pause, restoration and live adjustments; off, animated sources are shown as their first
frame. Measured defaults, hotplug restoration and physical-output qualification remain.
Reviewed on 2026-10-12.
The feature checklist remains in [TODO.md](../TODO.md#animated-wallpapers).

## Scope and constraints

Local GIF and animated WebP, rendered by wallpicker. Single-frame GIF and ordinary WebP
use the still-image path. Video, audio and network streaming remain separate work.
Static sessions must retain sleeping event loops and release decoded images as today.
Quickshell owns only the picker preview; closing it must not affect wallpaper playback.

Keep original paths in state, history, presets and backups. First-frame thumbnails, colors
and Noctalia updates occur once per application/source change, never per playback frame.
Playback repeats indefinitely; file loop counts remain metadata rather than a stop policy.

## Findings before the request refactor

This baseline informed the plan. Section 2 records the completed request and state changes;
the playback worker, GPU and representative-frame consumer changes remain open.

| Area | Current behavior | Required change |
| --- | --- | --- |
| `core/src/wallpapers/drawing.rs`, `transform.rs` | `Backend::shown` bakes cuts, framing and flips into cache images. | Preserve source and transforms in animated requests; retain static preparation. |
| `core/src/cmd.rs`, `daemon.rs`; `wallpicker/src/cli/daemon.rs` | The request contains one monitor, path, fit and transition. | Typed, versioned requests with application intent and batch identity. |
| `core/src/presets/plan.rs` | Preflights selected sources and prepares static copies before applying. | Prepare first frames and geometry for every selected animated source before application. |
| `core/src/wallpapers/span.rs` | Saves current paths and span state before per-output application. | Prepare first, apply, then commit successful assignments; test partial spans explicitly. |
| `wallpicker/src/renderer/state.rs` | Spawns still-image workers; generation checks discard obsolete results. | Separately owned playback workers, cancellation, bounded admission and queues. |
| `wallpicker/src/renderer/gpu/target.rs` | One incoming, output-sized texture and temporary EGL surface per transition. | Persistent playback targets with source-sized reusable textures and transform uniforms. |
| `wallpicker/src/renderer/drawing.rs` | Transition completion attaches the incoming shm buffer. | Handoff to either a static buffer or a frozen playback target before starting its clock. |
| `core/src/library.rs`, `thumbs.rs` | Extension allowlist excludes GIF; thumbnails use `image::open`. | Content classification and an explicit representative-frame API. |

The existing per-monitor generation is useful for invalidation, but it does not cancel work,
bound memory or identify a shared playback session. These are separate responsibilities.

## 1. Media foundation and decoder qualification

Add `crates/core/src/media/mod.rs` with focused siblings for probing, GIF/WebP adapters,
limits and tests. Keep codec types private. Expose file identity, format, dimensions,
static/animated classification, representative RGBA frame and sequential timed frames.
Use source-resolution, fully composited frames with origin `(0, 0)`.

Inspect magic bytes; extensions can shortlist library candidates but cannot determine
playback. Treat an unreadable or malformed candidate as an error, not a static fallback.
Detect single-frame GIF without collecting frames. Cache metadata by file identity, length
and high-resolution modification time; do not copy the current seconds-only thumbnail key.
Recheck identity at preparation/open and reject or reprepare a changed source.

The installed sources are `image 0.25.10` and `image-webp 0.2.4`:

- `image`'s GIF iterator composites transparency and Keep/Background/Previous disposal
  into full RGBA canvases. It maintains a previous canvas and can allocate a frame patch
  and a further full canvas. The media foundation enables the GIF Cargo feature.
- `image`'s WebP wrapper detects animation and yields composited frames, but allocates
  output buffers per frame and does not expose the inner decoder's memory control.
- `image-webp` exposes `read_frame`, frame count, delays and reset. Its source explicitly
  says some allocations ignore `set_memory_limit`; a bounded queue alone is insufficient.
- `reset_animation` retains canvas state. Initially reopen a decoder at loop boundaries
  to guarantee a clean canvas; qualify reset reuse with partial-frame fixtures separately.

Qualification selected `image`'s GIF iterator. The `image-webp 0.2.4` animation compositor
failed a fixture where a full-canvas blended frame follows disposal of a partial frame:
it cleared the whole canvas. Its reciprocal-based blending also darkened test colors.
The adapter therefore uses `image-webp` to decode individual compressed patches and owns
the RGBA canvas, rectangle disposal and integer source-over blending. A seekable view
synthesizes a 30-byte still-WebP header over each patch without copying compressed frames.
Behavior follows the [WebP container specification](https://developers.google.com/speed/webp/docs/riff_container#animation),
with transparent canvas/background disposal as the wallpaper policy.

GIF decoding is enabled; the library scans GIF files and thumbnails their first frame, and
source-aware requests carry them to the renderer. The API also handles JPEG, PNG, ordinary WebP
and single-frame GIF/WebP as stills. Animated PNG is outside this implementation's scope.

`Source::open(path, limits)` validates container metadata; `representative()` decodes its
first composited RGBA frame; `frames()` returns a fused one-pass iterator. Later compressed
payload errors terminate the stream without changing frames already owned by the caller.
Reopen the stream for a new loop. Encoded delays remain available; `playback_delay()` applies
the provisional 10 ms minimum. File loop counts are normalized to total plays, independent
of the future wallpaper repeat policy. `MetadataCache` is a bounded metadata-only LRU keyed
by canonical path and device/inode/size/high-resolution mtime/ctime identity. Opening and
advancing streams rejects detected source changes; identity checks are not immutable snapshots.

Foundation defaults: 64 MiB encoded file, 16,384 pixels per axis, 64 MiB per RGBA frame,
10,000 frames per pass and 256 MiB best-effort codec allocation limit. These are configurable
API limits, not measured playback defaults. WebP checks its canvas/patch/output allocation
estimate as well. GIF/WebP container probes check patch dimensions before decoding, and
bounded readers reject seeks beyond file/frame boundaries. Codec-internal scratch memory,
caller-held frames and concurrent streams are not covered by a strict process-memory limit.

The playback worker foundation now contains codec allocations in a fresh Linux process with
a hard `RLIMIT_AS` limit installed before exec. It reserves child address space and parent
frame capacity before spawning. Direct core API calls retain their best-effort limits.
See [decoder workers](decoder-workers.md) for the ownership/IPC contract, cancellation,
small-fixture RSS measurements and remaining desktop-sized qualification.

Enforce checked canvas/patch dimensions, encoded-file size, frame count/work bounds and
decoder allocation limits before expensive allocation where possible. Audit header parsing
and codec scratch memory too. If the adapter cannot enforce the agreed working-memory
bound, qualify a different adapter, patch the codec or isolate decoding before claiming
bounded operation. Panic catching does not contain an allocator abort.

Acceptance: deterministic fixtures for still/animated classification, misleading extensions,
transparent first frames, offsets, every disposal mode, WebP blend/replace, varying and zero
delays, finite loop metadata, clean loop boundaries, truncation and oversized headers.
No full-animation collection and no output-sized frame cache.

## 2. Source-aware requests and preparation

Implemented in `core/src/render/`: versioned `Batch` carries an application ID, intent,
transition and per-output `Picture` (original source, geometry and static/animated content).
The runtime grouping key for animation is application ID plus file identity; an extra group
number is unnecessary. Apply, presets, spans, Previous, reapply, workspace changes and
restoration use the contract. Static preparation still uses cache files; resize resolves
portable geometry against the new output size and rebuilds from the original source.

The daemon negotiates `render_version: 1`, preflights static batches before queuing a single
renderer message, and reports per-output results. Clients validate complete result coverage
before recording success. Older clients' static `show` requests remain accepted; a new client
reports an incompatible running daemon and requires a restart. Saved state and backups keep
original paths and existing geometry only, never application IDs or prepared cache paths.

Spans now preflight before state/preferences change and commit only successful assignments;
partial application leaves successful crops standalone and preserves failed output history.
Restored outputs share one runtime restoration identity. Animated content retains its source
identity and geometry, but admission explicitly rejects it before application. Animated
first-frame preflight now acquires and retains isolated workers/reservations in an owned
`wallpicker::preparation::PreparedBatch`. It validates every source/geometry before starting
workers, groups equal identities within the application, and decodes exactly one frame per
group. Workers pause before publishing that frame, so preflight cannot race into decoding
frame two. Failure drops/reaps all admitted workers, leaving desktop/state unchanged.

Ownership handoff rechecks every source alias and prepared static path. It transfers the
batch, per-output session indices and paused sessions without copying pixels or losing their
reservations. Distinct applications get distinct sessions/generations. The static daemon path
uses this same owned preflight and handoff. Animated handoffs now pass private Wayland/GPU
checks. `PreparedAdjustment` preflights without new workers, and the renderer reuses the
matching live source session and texture, preserving playback and application identity.
Public daemon acceptance and restoration integration remain open.
Legacy/static-shaped requests are also checked for animation.

The initial conceptual design below explains the intended playback contract; concrete type
names and grouping reflect the implementation described above.

Introduce shared request types in `core/src/render/mod.rs`. Conceptually:

```text
RenderBatch { application_id, intent, outputs, transition }
OutputRequest { monitor, original_source, prepared_content }
PreparedContent = Static { prepared_path, fit }
                | Animated { source_identity, geometry, playback_group }
Geometry { fit, flip, view: Full | PortableFrame | SourceRegion }
Intent = Apply | Adjust | Restore
```

Names may change during implementation. Preserve transform order from `transform.rs`:
source cut or frame, then flips. Resolve portable framing against current output dimensions;
preserve standalone source-region coordinates and the span's physical canvas/bezel model.
Resize updates geometry/targets without restarting a source. Restoration reconstructs
requests from saved originals; runtime session IDs and prepared paths are not persisted.

Migrate Runner, daemon IPC, renderer `Picture`/`Saved`, presets, ordinary apply, Previous,
adjustments, spanning, restoration, slideshow and workspace application together.
Retain legacy static requests when practical; new clients must detect an old daemon's
missing capability and explain the required restart instead of silently flattening animation.

All targets in an application share one batch identity. Group equal source identities within
that batch, with independent groups for different sources. Reapplying a source creates a
new session; adjusting it preserves its session and clock. Path equality alone must not join
unrelated applications. Reconstruct saved span groups when restoring the daemon.

Preflight selected sources, first frames, geometry and resource reservations before changing
the desktop. Full validation of every future compressed frame is not part of first-frame
preflight; later decode errors retain the last valid frame and appear in playback status.
Static preflight reads headers outside presets, whose contract decodes every image before
changing the desktop. Report per-output failures and remember only
successful assignments, retaining the established missing-output/incomplete-preset-span rules.

Acceptance: IPC round trips/capability mismatch, original-path preservation, static transform
regressions, apply versus adjust semantics, preflight failure without state changes, partial
application, and restoration/resize with the correct source geometry.

## 3. Playback ownership, timing and GPU presentation

`renderer/gpu/playback/` now implements a persistent Wayland EGL surface wrapper, shared
source-resolution textures, a reusable shader/quad and portable source-to-output mapping.
Textures allocate once and update with `tex_sub_image_2d`; generation/sequence checks reject
stale frames and avoid duplicate shared uploads. Source textures have explicit byte-budget
reservations. Fit, framing, source regions and flips run in the shader; four premultiplied
samples provide bilinear filtering onto opaque black without transparent-color fringes.

Headless EGL checks passed on the Radeon GPU for pixel correctness, shared uploads and
cleanup; the transition shaders also compile through the shared compiler. The qualified
Wayland host connects these primitives to session clocks, virtual-output resize/hotplug and
adjustment continuity. All incoming transition effects now reuse the source texture and
preserve frozen-frame/exact-endpoint semantics. Hardware EGL failure now displays a budgeted
representative still with no playback workers/timers. Physical display/swapchain costs remain open. See
[GPU playback primitives](gpu-playback.md) for API ownership and validation details.

The worker foundation lives in `wallpicker/src/decoding/`, allowing headless preparation
and future representative-frame consumers to use the same isolation.
`wallpicker/src/renderer/playback/` now owns prepared source sessions, an injectable clock
and per-output pacing decisions (publicly re-exported as `wallpicker::playback` for headless
tests). It retains one current frame, limits catch-up to four consumed frames per dispatch,
waits for notifications after starvation, and preserves remaining duration across independent
pause reasons. Output gates reject obsolete generations/callback serials and hold no pixels.
Real GIF/WebP workers exercise repeated shared playback, stalls and late-error freezing.
Its calloop adapter owns the deadline timer, tagged notification channel, output draw hooks
and bounded cleanup queue; tests run it with real timers and a recording presentation host.
See [playback scheduling](playback-scheduling.md) for API contracts and integration boundaries.
The Wayland host connects GPU targets and shared texture ownership for playback and transitions. See
[Wayland playback host](wayland-playback.md) for ownership, acceptance and qualification scope.

- One admitted decoder worker per source session, shared by its outputs. Admission reserves
  bytes globally and per session; rapid replacement cannot accumulate cancelled workers.
- Begin with at most two queued source frames, additionally bounded by bytes. Account for
  decoder canvases/scratch, queued frames, retained current frame, GPU textures and transition
  overlap separately. A slow output must not pin an unbounded history of frames.
- Send lightweight wakeups to calloop; frame storage belongs to the bounded queue. Check
  cancellation between decode steps and while waiting for queue space. Attach session and
  output generations to results; drop obsolete results even after cancellation was requested.
- Use a monotonic timeline, preserving positive source delays separately from presentation
  limits. Proposed test policy: clamp delays below 10 ms to 10 ms; benchmark before shipping.
  Presentation-rate caps drop presentations, never required source compositing operations.
- Bound catch-up decoding per wake. If the decoder cannot keep pace, rebase the timeline
  after bounded work rather than spinning indefinitely through missed frames. Pause/resume
  shifts the epoch so there is no accumulated catch-up debt.
- Present only when both the source deadline and the output's frame callback permit it.
  A callback received before the next deadline arms a timer; it must not request continuous
  refresh callbacks while the same frame remains due. Static/paused targets stop scheduling.
- Upload selected RGBA frames once per shared source texture where the existing EGL context
  permits sharing; each output samples its crop. Use reusable storage/subimage uploads and
  GPU fit/crop/flip transforms. Composite source alpha onto black consistently with the still
  fallback. No per-frame CPU fitting, disk writes, BGRX conversion or GPU readback.

Freeze the outgoing displayed content and incoming first frame for transitions. For a shared
batch, hold playback until successful members complete endpoint handoff; remove failed or
disconnected members from that barrier. Bound an occluded member's wait so suppressed
callbacks cannot block the whole batch indefinitely. Begin one clock for the remaining
members; different refresh rates may sample different presentations of that timeline.
Preserve the exact final transition frame that currently prevents Slide ghosts.

Without hardware EGL, show the first-frame still and report the fallback reason. A later
decode error freezes the last valid frame. Release workers, queue reservations, timers,
textures and surfaces when their last consumer disappears or the renderer exits.

Acceptance: fake-clock deadline/cap/late-frame tests; pause and cancellation under queue
backpressure; bounded worker admission; two outputs sharing a timeline; adjustment continuity;
transition handoff; decode failure; disconnect/replacement and no-EGL still fallback.

## 4. Existing features, controls and desktop qualification

Route thumbnails, framing previews, color extraction and Noctalia through the representative
frame. Add media classification and an animated badge to library responses. Keep grid
thumbnails still. Verify installed Quickshell image plugins and timing before choosing an
animated details preview; load it only while visible and otherwise explain the still preview.
Audit provider format filters without expanding this phase into new online/video providers.

Expose enable, manual pause and presentation-rate controls plus playback/fallback/error
status. Keep manual pause separate from session-lock pause so unlocking never overrides it.
Verify a supported session-lock signal and resume without catch-up. Fullscreen visibility
detection is a separate Hyprland-specific qualification, with unknown visibility explicit.

Measure one/three outputs at 1080p, QHD and 4K: CPU, GPU, RSS, upload bytes, frame lateness,
paused wakeups and static idle behavior. A single 3840×2160 RGBA frame is about 31.6 MiB;
30 uploads/s is about 949 MiB/s before driver copies. Source sharing and dropping uploads
therefore matter even with GPU drawing. Set shipping byte budgets and rate defaults from
these results, including decoder scratch and transition peaks.

Run the existing core/service/CLI suites after request migration; add fixture-driven tests
where new behavior is introduced. Desktop checks cover fits, flips, framing, source cuts,
spans, presets, Previous, slideshow, workspace changes, restart, mixed refresh/scale/rotation,
resize/hotplug, lock/resume, replacement during decode and shutdown. Soak long loops and
rapid replacements; verify RSS returns near baseline after switching back to static.

## First implementation slice

The media API, compositing fixtures, first-frame semantics, input/frame limits and versioned
source-aware static request path now exist. Isolated decoder workers enforce child memory
limits and bounded admission; small-fixture loop/cancellation checks pass. Owned batch
preflight now holds paused workers and representative frames through an identity-checked
handoff. Persistent GPU drawing primitives now pass headless EGL checks. Source-session
ownership, monotonic timing and output pacing now pass deterministic and real-worker tests.
The calloop adapter dispatches deadlines/notifications and retires workers without blocking
live presentation. Its Wayland/GPU host now accepts prepared applications and transitions through
an explicit qualification entry point, with real callback pacing and virtual-output lifecycle
checks, adjustment continuity and representative-frame fallback. The daemon accepts them
behind `animation.enabled` (`cli/daemon/playback.rs`: provisional limits, pause, restoration).
Next: physical-output/resource qualification, measured defaults and library classification.
Do not mark animated wallpaper support complete until feature integration, manual pause,
fallbacks and desktop/resource checks pass. Video receives its own dependency and decoding
design after this path is stable.
