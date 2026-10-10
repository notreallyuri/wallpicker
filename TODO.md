# TODO

What's done is in the README and git history (this file's earlier versions hold the finished
plans: the Quickshell port's phases, spanning, framing, backup, color filter).

## Checks on the desktop

Things the headless checks can't reach (cage has no portal and no layer-shell panel):

- [ ] Settings → Backup: export and import through the system's file dialog, round trip.
- [x] Settings → Appearance: the "Noctalia's colors follow the wallpaper" switch, by click.
- [ ] When the QHD/HDR monitor arrives (mid-October 2026): three monitors end to end
  (the drawer, All, spanning with mixed resolutions), and how HDR affects wallpapers under each
  backend.

## More wallpaper sources

Before planning one, read its API terms: Unsplash's forbid wallpaper applications ("you cannot
replicate the core user experience of Unsplash (unofficial clients, wallpaper applications,
etc.)"), so it was built and taken out (2026-10-07).

Adding one: another Moebooru site is one `Site` in `moebooru.rs` plus a `Provider`. Anything
else needs a core module (query → params, response → `OnlineWallpaper`/`Page`), a branch in
`service/online/` (search, details), its image host in `Provider::download_prefixes`, a filters
file in `quickshell/views/online/` (its choices in `serve`'s `online.options`), and a section in
`quickshell/views/settings/`.

- [ ] **Pexels** / **Pixabay**: stock photos (Pixabay also illustrations). Free keys,
  rate-limited. Check their terms for wallpaper apps first.
- [ ] Daily images, not searchable: Bing's daily wallpaper, NASA's Astronomy Picture of the
  Day. Check the terms and the images' rights (APOD's are often, not always, public domain).
- Not possible: **Wallpaper Abyss** (Alpha Coders, checked 2026-10): the old APIs return 404,
  the new one only offers paid enterprise integrations by email; scraping is out.

## Features

- [ ] **Animated wallpapers** (needs a plan): add GIF/animated WebP playback to the native
  renderer, first-frame library thumbnails with a ▶ badge, and an animated details preview.
  Plan video decoding (mp4/webm), CPU/GPU cost and pausing under fullscreen separately.
- [ ] **Lock and login screens**: the current wallpaper for hyprlock and display-manager login screens.
- [ ] **Wallhaven blacklist**: skip the tags and uploaders your account blacklists (from
  `/api/v1/settings`).
- [ ] **Wallpaper presets**: named wallpaper setups for all monitors, with their adjustments
  and spanning arrangement. Implementation plan below.
- [ ] **matugen hook** (optional): run matugen on each applied wallpaper, for setups themed with
  it.

## Wallpaper presets (planned 2026-10-09)

Save the wallpaper setup as a named preset, then restore it with one action. Working name:
**Wallpaper presets**; actions **Save preset** and **Apply preset**. A preset describes
wallpaper placement, not the compositor's monitor configuration: applying it never moves,
rotates or changes the resolution of the actual desktops.

### Behavior and scope

- Capture original image paths and assignments by monitor name, effective fit modes,
  horizontal/vertical flips, zoom and pan. Resolve the current global/monitor fit when saving,
  so changing Settings later does not silently change a saved preset's appearance.
- Capture separate/spanned mode, selected span monitors, span zoom/pan/flips, wallpaper canvas
  positions and bezel spacing. Support a span on selected monitors alongside independent
  wallpapers on the others. Left/right and top/bottom placement belongs to this saved canvas.
- Use the native renderer and current transition when applying; presets do not change
  backend, theme preferences, transition settings, slideshow or workspace settings.
- Presets are snapshots: later adjustments do not edit them. Updating a preset explicitly
  replaces its saved setup. Rename, delete and update refer to a stable ID, independent of its
  display name. Reject empty names and accidental duplicate names.
- Match connected displays by connector name. Skip absent monitors and report them; retain
  their assignments in the preset. Leave connected monitors not included in the preset alone.
  Never silently assign a missing monitor's wallpaper to another monitor.
- A span requires its saved monitor group to be connected. Initially skip an incomplete span
  as a group, report which monitors are absent, and still allow independent assignments to
  apply. Do not silently shrink/rearrange the span or trigger the existing "all monitors"
  fallback. Explicit monitor reassignment is a later phase.
- Check all images needed by connected assignments before application. Missing/unreadable
  files prevent application with a clear report; keep the saved preset intact. Do not make
  partial changes merely because an image is missing. Backend failures after preflight are
  reported per monitor; multi-monitor application is best effort, not an atomic compositor
  transaction. Remember only assignments that actually succeeded.
- Apply through normal wallpaper history and notifications. Per-monitor **Previous** keeps
  its current meaning; group undo is not part of the first release. Slideshow/workspace
  automation may subsequently replace a preset's wallpapers and is not silently disabled.

### 1. Core model and storage

- [x] Add `crates/core/src/presets/mod.rs` with versioned, serde-backed preset types; separate
  capture, persistence, validation and tests into sibling files. Use directory `mod.rs`
  structure and keep each file below 300 lines where practical.
- [x] Store a versioned `presets.json` in wallpicker's data directory using the existing atomic
  JSON-write conventions. Missing file means an empty collection; reject malformed data and
  unsupported versions without overwriting them. Add stable IDs and validated names; file
  locking serializes edits from different processes.
- [x] Implement list, save-current, explicit update, rename and delete. Capture only wallpaper
  configuration, not the whole `Settings`/`State` or histories. Saving requires at least one
  known assignment and does not alter the current desktop.
- [x] Store source images, not generated transformed/cache paths. Reuse `Adjust`, `Frame`,
  `Flip` and `SpanLook` concepts, but separate portable zoom/pan intent from output-sized
  derived data. Preserve standalone cuts left by **Separate** as source-image regions.
- [x] Tests: separate and mixed spanned/independent snapshots, effective-fit capture,
  round trips, validation, stable IDs across rename, explicit update, deletion, empty state,
  malformed/versioned files, concurrent writes and preservation of unrelated configuration.
  Ten preset checks pass; full suite: 171 automated checks, six desktop checks ignored.

### 2. Application and CLI

- [x] Build an application plan from the preset and the shared Wayland monitor discovery.
  Preflight image decoding, selected output matches and span completeness before changing
  state or showing anything. Return applied, skipped and failed details.
- [x] Rebuild frames for current output dimensions and span cuts from the saved canvas and
  current monitor dimensions. Reuse existing image fitting, `transform` and `span` code;
  avoid restoring stale output-sized cuts. Keep missing monitor assignments available in the
  preset without applying them automatically on hotplug.
- [x] Apply prepared per-monitor images through the existing `Backend`/daemon paths. Introduce
  small shared helpers where necessary: current span application saves state before backend
  success, so calling it blindly would violate the preset's success-only bookkeeping.
  Preserve history, coherent span state and cache pruning.
  Backend-default fits are explicit in the saved adjustment, so later global fit changes
  cannot alter them.
- [x] Expose `wallpicker presets list [--json]`, `save NAME`, `apply ID`, `update ID`,
  `rename ID NAME` and `delete ID`. IDs are printed by list; errors distinguish missing images,
  absent outputs and backend failures. Document how automation interacts with preset apply.
- [x] Tests with injected monitor/backend providers: monitor ordering, missing monitors,
  missing files with no side effects, resized/rotated/fractionally scaled displays, incomplete
  spans, partial backend failure, history and state preservation. Presets use the shared
  backend dispatch, whose command/IPC contracts have existing protocol tests. The real CLI's
  CRUD, JSON output and errors are checked in an isolated XDG profile. Full suite: 183
  automated checks passed, six desktop checks ignored.
- [x] Native wallpicker backend on the desktop: two 1920×1080 monitors (DP-1/DP-2),
  separate wallpapers with flip and zoom/pan, mirrored spans, switching both ways and
  two daemon restarts. Eight screenshot checks matched expected pixels. Reproducible
  isolated-profile check: `python3 scripts/test-presets-live.py` (clear both desktops).
- [x] High-zoom dragging regression: accumulate each pointer gesture before rounding
  frame coordinates; symmetric rounding avoids direction bias. Slow horizontal/vertical,
  reverse and mirrored drags pass at 330% zoom in the headless UI
  (`python3 scripts/test-framer-drag.py`), without QML errors or warnings.
- [x] Adjustment transition regression: individual and span zoom/pan updates cut
  immediately while wallpaper selections retain transitions. Two regression tests and
  live native daemon check passed (`scripts/test-presets-live.py --adjustments-only`).
  Full automated suite: 186 passed, six desktop checks ignored.
- External-backend checks retired: wallpicker is now the sole wallpaper renderer.

### 3. Picker integration

- [x] Add `presets.*` service methods using the same core operations as CLI; notify the picker
  after preset CRUD and wallpaper application, including changes from another process.
- [x] Add an accessible **Presets** view with named cards showing each monitor's wallpaper,
  monitor labels and separate/spanned mode. Reuse thumbnail caching; show missing files or
  displays explicitly. Do not invent an active preset indicator that becomes stale after
  adjustments or automation; derive a current match if such an indicator is added.
- [x] **Save current setup** asks for a name. Each card offers Apply, Update from current setup,
  Rename and Delete. Updating/replacing and deleting have clear UI intent; applying needs no
  redundant confirmation when preflight succeeds. Surface skipped outputs and failures.
- [x] Validate QML and service responses in an isolated profile. Full suite: 184 automated
  checks passed; six desktop checks ignored. Save dialog and thumbnail cards checked headless.
- [x] Desktop daemon checks: separate and spanned setups applied repeatedly on both
  monitors and restored after restart, using an isolated profile.
- [x] Refine preset browsing with shared smooth wheel scrolling, responsive cards, larger
  previews, name search and a management menu. Fix concurrent thumbnail cache writes;
  187 automated checks passed, with six desktop checks ignored.
- [ ] Picker checks: create/apply both setups through the UI and reopen the picker;
  verify refreshed assignments and adjustments.

### 4. Portability and completion

- [ ] Add explicit monitor reassignment for renamed/replaced connectors, with a preview and
  validation of one-to-one mapping; update span membership/canvas references consistently.
- [ ] Add preset export/import and backup coverage without silently changing the existing
  library-only backup contract. Preserve original paths, report missing images and reuse
  library path relocation rules where appropriate; export does not bundle wallpaper files.
- [ ] Document presets, missing-display behavior, automation and CLI usage in README. Verify
  two/three displays, reordered connector enumeration, unplug/replug, separate and span mode,
  restored zoom/pan/flips and a backend failure on the desktop before marking complete.

## Its own renderer (planned 2026-10-08, steps 1-4 built 2026-10-09)

The daemon draws wallpapers on the background layer (wlr-layer-shell), with shared-memory
buffers for the steady image and GPU shaders for transitions. Initially introduced alongside
external backends, it became the sole renderer on 2026-10-09; shared application/state code
now lives in `wallpapers/`. Noctalia remains only for wallpaper-derived colors.

Less work than it sounds, here:

- The image math exists: `transform.rs` frames and scales to a monitor's exact pixels, `span.rs`
  cuts each monitor's part. The renderer shows a picture already the output's size; only the fit
  modes are new (Contain's bars, Tile's repeat, composed into the output-sized buffer).
- The daemon exists: one per session (`claim()`), running from login, restoring the state.
- `smithay-client-toolkit` handles outputs, layer shell and shm. Shader effects use
  runtime-loaded Wayland EGL and GLES libraries, with the shared-memory Fade as fallback.

What costs more than it looks:

- **Memory**: a buffer is width × height × 4 bytes per monitor: 33 MB at 4K, 15 MB at QHD, 8 MB
  at 1080p. Once the compositor releases a buffer (it uploads shm to a texture) the daemon
  drops it and redraws from the file on a resize or scale change. Measured: 7.6 MB RSS showing
  two 1080p monitors, steady over applies (glibc kept 85 MB of freed frames in the workers'
  arenas until big allocations were mapped on their own: `mallopt` in `renderer/image.rs`).
- **Scaling**: `wp_fractional_scale` and `wp_viewporter`, to draw at the output's real pixels.
- **Transitions**: not 60 fps blending on the CPU. The new picture on a temporary background
  layer above the old one with `wp_alpha_modifier_v1`: the compositor blends on the GPU, the
  daemon only steps the alpha each frame callback. Hyprland 0.56.2 doesn't apply this modifier
  to layer-shell subsurfaces. Wipes, slides, circles and waves use GLES shaders in the same
  temporary layer: upload the incoming texture once, then change uniforms on frame callbacks.
  No per-frame CPU pixel blending or GPU readback. Missing hardware EGL falls back to Fade;
  a compositor without the alpha modifier gets a plain cut.
- **The daemon becomes required** for this backend: applying talks to it over its socket (today
  only a lock) and starts it when it isn't running, as awww's daemon is started. If it crashes,
  the wallpaper goes with it: the renderer on its own thread, its errors logged, never a panic
  out of the daemon.
- **Checks**: cage has no layer shell. Done in the real session with a throwaway profile:
  `hyprctl layers`, the socket's `status`, and `grim -o` of a headless output compared with the
  expected picture. Careful: adding a headless output (`hyprctl output create headless`) and
  changing its scale (`hyprctl eval 'hl.monitor{…}'`: `keyword` doesn't work with the Lua
  config) left the pointer unable to cross from DP-1 to DP-2 until it was removed. Hyprland's
  color management (`render:cm_enabled`) shows sRGB about 3.5% darker on screen, for any
  wallpaper program: (200, 150, 100) comes out (193, 145, 97).

What it opens up: animated wallpapers whatever the setup (GIF frames on frame callbacks),
and tests that no longer drive the real awww (a renderer on the headless compositor's socket).

Decided against: Quickshell drawing the wallpaper. A QML engine running all the time (tens of
MB at least) goes against the picker exiting to 0 MB after closing.

Each step checked and committed (an earlier attempt that didn't build was thrown away; it's in
`git stash list`, "Before cleanup 2026-10-08"):

1. [x] **Spike**: skipped, the renderer was built directly. The binary grows 0.55 MB (8.09 →
   8.64 MB); smithay-client-toolkit without its keyboard feature, no system libraries.
2. [x] **Composing** (`compose.rs`, unit-tested): an image and a fit → the output's pixels as
   XRGB8888, Lanczos3 with SIMD (fast_image_resize: 8742×4918 to 1080p in 47 ms, image's took
   920); a picture already the output's size as it is. Decoding is the rest: ~250 ms for a 20 MB
   JPEG (zune; jpeg-decoder's scaled decoding was slower). The decoded picture is kept 3 s for
   the next monitor: 6lezjx on both monitors in 0.39 s (2.5 s before).
   - [x] `transform.rs` (framed and spanned copies, every backend) scales with it too, the
     pixel type kept: 6lezjx's framed copy in 0.31 s, decoding and saving included (the
     scaling alone took 0.49 s).
3. [x] **The renderer in the daemon** (`renderer/`): its own thread and calloop loop (no CPU
   idle); a layer surface per output, drawn at the output's own pixels (`wp_viewporter`; the
   integer buffer scale without it), checked at scale 1, 1.5 and 2; outputs coming and going (a
   new one shows its saved wallpaper: `wallpapers::saved_for`); a new size or scale stretches
   what's shown until it's drawn again; a picture that can't be read leaves what's shown. The
   socket (`daemon.rs` in core, a JSON line each way): `show {monitor, path, fit}` answered once
   it's up or with the error, `status`. Running while the backend is wallpicker (then no
   restore: it shows the saved state), stopped when another is picked.
4. [x] **The backend**: `WallpaperBackend::Wallpicker`; applying goes to the daemon, started
   when it isn't running; all five fit modes (`serve`'s options have its fits); conflicts with
   every other program; a note in Settings → General; a README section. The Noctalia plugin goes
   through the command line, so it follows on its own.
   - [x] On the desktop: Settings → General with this backend (the note, the fit choices): the
     QML wasn't run.
5. [x] **Fade**: the alpha modifier; Settings' transitions offer Fade or None for this backend.
   Checked on the desktop, including screenshot pixels midway through the fade, replacement,
   interruption by a cut, and shutdown.
6. [ ] With the HDR monitor: how 8-bit sRGB shm looks there, next to the other backends.

### Shader transitions for wallpicker

The daemon, image loading, fitting, and monitor handling remain shared. EGL/GLES code lives
in `renderer/gpu/`; effect selection and individual shaders in `renderer/transition/`.
Slide directions and circle origins share implementations. Settings offers the native renderer's implemented effects. Shader effects use elapsed time at the monitor's refresh
rate. Screenshot checks verify intermediate regions, direction, texture orientation, final
pixels, replacement, cuts, and shutdown on the live hardware GPU.

- [ ] Simple fade (low priority): awww's per-channel, fixed-step color convergence;
  duration depends on image differences and step size. Normal Fade already covers
  the predictable crossfade use case.
- [x] Wipe.
- [x] Wave.
- [x] Grow as a circle, from the middle, or from a random point.
- [x] Shrink as a circle.
- [x] Slide from the left, right, top, or bottom.
- [x] Submit the exact shader endpoint and retire the temporary layer on the next
  compositor frame. Prevents a frozen, shifted image during Hyprland's layer fade-out;
  live checkerboard handoff checks passed in all four Slide directions.
- [x] A different transition each time, chosen from implemented effects.

### KDE Plasma Wayland compatibility

- [x] Compositor-independent monitor discovery: use `wl_output` and `xdg-output`
  for connector names, descriptions, physical/mode dimensions, logical positions,
  scale and rotation. Keep Hyprland focus/workspace data as optional enrichment.
  Use the same discovery for CLI, picker, applying, spanning and theme selection.
  Native snapshots verified against both connected outputs on Hyprland; unit checks
  cover fractional/rotated scaling, negative positions and metadata enrichment.
  - [ ] Verify fractional scale, rotation and refreshed snapshots after hotplug on
    real KDE displays. KDE testing is required before claiming support.
- [ ] Test wallpaper stacking in a real Plasma session first: ensure background
  layers remain visible with Plasma's desktop, without breaking icons, desktop
  clicks, widgets or panels. Decide whether Plasma-specific integration is needed.
- [ ] Remove the alpha-modifier requirement from shader transitions. Provide shader
  Fade when compositor opacity is unavailable; retain compositor Fade where supported.
- [ ] Verify temporary-layer stacking and final buffer handoff on KWin, including
  replacement, cut, resize, disconnect and shutdown. Preserve the exact-endpoint
  handoff that fixes Slide ghosting on Hyprland.
- [ ] Replace the picker's Hyprland-only focused-monitor selection with a portable
  fallback; make monitor selection behavior explicit when focus cannot be queried.
- [ ] Hide unsupported per-workspace controls on other compositors initially.
  Investigate KDE virtual desktops/activities and their wallpaper semantics before
  implementing a KDE-specific workspace integration.
- [ ] Update compositor-specific UI text, autostart instructions and documentation;
  verify daemon startup, restoration, slideshow and spanning under Plasma Wayland.

## Spanning, refined (done 2026-10-07)

Spanned, the monitors act as one: the drawer shows one **Spanned** card (instead of each
monitor and All) and the top bar one pill; the picture's tile one "Spanned" badge; a click on a
tile spans that picture. The span has its own Adjust, as framing one monitor: zoom, position
(drag, scroll, sliders), mirroring, over the whole layout (what the bezels hide shown dimmed).
**Separate** goes back to a card per monitor, each keeping its part; Alt+1..N separates too.
Anything else put on a monitor ends the span. Each step checked headless and committed:

1. [x] **Core and backend**: the span remembered in the state (the picture, its zoom, center
   and mirroring) and dropped when a monitor shows something else; `span.rs` cuts with the
   zoom and center (a frame over the whole layout) and mirroring; spanning the same picture
   again re-cuts it without touching the history; `wallpicker span <image> [--frame ZOOM,X,Y]
   [--flip h|v|both]`, `wallpicker span --separate`; `serve`'s `wallpapers.span` (with the
   look) and `wallpapers.unspan`.
2. [x] **Drawer, pills, tiles**: the Spanned card (the monitors on a desk, Adjust, Separate,
   Previous), the Spanned pill, the tile's badge; a tile click spans while spanned; Alt+N
   separates.
3. [x] **The span's Adjust dialog**: the layout as it is (positions, sizes, the bezel gap),
   the picture over it, drag, scroll, sliders, mirroring, Reset.

## Spanning: chosen monitors, a physical canvas (done 2026-10-07)

Decided with the user: span across the monitors you choose (M3 above M1 can stay out while M1
and M2 span), arranged on wallpicker's own canvas (spanning only; Hyprland's config never
touched), each monitor at its physical size (Hyprland's `physicalWidth`/`physicalHeight` in
mm), so lines carry on straight in the real world across different sizes and densities. Each
step checked headless and committed:

1. [x] **Core and backend**: monitors carry their physical size (96 dpi when unknown); the
   arrangement in mm (saved in the settings per monitor; the first one from Hyprland's layout,
   neighbours touching, offsets kept), the bezel width (mm, what monitors snap apart at,
   replacing the pixel `span_gap`) and the monitors last spanned; `span.rs` cuts from the
   arrangement for the chosen monitors; the span in the state keeps its monitors and their
   places; `serve`'s `wallpapers.span` takes the monitors and the arrangement,
   `wallpapers.span_layout` gives it; `wallpicker span --monitors A,B`.
2. [x] **Picker**: spanned is the span's monitors showing its picture; the drawer has the
   Spanned card (the monitors in their arrangement, one continuous picture) plus a card per
   monitor outside it, and All; the pills likewise; selecting the span is a target like a
   monitor; Alt+N on a spanned monitor picks the span.
3. [x] **The Span dialog**: "Span across…" in the tile menus (Online downloads first) and the
   Spanned card's Adjust; Arrange (click a monitor in or out, drag with snapping to edges at the
   bezel width and to aligned edges and centres, the bezel width) and Frame (zoom, move,
   mirror); bezels inside each monitor, names on chips; Settings → General's gap goes.

## Metadata: artists, uploader, source (planned 2026-10-08)

Downloads are tagged with their artists today, so the tag row is mostly artist names used once
(47 tags in the library, nearly all artists). Instead each wallpaper gets **metadata** beside
its tags: where it came from, who drew it, who uploaded it. Tags go back to describing the
picture, and the artists get a filter of their own and a way back to more of their work.

In `library.json`, an entry (all optional, an entry with only metadata is kept):

```json
{
  "favorite": false,
  "tags": ["surreal", "architecture"],
  "origin": { "provider": "wallhaven", "id": "mlom5m" },
  "source": "https://www.artstation.com/artwork/abc",
  "uploader": { "name": "jrmnt", "query": "@jrmnt" },
  "artists": [{ "name": "Florent Lebrun", "query": "id:12345" }]
}
```

- **origin** (provider + id) is what "search it on its source" needs: which provider to
  search for the artist, and the "Open on Wallhaven" link. Downloads have it from the file
  name already; storing it keeps it when a file is renamed or moved.
- **source** is the provider's source link (often the artist's page; often empty).
- **artists** is a list: Moebooru posts often have two artist tags, Wallhaven sometimes too.
- A person is a **name and a query**: Wallhaven's exact artist search is the tag's id
  (`id:12345`; the name alone searches loosely), its uploader search `@name`, a Moebooru
  artist's the tag itself. A person added by hand has no query: filtered by, never searched.
- `"version": 2` at the top of the file. An older file is version 1; the first load saves it
  as 2 (and is where later migrations go). The script below refuses to write a version-1 file,
  since the app before step 1 would drop the metadata on its next save.

Each step checked headless and committed:

1. [x] **Core**: `Entry` gets `origin`, `source`, `uploader`, `artists` (`Person {name,
   query}`), `is_empty` counts them; the catalog's `version`, saved as 2 when `serve` or the
   daemon starts; setting and clearing artists (names cleaned like tags, compared ignoring
   case); duplicates move the metadata to the kept copy (where it has none); backup exports
   and imports it (an old backup still imports); `serve`'s `library.catalog` carries it,
   `library.set_artists` edits it; `wallpicker artist <image> add|remove <name>`.
2. [x] **Downloads write metadata**: `download_and_tag` fills origin, source, uploader and
   artists from the details instead of adding artist tags (the tags picked in Details stay
   tags); a downloaded file without metadata gets it on the next download or apply, and its
   old artist tags go. Wallhaven's removed accounts ("deleted") are no uploader.
   `wallpicker details` and the CLI unchanged.
3. [ ] **Migration** (`scripts/wallhaven-metadata.py`, below): run once on the real library
   after step 1 is installed, dry run first.
4. [x] **Filter by artist**: an **Artists** row under the tags (its own chips with
   counts, Expand like the tags), any of the artists picked AND the tags picked; the name
   filter also matches artists and uploaders; tiles show the artists (a brush) before the
   tags. `ui-check.sh` takes `CHECK_LIBRARY=FILE`.
   - [ ] Later: the slideshow (and `wallpicker random --artist`) picking from some artists.
5. [x] **A local wallpaper's details**: right-click → **Details…** (today's Edit tags…
   grows into it): the artists (**More by <artist> on Wallhaven**: the Online tab, its
   provider, the stored query), the uploader (**More from**), **Open source**, **Open on
   Wallhaven** (from the origin: `serve`'s `library.origin_page`), the tags as today; artists
   can be added or removed by hand. `--open details <image>` (`tags` still works).
6. [x] **Noctalia plugin** (by the app-and-plugin decision): the artist in its details, the
   name filter matching it. No artist filter there. `wallpicker list --json` gives each
   image's `artists`. The smoke test (`tests/run.py`) not run: no `luau` here; the mock
   library has no artists, so its snapshot is unchanged by this.

Moebooru (2 Konachan files today): the script handles Wallhaven only; Konachan and yande.re
can follow (`post.json?tags=id:N`, then the tag kinds from `tag.json`) once the Wallhaven
pass is done.

### The migration script

`scripts/wallhaven-metadata.py [--write] [--refresh] [--overwrite]`, Python's standard library
only:

- Finds `wallhaven-<id>.{jpg,png,webp}` under the wallpaper folders and the download folder
  (all of them, not only the files the library has entries for: 122 on disk, 79 entries).
- Asks `/api/v1/w/<id>` for each (the API key from the settings, as a header, so NSFW ones
  work), at most 40 a minute (Wallhaven allows 45), backing off on 429. Answers are cached
  in `~/.cache/wallpicker/metadata/wallhaven/`, so a rerun or an interrupted run doesn't ask
  again (`--refresh` does). Wallpapers Wallhaven removed (404) are reported and left alone.
- Builds the metadata above; removes the tags that are one of that wallpaper's artists (by
  name or Wallhaven alias, ignoring case), keeps every other tag. Fills only what's missing
  (`--overwrite` replaces what's there).
- Dry run by default: each file's artists, uploader, source and the tags it drops, then the
  totals (distinct tags before and after). `--write` saves a copy of `library.json` beside
  it first, re-reads the file right before writing (so a tag edit made meanwhile isn't
  lost) and writes atomically; the picker sees the change (it watches the file).

## Going public (if ever)

- [ ] **AUR package** (`wallpicker-git`): a plain cargo build; installs the binary, the picker
  (`quickshell/` to /usr/share/wallpicker/ui), the desktop entry and icon; depends on quickshell.
- [ ] **A systemd user service** for `wallpicker daemon`, as an alternative to Hyprland's
  autostart.

## Cleanup

- [x] **Retire external wallpaper backends** (2026-10-09): remove hyprpaper, awww/swww and
  Noctalia wallpaper adapters, backend selection and external config generation. Shared native
  application, preparation, span, restore and conflict detection live in `wallpapers/`.
  Legacy backend/FPS fields are ignored and disappear on the next settings save; supported
  transitions, original paths, state, history and presets remain intact. Simple fade becomes Fade.
  Keep Noctalia's color integration and competing-daemon detection; remove the Noctalia picker
  plugin. Regression suite: 170 passed, six desktop checks ignored; migration, restore, preset
  application and Noctalia palette/scheme restoration covered. Headless Settings checked without
  QML warnings. Live native daemon: eight prepared-file checks across both monitors, separate/
  span application and two restarts passed (`--status-only`); strict compositor screenshot
  comparison was obscured by windows/shadows and remains unverified for this cleanup.
  Release build and live single-monitor/span adjustment checks passed: adjustments cut
  immediately, while new wallpaper selections retain their configured transitions.
- [x] Remove `prototype/` (`gpui-local`, `quickshell-local`), including their old lockfiles,
  assets and dependencies.

## Decisions to keep

- **One wallpaper renderer** (2026-10-09): retire hyprpaper, awww/swww and Noctalia wallpaper
  support removed through the cleanup above. Noctalia remains for wallpaper-derived colors;
  this supersedes the earlier app/plugin division.
- **No tray** (2026-10-07): the picker opens in ~0.3 s from a keybind or launcher.
- **The daemon keeps time** (2026-10-07): `wallpicker daemon` runs the slideshow's ticks
  (serialized against manual ticks), the workspace listener, the restore at
  login and Noctalia's palette.
- **Closing** (2026-10-07): Settings → Performance → Keep the picker ready after closing (Off to
  30 min, 1 min by default): hidden and instant to reopen meanwhile, then it exits (0 MB).
- **Picker size** (2026-10-07): Large, Comfortable or Compact as a share of the screen, keeping
  the old window's 1100×760 shape, so 720p screens work too.
