# TODO

Keep completed checklist items as `[x]` alongside unfinished `[ ]` items while their
milestone is in progress. Only remove or archive a milestone once it is fully finished.
Detailed implementation notes, usage and validation history belong in README, design docs
and git history.

## Checks on the desktop

- [ ] With the QHD/HDR monitor (expected mid-October 2026): test three monitors end to end,
  the drawer, All, spanning with mixed resolutions, and 8-bit sRGB wallpaper rendering on HDR.
- [ ] Preset picker: create/apply separate and spanned setups, reopen the picker, and verify
  refreshed assignments and adjustments.
- [ ] Preset display coverage: two/three displays, reordered connector enumeration,
  unplug/replug, restored zoom/pan/flips, and a renderer failure on the desktop.
- [ ] Native-renderer cleanup: repeat strict compositor screenshot comparisons with clear
  desktops; earlier prepared-file and restart checks passed, but windows/shadows obscured
  the screenshot comparisons.

## Features

- [ ] **Animated wallpapers**: GIF/animated WebP first; implementation plan below. Video is
  a separate phase.
- [ ] **UI updates** (`feat/ui-updates`): layout, the control panel, animated wallpapers in
  the picker, a visual refresh; plan below.
- [ ] **Wallpaper control panel**: per-monitor playback controls, first in Adjust, later a
  non-modal side panel; plan below.
- [ ] **Lock and login screens**: use the current wallpaper for hyprlock and display-manager
  login screens.
- [ ] **Wallhaven blacklist**: skip tags and uploaders blacklisted by the account
  (`/api/v1/settings`).
- [ ] **matugen hook** (optional): run matugen on each applied wallpaper.
- [ ] **Simple fade** (low priority): awww's per-channel, fixed-step color convergence;
  duration depends on image differences and step size. Normal Fade already provides a
  predictable crossfade.

## Animated wallpapers

Start with local GIF and animated WebP in the native renderer. Keep static wallpaper behavior
and idle resource use unchanged. Video support follows only after image animation is stable.

Implementation sequence and code/decoder findings: [animated wallpaper design](docs/animated-wallpapers.md).
Start with the media API and decoder qualification, then source-aware batch requests, then
persistent GPU playback and feature integration. Shipping resource defaults require measurements.

### 1. Media model and decoding

- [x] Add content-aware JPEG/PNG/GIF/WebP metadata, static/animated classification,
  representative RGBA frames and sequential composited GIF/WebP decoding. Cache metadata
  by file identity and check identity when opening/advancing streams.
- [x] Enforce encoded-file, dimension, frame-byte and frame-count limits. Isolate worker
  decoding under a hard child address-space limit to contain codec scratch allocations.
- [x] Implement worker admission, two-frame queues, three leased frame slots, generation
  tags, pause, response deadlines and cancellation/reaping. Retained frames keep their
  reservations until released.
- [x] Repeat workers from a fresh decoder at loop boundaries regardless of finite file
  loop counts. Preserve encoded delays and provide the provisional 10 ms minimum.
- [x] Qualify three local wallpaper-sized GIFs in the release build (1920×1080 and
  1925×1075): repeated pixels/timing, sampled RSS, three held frame slots, cancellation
  and replacement cleanup. Keep the assets external and provide an opt-in qualification
  command. See [wallpaper GIF measurements](docs/wallpaper-gif-qualification.md).
- [ ] Qualify isolated decoder workers with desktop-sized/lossy/alpha fixtures and sustained
  replacement. Child limits, small-fixture long-loop checks and initial 1080p GIF runs pass;
  shipping budgets still require measurements. Direct media API limits remain best effort.
  See [decoder worker design and measurements](docs/decoder-workers.md).
- [x] Connect bounded decoder workers to renderer ownership and generation checks in the
  qualified instant-cut path. Replacement, disconnect and shutdown retire workers off-loop;
  obsolete source/output callbacks cannot revive a replaced target.
- [x] Integrate the repeat/delay policy with an injectable monotonic source-session timeline;
  bound catch-up, rebase after starvation, preserve pause phase and retain the last valid
  frame on decoder failure. Consume owned prepared workers without pinning the first frame.
- [x] Surface source-session failures in renderer playback status and connect retirement to
  off-loop cancellation/reaping during replacement, disconnect and shutdown. Qualify late
  source-change freezing and preservation of the old wallpaper on GPU admission failure.

### 2. Source-aware renderer and application

Typed source-aware requests, batch IPC and owned first-frame preparation are implemented,
with static preflight and success-only span commits. The daemon plays prepared sessions
behind the experimental `animation.enabled` setting (off by default), including restoration
and live geometry adjustment without restarting playback. Off, animated sources keep their
identity and the renderer shows their first frame.

- [x] Introduce versioned source-aware batches with original paths, portable geometry,
  application IDs and apply/adjust/restore intent. Migrate apply, presets, spans, Previous,
  adjustments, workspace changes and restoration to the shared contract.
- [x] Negotiate daemon capabilities, preflight static batches and validate per-output
  reports before committing success. Reject incompatible daemons. Static preflight reads
  headers (presets decode completely); preparation errors fail only their output.
- [x] Rebuild resized static crops from original sources and portable geometry.
- [x] Preflight spans before changing state/preferences; commit only successful outputs
  and preserve failed output history during partial application.
- [x] Complete animated first-frame preflight using isolated workers and their reservations;
  validate source identity again before ownership handoff. Share paused workers by source
  identity within a batch and preserve per-output geometry.
- [x] Wire prepared animated sessions through daemon/renderer ownership and application
  acceptance behind `animation.enabled`, including restoration at daemon start and on
  toggling, and live-adjustment session reuse. Admission/texture failures fall back to stills.
- [x] Extend `renderer/gpu/` with persistent playback surface and reusable source-texture
  primitives. Fit/crop/zoom/pan/flips and alpha-correct filtering run on GPU without per-frame
  CPU resizing, disk writes or GPU readback. Bound texture bytes/dimensions and reject
  duplicate/stale uploads. GIF/WebP decoding/compositing remains on CPU.
- [x] Connect persistent GPU primitives to prepared sessions and the Wayland output lifecycle
  for instant cuts. Share uploads, stage incoming layers, acknowledge the first successful
  swap, preserve old content on failure, and bound the shared first-frame barrier. Qualify
  actual EGL resize, rotation, scaling, virtual output removal/re-addition and cleanup.
  See [Wayland playback host](docs/wayland-playback.md).
- [x] Complete incoming animated transition handoff in the qualified renderer path: reuse
  the effect shaders, freeze incoming frame zero and outgoing presentation, wait for each
  exact-endpoint callback, then release the shared clock. Bound callbacks/timers, preserve
  old content on failure and cancel obsolete transitions during replacement/disconnect.
- [ ] Qualify physical mixed-refresh outputs and driver/swapchain memory, and measure
  upload/presentation rates at desktop sizes, including transition overlap.
  See [GPU playback primitives](docs/gpu-playback.md).
- [x] Implement headless output pacing decisions: callback backpressure, independent rate
  caps, shared source sequences and generation/serial rejection. Suppress timers on decoder
  starvation and pause; releasing lock/transition pauses does not clear manual pause.
  See [playback scheduling](docs/playback-scheduling.md).
- [x] Add a calloop playback adapter with one deadline timer, tagged decoder/callback events,
  shared source ownership, per-output draw hooks and bounded off-loop worker retirement.
  Count retirement against admission until cleanup completes; keep static/frozen timers idle.
  Validate real event-loop dispatch, partial draw failure and nonblocking cleanup headlessly.
- [x] Drive presentation with tagged Wayland frame callbacks plus deadline timers. Honor
  source delays, cap presentation rate and drop overdue presentations while retaining correct
  source compositing. Stop requesting callbacks while paused or showing a static image.
- [x] Share decoding and a playback clock within a multi-output application; each output
  draws its own geometry behind its callback gate. Qualify shared uploads, independent
  reapplication, partial failure, scale/rotation and output removal on virtual outputs.
- [ ] Qualify shared playback on physical outputs with different refresh rates.
- [x] Restore playback on hotplugged outputs: the daemon plays an animated wallpaper shown as
  a still on a (re)connected monitor, once per output global (a quick replug counts as new),
  so one that can't play isn't retried every second. It reads what's shown and queues its
  request under the daemon's change lock (every wallpaper change takes it until queued) and
  leaves monitors with a change on its way alone, so a request made meanwhile isn't undone.
  Qualified with kscreen-doctor disable/enable on private outputs.
- [x] Preserve source sessions, application identity and playback position during live
  geometry adjustments in the qualified renderer path. Validate without new decoder workers,
  share existing textures, acknowledge the adjusted swap and reject mismatched live sources.
  Qualify repeated shared/solo adjustments, partial failure and new-clock reapplication.
- [x] Freeze outgoing content and the incoming first frame during wallpaper transitions,
  then start incoming playback at the normal exact-endpoint handoff. Adjustments remain
  immediate. Qualify every effect, supersession, resize and output removal on private
  Wayland outputs with all three downloaded GIFs; verify effect endpoints headlessly.
- [x] Connect transition handoff and adjustment reuse to daemon acceptance; qualify apply,
  adjust, pause/resume, off/on, static return and daemon restart through the real daemon/CLI
  on private outputs (`test-playback-headless.py --daemon [--source FILE]`).
- [x] Qualify mixed static/animated feature flows with playback on: spans, presets (of a
  span), Previous and random (the slideshow's path) through the daemon scenario, with the
  generated WebP and the 4K GIF.
- [ ] Slideshow timer and workspace changes with playback on (need Hyprland: desktop check).
- [x] Without working hardware EGL, show the representative still frame and report playback
  as unavailable in the qualified renderer path. Reuse prepared pixels, cancel decoders on
  the caller, bound retained RGB/fitting storage, and preserve geometry on resize/adjustment.
  Qualify software-EGL rejection, replacement, partial failure and cleanup with all three GIFs.
  See [representative still fallback](docs/representative-fallback.md).
- [x] On a mid-playback decode error, retain the last valid frame and report it.
  Cancel playback on replacement; release workers, textures and surfaces on disconnect/exit.

### 3. Library, picker and existing features

- [x] Scan, thumbnail and apply GIF files (thumbnails use the first frame).
- [ ] Media responses with animated classification, cached by file identity/modification and
  invalidated on changes; audit provider format checks.
- [x] First-frame thumbnails and a film badge on animated tiles (the scan probes GIF/WebP for
  their frame count: `LocalImage.frames`). Details shows the picture; an animated one plays
  there (Qt's AnimatedImage at the preview's size) only while it's open, with its own pause.
- [ ] Verify fits, frame adjustments, standalone cuts, spans, presets, Previous, slideshow,
  workspace automation and daemon restoration. Use the representative first frame for colors
  and Noctalia; do not retheme, append history or notify the picker on every animation frame.

### 4. Playback policy and performance

- [x] Explicit enable and manual pause (settings, CLI, picker; pause acts at once and
  survives restarts and transitions). Provisional limits in `cli/daemon/playback.rs`.
- [x] Presentation-rate control (`animation.max_fps`, live), playback status in the drawer and
  Settings → Animated wallpapers, and renderer GPU/CPU/memory in Settings → Performance.
- [x] Decoder address space scales with frame size (64 MiB + 4 frames): a fixed 128 MiB cap
  aborted a 4K GIF decoder (measured peaks: 29 MiB at 1080p, 111 MiB at 4K).
- [ ] Document proposed defaults after measuring CPU, GPU, RSS and upload bandwidth at
  1080p/QHD/4K with one/three outputs (first desktop sample: 1080p GIF at 6 fps, 0.3% GPU,
  2.3% daemon CPU, 7.6% decoder CPU of one core; `wallpicker-test-4k.gif` for 4K).
  Set byte budgets for decoder working memory, queues and texture copies; verify bounded
  memory over long loops and rapid replacement, including codecs' internal allocations.
- [x] Rest while hidden: when every output of a source has waited 1 s for its frame callback
  (locked, powered off; compositors that skip covered surfaces too), the source pauses
  (`PauseReason::Hidden`, status "hidden") and continues from its held frame without a
  catch-up burst once a callback arrives. Runtime test with withheld callbacks.
- [ ] Check it on the Hyprland desktop (hyprlock, DPMS off, a fullscreen window): KWin keeps
  answering covered surfaces and its virtual outputs ignore DPMS, so the private scenario
  can't. Evaluate per-output fullscreen detection only if Hyprland keeps answering covered
  wallpapers. Manual pause must always work.
- [ ] Static-only sessions retain sleeping event loops and no persistent playback workers or
  textures. Measure power/resource impact before selecting default playback limits.

### 5. Validation and later video support

- [x] Add decoder fixtures for classification, transparency, disposal/blending, variable/zero
  delays, loop boundaries, truncated/corrupt payloads and resource limits.
- [x] Test render-request round trips, original path preservation, static preflight failure,
  transforms, protocol rejection and success-only span updates.
- [x] Test isolated workers with bounded queues, pause/resume, source changes, cancellation,
  forced stalls, process limits and rapid replacement. Run 1,000-loop small-fixture RSS
  qualification for GIF and WebP, including parent RSS checks.
- [x] Test owned animated preflight: shared/independent sources, mixed batches, first-frame
  failure, reservation rollback, source aliases, handoff identity checks and no decode-ahead
  until explicitly resumed. Later corrupt frames preserve the admitted first frame.
- [x] Test GPU geometry and texture reservations headlessly; validate real EGL shader pixels
  for fitting, tiling, crop/frame/flips and alpha edges, shared upload reuse, stale frame
  rejection and texture cleanup. Existing transition shaders still compile.
- [x] Add deterministic clock/catch-up and output-generation/callback rejection tests, plus
  real-worker tests for shared GIF/WebP timelines, decoder stalls, overlapping pause reasons,
  replacement wakes, late corruption and cleanup. Test the calloop adapter with real timers,
  blocked cleanup and shutdown. The Wayland host and daemon now connect these callbacks.
- [ ] Extend headless/service tests to library classification, thumbnails and shared
  playback timelines.
- [ ] Live GPU checks: moving fixtures, all fits/transitions, zoom/pan/flips, mixed spans,
  different refresh rates, replacement during decoding, resize/hotplug, lock/resume,
  no-EGL fallback and shutdown. Check continuity, timing and resource cleanup, not only stills.
- [x] Run actual Wayland/EGL lifecycle checks on private KWin virtual outputs using a
  generated GIF and all three downloaded wallpaper-sized GIFs. Cover shared uploads, crops,
  rotation/scale, output disable/re-enable, repeated replacement, partial application,
  late decode failure, static return, GPU admission rejection and shutdown without changing
  the desktop. Keep physical-output and animated-transition checks open above.
- [ ] Plan MP4/WebM separately: compare decoder dependencies and packaging, software versus
  hardware decoding, codec support, timestamp/seek/loop behavior, YUV conversion, GPU import
  and fallback cost. Local files first, muted playback; no audio/network streaming scope.

## UI updates (planned 2026-10-13)

Plan, findings and mockups: the Roadmap page of the
[design system file](https://www.figma.com/design/gGlwX9OP3fJrNwAvSmChDC/?node-id=24-2).
Keep the core components; change the layout around them. Mockups first, reviewed there.

- [x] Load Nunito: `Theme.font` stayed "sans-serif" (a binding on the Instantiator's
  `objectAt()` never re-ran), so the UI used the system sans since the Quickshell rewrite.
- [x] Decided (2026-10-13): the Adjust dialog is retired for the panel; the panel opens on
  demand from a toggle on the picker's right edge; it is resizable (drag its left edge);
  Filters is a popover (a collapsible row mocked too, to compare).
- [x] Stage 1 mockups final (2026-10-14): the drawer is removed (mockup A: monitors are
  chosen in the bar's pills, adjusted in the panel); Filters as a popover (mockup C), with
  the collapsible row (mockup D) as an alternative picked in Settings → Appearance.
- [x] The open panel narrows the grid at Comfortable and Large (P1; columns animate, every
  tile stays clickable) and overlays it at Compact (P4).
- [x] Stage 2 and 3 mockups final (2026-10-14): "Stage 2 · Control panel" (P1–P4, P6; the
  panel is a floating card with the tiles' hairline border) and "Stage 3 · Animated" (S1–S3).
- [x] Stage 1, picker layout: the drawer is gone (the pills stay in the bar; right-click one
  to adjust), Local's Filters button opens the tags and artists in a popover (or rows, Settings
  → Appearance), Color/Flat/Random turn to icons when narrow. Online/Collections already had
  the one-row card with Filters. The first tile row starts ~143 px down (was ~460).
- [ ] Check stage 1 on the desktop (two monitors, a span, Compact size, the popover's keys).
- [x] Stage 2, the control panel (panel/): a floating card beside the grid following the pills
  (a monitor: preview/framing, Frame, Mirror, Fit, About; All: Mirror and Fit for every
  monitor; the span: framing, Arrange → the Span dialog), footer Previous/Favorite/Details/
  Stop spanning/Reset; edge toggle, resizable 320–560, floats over the grid at Compact (Esc).
  The Adjust dialog is gone; right-clicking a pill opens the panel on it.
- [ ] Check stage 2 on the desktop: two monitors, All, a span (framing, Arrange, Stop
  spanning), resizing, and the Framer at the panel's sizes.
- [x] Stage 3, animated wallpapers in the picker: film badge on tiles, Details preview that
  plays, the panel's Playback section (per-wallpaper pause), a play/pause mark on the pills.
- [ ] Check stage 3 on the desktop: Pause/Play on a shared animation (both monitors), the pill
  marks, a paused wallpaper across a restart, the Details preview of a 4K GIF (memory).
- [x] Stage 4, visual refresh (mockups R1–R4): tiles keep their tags on one line (+N); preset
  cards four across, On now and a calm Not connected; one EmptyState for empty/no-match/error
  views and toasts with the same marks; Settings with section icons, grouped rows (Group,
  Field `side`, ToggleField rows), a Notice, and Unsaved changes/Revert in the footer.
- [ ] Check stage 4 on the desktop (On now with your monitors, every Settings page, toasts).

## Wallpaper control panel (planned 2026-10-12)

Active controls are growing (frame, mirror, fit, now playback). Stage them: extend Adjust
first, then promote its sections into a Wallpaper Engine-like side panel once it needs to stay
open while browsing. Both host the same section components, so stage 2 moves no logic.

### 1. Playback (built straight into the control panel, 2026-10-14)

- [x] A Playback section for the monitor's animation (panel/PlaybackSection.qml): state and
  fps, Pause/Play, why it's a still, Try again after a decoding error. Only for animations.
- [x] Pause per wallpaper: a `paused` adjustment (core `Adjust`, render `Picture`), held by a
  new `PauseReason::Wallpaper` while any monitor showing the session says so; independent of
  the global pause. Status reports `held`.
- [x] Temporary like adjustments: in `state.json`, cleared with the wallpaper, restored paused
  from its first frame (private-KWin check). `wallpicker adjust MONITOR --pause|--play`.

### 2. Side panel

- [x] Done as stage 2 of the UI updates (panel/): follows the pills (no switcher of its own),
  reflows the grid, overlays at Compact, replaced the Adjust dialog and the drawer.
- [ ] About: loop length and decoder memory (the probe gives the frame count only).

### Later playback options (not scheduled)

- [ ] Playback speed (×0.5–2, scaling source delays on the shared clock).
- [ ] A chosen still frame while paused or with playback off, instead of frame one.
- [ ] Per-wallpaper preferences that survive reapplying (start paused, own fps cap), stored
  with the library entry; only if the temporary per-monitor state proves insufficient.

## Wallpaper presets

Implemented: versioned storage, stable IDs, CRUD, multi-monitor application, picker/CLI,
monitor reassignment with preview, and backup export/import plus daily snapshots.

- [ ] Standalone preset export/import for sharing individual setups.
- [ ] Finish documentation and desktop validation before marking the feature complete.
  README already covers usage, missing displays, automation, reassignment and backups;
  remaining desktop checks are listed above.

Keep these contracts when extending presets:

- Save original image paths, effective fits, flips, zoom/pan, standalone source cuts and
  span membership/canvas/bezel spacing. Output-sized frames and cuts are rebuilt on apply.
- Presets are explicit snapshots; live adjustments do not edit them. IDs survive rename,
  update and reassignment. Reassignment never changes the compositor's monitor layout.
- Skip absent outputs and incomplete spans, retaining their saved assignments. Leave other
  monitors alone; never silently reassign or shrink a span, or auto-apply on hotplug.
- Preflight all required images before changing wallpapers. Renderer failures may produce
  partial application; report failures and remember only successful assignments.
- Use normal history and transitions. Slideshow/workspace automation continues; group undo
  is outside the current scope. Saving a preset does not change live state or preferences.
- Older library-only backups leave presets untouched. Preserve missing original paths,
  report them and reuse image relocation rules. Backups do not bundle wallpaper files.
- If an active-preset indicator is added, derive it from the current setup rather than
  keeping a flag that becomes stale after adjustments or automation.

## KDE Plasma Wayland compatibility

Implemented: shared `wl_output`/`xdg-output` monitor discovery; Hyprland focus/workspace
metadata is optional enrichment. KDE testing is required before claiming support.

- [ ] Verify fractional scale, rotation and refreshed monitor snapshots after hotplug on
  real KDE displays.
- [ ] Test wallpaper stacking in a real Plasma session: background layers must remain
  visible without breaking desktop icons, clicks, widgets or panels. Decide whether
  Plasma-specific integration is needed.
- [ ] Remove the alpha-modifier requirement from shader transitions. Provide shader Fade
  when compositor opacity is unavailable; retain compositor Fade where supported.
- [ ] Verify temporary-layer stacking and final buffer handoff on KWin, including
  replacement, cut, resize, disconnect and shutdown. Preserve the exact-endpoint handoff
  that fixes Slide ghosting on Hyprland.
- [ ] Replace the picker's Hyprland-only focused-monitor selection with a portable fallback;
  make selection behavior explicit when focus cannot be queried.
- [ ] Hide unsupported per-workspace controls on other compositors initially. Investigate
  KDE virtual desktops/activities and their wallpaper semantics before implementing support.
- [ ] Update compositor-specific UI text, autostart instructions and documentation; verify
  daemon startup, restoration, slideshow and spanning under Plasma Wayland.

## More wallpaper sources

Check API terms and image rights before planning a provider. Unsplash prohibits wallpaper
applications and was removed (2026-10-07). Wallpaper Abyss/Alpha Coders was ruled out
(2026-10): old APIs return 404, the replacement offers paid enterprise integration only;
no scraping.

- [ ] **Pexels / Pixabay**: stock photos (Pixabay also illustrations). Check terms for
  wallpaper applications; free keys are rate-limited.
- [ ] **Daily images**: Bing wallpaper and NASA APOD. Check terms and individual image
  rights; APOD images are not all public domain.

Provider integration: another Moebooru site needs a `Site` in `moebooru.rs` and a `Provider`.
Other sources need core query/response mapping to `OnlineWallpaper`/`Page`, search/details
in `service/online/`, allowed image hosts in `Provider::download_prefixes`, online filters
and `online.options`, and a settings section.

## Metadata follow-up

Implemented: origin/source/uploader/artists metadata, download enrichment, artist filtering
and local details/editing. Tags describe pictures separately from artist identities.

- [ ] Extend migration to Konachan and yande.re after the Wallhaven pass: fetch posts by ID
  (`post.json?tags=id:N`) and classify artist tags with `tag.json`.
- [ ] Artist selection for slideshow and `wallpicker random --artist`.

Migration safeguards: standard-library Python; API key in headers, cached responses,
40 requests/minute with 429 backoff, removed files reported. Default dry run; `--refresh`
refetches, `--overwrite` replaces existing metadata. Keep non-artist tags, back up the catalog,
re-read before writing and save atomically; refuse version-1 catalogs.

## Going public (if ever)

- [ ] **AUR package** (`wallpicker-git`): cargo build; install binary, picker UI under
  `/usr/share/wallpicker/ui`, desktop entry and icon; depend on quickshell.
- [ ] **systemd user service** for `wallpicker daemon`, as an alternative to Hyprland autostart.

## Completed milestones

- Content-aware media foundation, sequential composited GIF/WebP frames, first-frame API,
  metadata cache and decoder qualification fixtures. Playback integration remains open.
- Versioned source-aware batch requests, explicit application intent, static resize from
  originals, daemon capability checks and success-only span commits.
- Isolated decoder workers with hard child address-space limits, bounded admission/frames,
  cancellation/reaping, generation tags, pause and repeated-loop RSS qualification fixtures.
- Native renderer with fitting/scaling, compositor Fade and GLES wipe/wave/circle/slide/random
  transitions; immediate live adjustments and corrected Slide handoff.
- Selected-monitor spanning on a physical canvas, bezel spacing, framing and separation.
- Preset storage/application/picker, monitor reassignment, and backup coverage.
- Metadata model, download enrichment, artist filters and local details.
- External wallpaper backends, Noctalia picker plugin and prototypes removed.
- Backup file-dialog round trip and Noctalia color-switch desktop checks completed.

## Decisions to keep

- **One renderer**: wallpicker renders wallpapers. Noctalia remains only for wallpaper-derived
  colors; competing wallpaper-daemon detection remains.
- **Rendering**: shm for steady images, GLES for shader transitions. No per-frame CPU pixel
  blending or GPU readback. Missing hardware EGL falls back to compositor Fade; without
  compositor opacity, the current fallback is a cut. Keep rendering errors contained in its
  own thread. Quickshell does not render wallpapers persistently.
- **Modules**: use `module/mod.rs` plus siblings; aim below 300 lines, up to 400 for complex code.
- **No tray**: open the picker from a keybind or launcher.
- **Daemon**: owns slideshow ticks, workspace listening, login restoration and Noctalia palette.
- **Closing**: keep the picker hidden for a configurable Off–30 min (default 1 min), then exit
  to 0 MB.
- **Picker size**: Large, Comfortable or Compact as a share of the screen, preserving the
  1100×760 aspect ratio so 720p displays work.
