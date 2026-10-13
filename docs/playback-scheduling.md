# Playback scheduling and source ownership

Implemented in `crates/wallpicker/src/renderer/playback/`, re-exported as
`wallpicker::playback` for tests without a compositor. These components consume real prepared
sessions; the daemon plays them behind the experimental `animation.enabled` setting, with a
renderer-wide manual pause (`Runtime::set_manual_pause`). The calloop adapter dispatches
timers, decoder notifications and callback tickets through the Wayland `State` draw/swap
hook. The host creates real EGL surfaces and a bounded handoff barrier, including frozen
incoming transitions and their exact-endpoint callbacks. See [Wayland playback host](wayland-playback.md).

## Source timing and ownership

`SourceSession::new` consumes an owned preparation session and starts paused for transition
handoff. It retains the session key (application plus file identity), worker generation,
one current leased RGBA frame, and a `Timeline`. Advancing replaces that frame, releasing
its slot; the initial frame is not pinned for the lifetime of playback. Multiple outputs
borrow this same current frame and use their own geometry and pacing gates.

Timeline arguments are `Duration` elapsed from a single monotonic epoch (the calloop adapter
uses `Instant::elapsed`). Tests inject these durations. Timeline samples clamp
backwards input and use saturating arithmetic. The core media policy supplies positive frame
delays with a provisional 10 ms minimum; the timeline also guards its public input.

- A deadline is when the current source frame expires. Early wakes do not consume frames.
- When late, consume at most four sequential composited frames per dispatch. If still late,
  rebase the current frame's expiry to `now + delay`; never loop through unlimited debt.
- An empty overdue decoder queue suppresses its timer. A subsequent worker notification
  retries the read and starts the newly available frame at `now`, without catch-up debt.
- Pause preserves the remaining duration by shifting the deadline on resume. Transition,
  manual, session-lock, no-visible-output and hidden reasons are independent; decoding resumes
  only after every reason clears. Manual is the user's pause (`animation.paused`); hidden is
  set by the runtime when every healthy output has waited `HIDDEN_AFTER` (1 s) for its frame
  callback (`pacing::visibility`), and cleared by the next callback. Session-lock is unused:
  a locked compositor stops answering, which hidden covers.
  Resuming a starved source schedules a retry, including when its worker notification was
  already handled during pause and no new notification would arrive from the full queue.
- The source rejects stale worker wakes without changing its clock. Decoder generation and
  sequential frame order are checked before replacing the current frame.
- Decoder failure freezes the last valid frame, retains the error, pauses scheduling and
  cannot be cleared by a resume request. Unexpected queue closure is also an error.

Workers continue to composite every source frame, including disposal/blending and loop resets.
Presentation skips do not seek past source frames. The existing three leased slots bound
receiving, queued and retained frames together; no unbounded history is introduced.

`SourceSession::into_worker` releases the current frame and transfers worker ownership without
joining its thread. The calloop adapter retires this worker through its off-loop cleanup
owner; dropping a standalone session directly still performs synchronous cancellation/reaping.
Admission remains reserved until worker and frame leases are released. Shipping resource
budgets remain unselected.

## Per-output pacing

`OutputPacer` takes a target's source/output generations and an explicit minimum presentation
interval. Zero disables the rate cap while retaining callback backpressure. Each gate keeps
only sequence/time/serial metadata, never RGBA allocations or a source clock.

After advancing the shared source, `plan` returns:

| Decision | Event-loop action |
| --- | --- |
| `Present(ticket)` | Upload the current source frame once if needed, draw this output, and associate its callback with the ticket. Record `presented` after a successful swap. |
| `WakeAt(time)` | A newer source frame exists, but the output cap has not elapsed. Arm a deadline, without requesting another callback. |
| `Idle` | No new sequence, paused source, pending callback or obsolete generation. Do not request a refresh. |

`frame_callback(ticket)` opens the gate only for that exact pending presentation. Serial tags
reject old/duplicate callbacks even within the same surface generation. Source/output tags
reject replaced targets. Recreating an output for a surface or geometry change must increment
its generation. A failed swap must not be recorded as presented.

The owner combines source deadlines and output cap deadlines into its next timer. A callback
received before the next source deadline leaves the same sequence idle; the source deadline
provides the next wake. A slow output samples the latest sequence when its callback arrives,
without blocking other outputs. Paused/static targets need no periodic scheduling. Geometry
redraws and frozen transition endpoints remain the lifecycle owner's responsibility.

## Calloop adapter and retirement

`Runtime<Host>` installs a notification channel and at most one deadline timer into an
existing calloop event loop. It owns source sessions and attached output gates. Its `Signals`
provide the callback passed to `Preparer::prepare` and an entry point for tagged compositor
callbacks. Late worker wakes, replaced target callbacks and obsolete timer serials are ignored.
Source, output-cap and hidden deadlines share the same timer; empty, frozen, paused and hidden
owners leave it disarmed. A cap crossed while another output is drawing schedules another dispatch.

The host implements `present(ticket)`: borrow the source's current pixels, reuse/upload the
shared texture, draw the target, associate a Wayland callback with that ticket, and swap. The
driver records a presentation only after success. It draws an attachment's initial frozen frame
once even with transition pause held; the lifecycle owner releases that pause after endpoint
handoff. The draw hook must not synchronously change source/output ownership. Draw failure
disables only that output. When no healthy consumers remain, decoding and deadlines pause.

`detach` retires a source when its last output disappears; `retire` also handles preparations
that never gained an output. One cleanup thread receives workers through a bounded channel,
kills/reaps them, joins their pumps and notifies calloop of completion. Retiring sources still
count against driver capacity until that acknowledgement. The capacity is explicit and should
match shared decoder admission. Rejected insertions return their preparation to the caller,
which must dispose of it off-loop. A failed cleanup handoff retains worker ownership and
blocks new admission instead of joining in dispatch or growing an unbounded backlog.

Construct the runtime lazily when an animated owner is needed; static rendering needs neither
this channel nor its cleanup thread. Drop it after event-loop dispatch has ended: shutdown
unregisters timers/notifications and joins both active and already-retiring workers. The driver
contains no Wayland or GPU objects, so their destruction order remains the host's responsibility.

## Validation and remaining integration

Fourteen deterministic tests exercise variable/minimum delays, bounded catch-up, starvation,
pause phase, backwards/overflow inputs, early callbacks, caps (and changing them), hidden
outputs, slow outputs, generation and serial rejection, paused outputs and failed swaps. Five
real-worker integration tests exercise GIF/WebP repetition shared by two output gates,
independent pause reasons, a SIGSTOP/SIGCONT decoder stall, replacement notifications, late
corruption and reservation cleanup. Injected source time avoids timing-dependent animation
assertions; only worker delivery has a timeout.
Seven additional tests run the actual calloop adapter with real decoder workers and a recording
draw hook. They verify timer-driven shared playback, idle/frozen behavior, stalled callbacks,
resting while every callback is withheld, partial draw failure, replacement generations,
bounded retirement, shutdown and responsiveness
while a deliberately blocked decoder pump prevents cleanup from completing.
Two further adapter tests cover per-output suspension during replacement and unbinding a
configured member while an unconfigured member still needs the shared source. The explicit
private-compositor suite exercises the real Wayland/GPU host, including rotation and output
removal, with generated media and the three local wallpaper-sized GIFs.

```sh
cargo test -p wallpicker --offline renderer::playback
cargo test -p wallpicker --offline --test playback
cargo test -p wallpicker --offline --test playback_runtime
```

The Wayland host also reuses source sessions during geometry adjustments, staging a fresh
output target while keeping the outgoing one active until the adjusted swap succeeds.

The representative-still fallback releases decoders on the caller and does not create the
playback runtime when EGL initialization fails. See [fallback ownership](representative-fallback.md).

The daemon accepts animations behind `animation.enabled`, with pause and frame-rate controls,
and plays them again on monitors connected later. Desktop timing, lock/DPMS behavior on
Hyprland, shutdown and resource measurements remain acceptance work.
