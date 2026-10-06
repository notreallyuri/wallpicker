# TODO

## More wallpaper sources

The Online tab has a provider abstraction (`crates/core/src/online.rs`, `ui/src/views/online/`).
Another Moebooru site is one `Site` in `moebooru.rs` plus a `Provider`. Anything else means: a
core module (query → params, response → `OnlineWallpaper`/`Page`), a branch in
`online_search`/`online_details`, its image host in `Provider::download_prefixes`, its image host
in the CSP (`src-tauri/tauri.conf.json`), a filters file in `ui/src/views/online/`, and a section
in Settings.

- [x] **Konachan** (Moebooru): konachan.net (SFW) or konachan.com (all ratings), a setting.
- [x] **yande.re** (Moebooru): same module as Konachan.
  Artists come from the API's tag kinds (`api_version=2&include_tags=1`).
- [ ] **Unsplash**: photography. Free key, 50 requests/hour in demo mode, 5,000/hour once the
  app is approved. Orientation filter, no exact resolution filter. Attribution required by its
  guidelines.
- [ ] **Wallpaper Abyss** (Alpha Coders): blocked (checked 2026-10). The old APIs
  (`wall.alphacoders.com/api1.0`, `api2.0`) return 404; `api.alphacoders.com/3.0` exists but
  answers `invalid_auth`, and the only offer on https://api.alphacoders.com/api/instructions is
  paid enterprise "custom integrations" by email. No public docs or free key, so nothing to
  build against. Scraping the site isn't an option (fragile, and it would sidestep a paid
  service).
- [ ] **Pexels** / **Pixabay**: stock photos (Pixabay also has illustrations). Free key,
  rate-limited; check the current limits.
- [ ] Daily-image feeds, not searchable: Bing's daily wallpaper, NASA's Astronomy Picture of the
  Day.

## The app and the plugin: who does what

Decided 2026-10-05: they stop chasing feature parity. The plugin is the quick picker (always
there on SUPER+W, nearly free, native to the shell); the app is the library manager (real
dialogs, wrapping layouts, easier to build and test).

- [x] **Plugin = quick picker.** Keep it to applying, random / previous, favorites, search and
  online browsing (with filters), plus favorite, fit and trash. Settings, Duplicates,
  Collections and tag editing moved to the app (v0.16.0).
- [x] **App first for new features.** Blacklist, export / import, spanning across monitors and
  the like land in the app; the plugin gets them only if they fit a quick picker.
- [x] **"Open in wallpicker" in the plugin**: `wallpicker --open
  local|online|collections|settings|duplicates` and `--open tags <image>` show the window there
  (starting the app if needed); the panel's cog, copy icon, Collections, "Edit in wallpicker" and
  "Open wallpicker" use them.
- [x] **Autostart optional.** The app needs to run only while managing the library (it uses
  ~214 MB, plus WebKit helpers). The README now presents `wallpicker --tray` at login as an
  option; the slideshow keeps running from the plugin's service either way.

## Going public (if ever)

What stands between "my tool" and one others can install (2026-10-05):

- [x] **awww (swww) backend**: `awww img` with fit and transitions, the daemon started when
  needed, a warning when another program draws wallpapers too.
- [ ] **AUR package** (`wallpicker-git`): builds with trunk and the Tauri CLI, installs the binary,
  the desktop entry, the icon and the Noctalia plugin.
- [ ] **One-command hyprpaper setup**: add the `source` line for wallpicker's config to
  hyprpaper.conf (asking first), instead of documenting it.
- [ ] **matugen hook** (optional): run matugen on each applied wallpaper, for people theming
  their whole setup with it.

## Noctalia

- [x] Wallpaper backend setting: hyprpaper or Noctalia (`noctalia msg wallpaper-set`).
- [x] Command line for everything the app does (`wallpicker --help`).
- [x] Noctalia plugin (`noctalia-plugin/`): bar icon + a quick-picker panel (local, online,
  filters, slideshow clock); the rest opens in the app.
- [x] Tray icon for the app; closing keeps it running, `--toggle` / `--tray`.
- [x] Colors from the wallpaper: the app (Noctalia's generator, `noctalia theme`) and Noctalia
  itself (`color-scheme-set wallpaper`, a switch in Settings → Appearance).
  With the hyprpaper backend Noctalia isn't drawing the wallpaper, so its own colors can't follow;
  the app's still do (it reads the image).
- [ ] Use the current wallpaper for hyprlock and Noctalia's greeter (`noctalia msg greeter-sync`).

## Quickshell frontend (the port, decided 2026-10-06)

The UI moves from Tauri (WebKit + Leptos) to Quickshell (QML); the Rust core and CLI stay and do
the work. Why: the prototype (`prototype/quickshell-local`, the Local tab 1:1) measured ~215-260 MB
open against ~300-360 MB, and a full first screen ~0.43 s after starting from nothing against
~0.5-0.6 s for today's reopen from the tray, with nothing left running between opens. It looks
and feels like the app (real transitions and easing, glass, shadows); the scrolling needs work.

### How it fits together

- **`quickshell/`** (repo root): the UI, run with `qs -p`. Split by focus, no god-files: one
  component per file, a folder per area; a file that grows past ~300 lines gets split.
  - `shell.qml`: the entry only (ShellRoot, the window's life, IPC).
  - `services/`: singletons holding state and talking to Rust: `Backend` (the connection),
    `Theme`, `Library`, `Monitors`, `Online`, `Settings`, `Toasts`.
  - `components/`: the app's building blocks, mirroring `ui/src/components`: Btn, Icon,
    TextField, Chip, Badge, Pill, Switch, Select, Menu, Popover, Tooltip, Dialog, Pager, Spinner,
    Toast, Skeleton, ScrollArea (the smooth scrolling lives here, once).
  - `bar/`: the top bar (tabs, monitor pills, the drawer, Previous, Settings).
  - `views/local/`, `views/online/`, `views/collections/`, `views/settings/` (a file per section).
  - `dialogs/`: Adjust (mirror, fit, framing), Tags, Duplicates, Trash, Details.
- **`wallpicker serve`**: one long-running Rust process per open UI, JSON lines over stdin and
  stdout (`{id, method, params}` → `{id, result | error}`, plus pushed `{event, data}`: state
  changed, workspace switched, thumbnail made, download progress). Not a process per action
  (~50-120 ms each). Its methods are today's 50 Tauri commands (`src-tauri/src/commands/`): their
  logic moves to a layer without Tauri types, which both the Tauri app (until it's gone) and
  `serve` call. The CLI's one-shot commands stay as they are (the Noctalia plugin uses them).
- **Process model**: `wallpicker` (no arguments) starts `qs -p <the ui>` if it isn't running, else
  toggles it over IPC (`--toggle`, `--open <place>` likewise); closing exits Quickshell (0 MB
  between opens). What must keep running moves to a small `wallpicker daemon` without GTK or
  WebKit: the workspace listener, the tray (ksni: a D-Bus tray item, no GTK) if we keep one.
- **Installed**: `install-local.sh` puts `quickshell/` in `~/.local/share/wallpicker/ui`;
  `wallpicker` finds it there (or next to the binary, for development).

### Phases (each ends with something usable; the Tauri app stays until phase 6)

- [ ] **0. Foundation**: the `quickshell/` layout; `serve` with the library, monitors, theme and
  apply methods (the shared command layer started); `Backend` service (requests, events);
  `Theme` (palette like palette.rs, the color themes, dark/light, Nunito weights); the base
  components; smooth wheel scrolling (ScrollArea: a notch glides, touchpads 1:1, kinetic
  flicks); `wallpicker` starting / toggling the UI; headless checks (cage + grim, the QML log;
  qmltestrunner for components).
- [ ] **1. Local tab**: the prototype, rebuilt on the foundation, then everything it lacks:
  Folders layout and breadcrumbs, Color filter, the ⋯ menu, the tile's right-click menu (apply
  to, favorite, tags, show in folder, trash, duplicates), the Tags dialog, keyboard shortcuts,
  empty and loading states, Random.
- [ ] **2. Top bar and monitors**: the monitor drawer, All, Previous, the Adjust dialog (mirror,
  fit, framing by drag, scroll and sliders).
- [ ] **3. Online and Collections**: providers, search, the filters dialog, details (preview,
  tags, download, apply), downloads, Wallhaven collections, pages and Jump to.
- [ ] **4. Settings**: every section (General, Appearance, Downloads, Slideshow, Workspaces with
  its picker, Performance, Backup with file dialogs, providers), conflicts and Stop it.
- [ ] **5. Background**: `wallpicker daemon` (workspace listener, tray or not), following the
  wallpaper's colors live, Noctalia colors, toasts for what happens in the background.
- [ ] **6. Switch over**: `wallpicker` opens the Quickshell UI; the Noctalia plugin's handoffs
  (`--open`) go through it; remove the Tauri app, WebKit and Leptos (`ui/`, the GUI half of
  `src-tauri`); README, install script, scripts/memory and scripts/open-timing redone for QML.

### To decide on the way

- Tray: keep it (ksni in the daemon) or drop it (Quickshell starts fast; a keybind and the
  Noctalia plugin open it).
- File dialogs (backup export/import, snapshot folder): the XDG desktop portal's FileChooser,
  from Rust (`rfd` over the portal) or from QML.
- Window kind: a normal window (FloatingWindow, as now) or a layer-shell overlay like a launcher.
- Closing: exit (0 MB, ~0.45 s to open) or stay hidden (~200 MB, instant); maybe a setting, like
  the WebKit app's planned "Free the memory after".

## Next up (2026-10-06)

- [x] **Backup**: export / import the library (favorites, tags, fits) as one file, finding moved
  wallpapers; daily automatic snapshots (14 kept) with restore.
- [x] **Filter by color**: a profile per image (share of eight hues, lightness, how colorful),
  from the thumbnail, cached; Local's Color button; `wallpicker colors`. (The duplicates grid was
  too coarse: a 4×4 average loses small colored subjects.)
- [x] **Day and night sets**: the slideshow's night set (its own favorites, tags, folder,
  color) in a local time window that may cross midnight; it switches right away at the edges.
  Each set can filter by color (light by day, dark by night).
- [ ] **A faster cold open**: closing destroys the window (that's what gives the memory back), so
  every open rebuilds the page: WebKit's page process, the 2.5 MB WebAssembly UI, monitors,
  settings, the library, thumbnails. Measure each step (timestamps from the open request to a
  filled grid, headless, a few cycles), then cut the slow ones (e.g. draw the frame before the
  data, fewer requests at once, a smaller bundle). First of the two.
- [ ] ~~**Keep the window warm after closing**~~ (superseded by the Quickshell port, which
  decides this its own way; see "Closing" above): closing only hides it, so reopening is instant (page
  and scroll kept); it's destroyed after it has stayed closed for a while, and the memory goes
  back. The while is a setting (Settings → Performance, "Free the memory after"): 5 min by
  default; 30 min, always ready (never freed), or at once (today's behavior). After the cold open
  is fast.

## Planned (each needs a plan before starting)

- [x] **Frame a wallpaper per monitor (zoom and pan)**: in the Adjust dialog (drag, scroll, sliders;
  zoom up to 4×), cut to the monitor's exact resolution by `transform.rs`, per monitor and
  temporary like mirroring and fit. `wallpicker adjust <monitor> --frame ZOOM,X,Y`.
- [ ] **Span one wallpaper across monitors**: cut it to each monitor's place and size in the
  layout (`hyprctl monitors` has x, y, scale), mixed resolutions too. Shares the crop-and-cache
  step with framing above, so it's cheaper after that. Bezels: optional gap per monitor.
- [ ] **Animated wallpapers**. awww plays GIFs itself; hyprpaper and Noctalia don't.
  - Library: accept .gif (and maybe .webp animations); thumbnails from the first frame, with a
    "▶" badge; preview animates in the details view.
  - Applying an animated file through a backend that can't animate: warn, or apply the first
    frame.
  - Video (mp4/webm) means mpvpaper as a fourth backend: a bigger step, decide separately
    (CPU/GPU cost, pause on fullscreen).
- [x] **Wallpaper per workspace**: Settings → Workspaces; follows Hyprland's event socket (in the
  app, or `wallpicker workspaces run`), remembers what you apply, outside the history.
- [ ] Lock and login screens: the current wallpaper for hyprlock, and Noctalia's greeter
  (`noctalia msg greeter-sync`). (Also listed under Noctalia.)

## Monitors

- [ ] When the QHD/HDR monitor arrives (mid-October 2026): check three monitors end to end, and
  how HDR affects wallpapers under hyprpaper. Then the "All" pill's picture: one little screen
  per monitor, in their proportions.

## Library

- [x] History per monitor: `wallpicker previous`, "Previous" in the app, tray and panel.
- [ ] Wallhaven: skip what your account blacklists (tags and uploaders, from `/api/v1/settings`).

## Project

- [x] Version control (`git init`).
- [ ] Packaging: an AUR PKGBUILD or install script; a systemd user service for
  `wallpicker slideshow run` when Noctalia isn't running.
