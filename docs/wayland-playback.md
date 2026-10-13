# Wayland playback host

Implemented and qualified on 2026-10-12 in `renderer/animation.rs`. The renderer now consumes
prepared animation sessions and presents their RGBA frames on persistent Wayland/EGL surfaces.
The explicit internal qualification entry point accepts instant cuts, live adjustments and
incoming animated transitions. Public CLI/daemon animation capabilities remain disabled until
daemon admission and playback controls are connected. No shipping resource
defaults were selected by this work.

## Application and ownership

The caller retains the prepared handoff until the renderer has checked runtime admission and
created/uploaded every source texture. A failure returns ownership to that caller, where
workers can be cancelled and joined without blocking the Wayland loop. Successful admission
moves sessions into the calloop runtime, sharing one decoder, clock and texture per source
identity within the application. Source reservations and GPU texture reservations stay distinct.

Each incoming output gets a new background layer and viewport. Its old wallpaper stays intact
until layer configuration, EGL setup and the first successful swap. The renderer suspends only
that output's previous playback gate while preparing the replacement; other outputs using
the old source can continue. Failed preparation removes the incoming layer and resumes the old
target. For instant cuts, a successful swap promotes the incoming layer to the persistent layer, releases
the old EGL window before its layer role, and acknowledges application to the caller.

The shared source starts after all incoming members have either committed their first swap,
failed, disconnected or reached the configured first-presentation timeout. Missing output names
fail individually and never enter this barrier. Successful members remain applied. Animated
transitions extend the barrier to the final opaque swap and its frame callback.

## Animated transition handoff

`animation/transition.rs` drives Fade, Wipe, Slide, Grow, Outer and Wave using the existing
effect masks and the playback shader's geometry sampler. Fade also uses shader alpha, so
incoming animation effects do not need the compositor's alpha-modifier protocol. The incoming
source texture stays at its first frame, with its decoder paused. No second output-sized
texture, per-frame conversion or GPU readback is required.

The outgoing surface retains its last submitted image without further draws. Other outputs
sharing that source may continue playing: their texture uploads cannot change an already
submitted outgoing surface buffer. The incoming layer is promoted in place only after the
exact opaque endpoint is swapped and its tagged callback arrives. It then attaches to the
ordinary playback pacer. The shared source clock remains paused until all incoming members
have completed, failed or disconnected. Adjustments bypass effects and retain playback.

Each transition has at most one outstanding callback and one pacing deadline; a deadline is
armed only after its callback completes. Source/output generation and serial checks reject
obsolete callbacks. A total handoff deadline combines the configure timeout and the capped
transition duration (at most ten seconds). Failure removes the incoming layer and resumes
the outgoing target; supersession removes both timers and retires unused sources off-loop.
Physical mode changes also refresh incoming EGL dimensions even without a layer configure.
A resized endpoint must draw again before promotion; stale outgoing static resize results
are invalidated at handoff.

Static members retain their existing transition path and compositor fallback policy. Public
mixed application/restore flows, physical compositor screenshots and stalled-compositor fault
injection remain qualification work.

## Live adjustments

`PreparedAdjustment` validates source identities, geometry and any static members without
admitting decoder workers. Handoff rechecks every path off-loop. `Renderer::adjust_batch`
matches each animated member against the output's current source identity, retaining its
original application ID, decoder, clock and source texture. Missing outputs or mismatched
sources fail individually; an adjustment cannot cancel a pending Apply/Restore request.
Static members use the existing immediate drawing path.

For an animated adjustment, the renderer stages a new output layer using the current source
frame and requested geometry. The outgoing target keeps playing until the adjusted swap
succeeds; failure or timeout preserves it. A successful swap replaces only the output target,
without restarting the source or allocating another source texture. Repeated pending
adjustments supersede the preceding request and reject stale callbacks. This also presents
the current frame if its source is paused, without clearing manual or session-lock pauses.
The temporary EGL window/swapchain overlap still needs physical-driver memory measurements.

The public daemon uses this preflight path for static adjustments and retains its animated
capability gate. Ordinary `PreparedBatch` animation application still creates fresh sessions;
its entry point rejects Adjust intent to prevent accidental decoder restarts.

## Drawing, callbacks and output changes

`State` implements the playback runtime's host interface. It borrows the current source frame,
uploads its RGBA bytes only if the shared texture sequence changed, applies output geometry on
the GPU and swaps the persistent surface. A custom Wayland callback carries the exact source,
output generation and presentation serial. It is requested after drawing and before swapping;
its completion passes through the runtime's stale-ticket checks. The old untagged callbacks used
by static transitions remain separate.

There is no per-frame CPU fitting, cache-file write, image conversion or GPU readback. The
source decoder still composites every frame. Multiple output draws of the same sequence share
one upload; caps and callbacks control each output's presentation independently of source time.

Output rotation and mode changes resize the existing EGL window. A fresh output generation
forces a redraw with current geometry and rejects old callbacks while preserving the source
clock. Viewports/buffer scale follow the normal renderer placement rules. Source attachment
is renewed before the previous gate is removed so a sole consumer cannot accidentally retire
its source during resize. A replacement already in progress keeps its outgoing target frozen.

The host, rather than the runtime's ordinary last-output heuristic, decides when a source can
retire: unconfigured incoming layers still count as consumers. Removing the final current or
incoming consumer releases its texture and queues decoder retirement off-loop. Shutdown drops
EGL/native windows before their layer roles and joins active/retiring workers after dispatch.
Returning all outputs to static leaves zero source textures, decoder reservations and playback
deadline timers. The reusable EGL context/painter and idle cleanup owner can remain cached
after animation has been used; a static-only renderer never constructs them for playback.

## Status and failures

Renderer status includes source/output generations, current and uploaded frame sequence,
upload/presentation counters, active geometry, pending replacement and a playback error/state. Global fields
report runtime admission, playback-timer presence and logical texture bytes. Each output's
`animation_transition` includes its incoming source generation/sequence, upload/draw counters,
elapsed progress and pending endpoint. Transition pacing/handoff timers are separate from the
shared playback timer. Counters are diagnostic
metadata, not persistence/history updates on every source frame.

A late decoder failure freezes presentation and keeps the last valid content. A draw failure
freezes the affected output and allows other healthy consumers to continue. GPU admission
failure preserves the previously applied wallpaper. When hardware EGL initialization fails,
the caller converts the already prepared first frames into budgeted representative stills,
releases the decoders and queues SHM presentation. Resize/adjustments reuse those stills and
status reports playback as unavailable. Texture/admission exhaustion remains an error that
preserves old content. See [representative fallback](representative-fallback.md) for its limits.
Application/restore plumbing still uses the public animation gate.

## Private-compositor validation

```sh
python3 scripts/test-playback-headless.py
python3 scripts/test-playback-headless.py --source "$HOME/Pictures/Wallpapers/775121.gif"
python3 scripts/test-playback-headless.py --source "$HOME/Pictures/Wallpapers/869910.gif"
python3 scripts/test-playback-headless.py --source "$HOME/Pictures/Wallpapers/1q3f7z5fv7hf1.gif"
```

The script requires KWin, `dbus-run-session`, `kscreen-doctor`, `timeout`, `fusermount3` and Cargo.
It builds the worker/test binary, starts KWin with two **virtual** 960×540 outputs on a private
D-Bus session, Wayland socket and XDG directories, then runs only the opted-in animation test.
Rotation, scaling and output disable/enable commands target that private connection. It stops
the compositor/process group and removes its temporary resources, including a document-portal
mount if a Qt helper activated one. It does not connect the test renderer to the user's desktop,
modify wallpaper state or overwrite the input assets. Source-change checks use a temporary copy.

This passed on an AMD Radeon RX 9060 XT using Mesa radeonsi, with a generated 7×5 GIF and all
three wallpaper-sized GIFs. It covers:

- Two outputs sharing one worker/clock/texture, with distinct crop geometry and shared uploads.
- Actual hardware EGL window swaps, rotation to 540×960 and scale changes to 270×480 logical size.
- Reapplication with a new clock while the other output retains its previous application.
- Shared and solo adjustment continuity for framing, fit and flips, with unchanged source
  generation/application/texture bytes; source mismatch fails only its requested output.
- Repeated replacement, partial application with an absent monitor, and output disable/re-enable.
- Every incoming transition effect over static/animated wallpapers, frozen frame-zero uploads,
  outgoing presentation freeze, endpoint acknowledgment and shared-clock release. Cancellation,
  rotation during a static-to-animated transition and output removal during handoff also pass.
- Late source-change errors freezing playback, static return and source/texture cleanup.
- Shutdown during animation and GPU texture admission rejection preserving the old wallpaper.

These checks establish lifecycle behavior and successful swaps, not captured compositor-pixel
equivalence or physical display latency. The separate headless EGL pixel suite covers shader
geometry and alpha behavior. Virtual outputs do not qualify physical mixed-refresh displays,
VRR, driver/swapchain memory, session lock or visible compositor stacking; those TODO entries remain
open. The daemon accepts animations behind the experimental `animation.enabled` setting.
