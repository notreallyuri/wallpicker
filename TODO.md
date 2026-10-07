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

- [x] **0. Foundation** (done 2026-10-06; the color themes and light mode move to phase 4 with
  their settings, qmltestrunner to when components need unit tests): the `quickshell/` layout; `serve` with the library, monitors, theme and
  apply methods (the shared command layer started); `Backend` service (requests, events);
  `Theme` (palette like palette.rs, the color themes, dark/light, Nunito weights); the base
  components; smooth wheel scrolling (ScrollArea: a notch glides, touchpads 1:1, kinetic
  flicks); `wallpicker` starting / toggling the UI; headless checks (cage + grim, the QML log;
  qmltestrunner for components).
- [x] **1. Local tab** (done 2026-10-07): the prototype, rebuilt on the foundation, then everything it lacks:
  Folders layout and breadcrumbs, Color filter, the ⋯ menu, the tile's right-click menu (apply
  to, favorite, tags, show in folder, trash, duplicates), the Tags dialog, keyboard shortcuts,
  empty and loading states, Random.
- [x] **2. Top bar and monitors** (done 2026-10-07; tooltips left over, below): the monitor
  drawer, All, Previous, the Adjust dialog (mirror, fit, framing by drag, scroll and sliders).
- [x] **3. Online and Collections** (done 2026-10-07): providers, search, the filters dialog, details (preview,
  tags, download, apply), downloads, Wallhaven collections, pages and Jump to.
  The plan (2026-10-07), in this order, each step checked headless and committed:
  1. **Backend**: `src-tauri/src/commands/online.rs`'s ten commands into `service/online.rs`
     (as phase 2 did for adjust), then `serve/methods/online.rs`: `online.search`,
     `details`, `preview`, `downloaded`, `download` (announcing `library-changed`),
     `filters` / `save_filters`, `tag_suggestions`, `collections`, `collection`. Downloads
     report progress as pushed events (`download-progress {id, done, total}`), not a call
     per poll.
  2. **Services**: `Online` (provider, query, filters, page, results, loading, errors;
     remembered choices in Prefs) and `Downloads` (in flight, progress, done).
  3. **Views** (`views/online/`, mirroring `ui/src/views/online/`): OnlineView, its search
     card (reuse Glass, SearchCard's layout), OnlineGrid + OnlineTile (sizes, downloaded
     mark, progress), the provider's filters (Wallhaven, Moebooru: a file each), the pager
     with Jump to (components/Pager, JumpTo exist).
  4. **Dialogs**: Details (preview, tags with suggestions, download, download and apply).
  5. **Collections** tab (`views/collections/`): a Wallhaven user's collections and their
     pages; the top bar's tabs switch views (Picker hosts one at a time).
  6. Online's tooltips (the app's: "Sorting and filters", "Already in your download
     folder", the details dialog's two).
- [x] **Split from Tauri** (done 2026-10-07): `wallpicker` (`crates/wallpicker`: the command line,
  `serve`, the shared service layer; tokio, no Tauri/WebKit/GTK, 5.9 MB) and `wallpicker-app`
  (`src-tauri`: the old window only, frozen: no more fixes or features). `wallpicker` with no
  arguments, `--tray`, `--toggle` or `--open` hands over to `wallpicker-app` until phase 6.
- [x] **4. Settings** (done 2026-10-07; the file dialogs need a check on the real desktop): every section (General, Appearance, Downloads, Slideshow, Workspaces with
  its picker, Performance, Backup with file dialogs, providers), conflicts and Stop it.
  The plan (2026-10-07), in this order, each step checked headless and committed:
  1. **Backend**: Tauri's save_settings logic (keeping the snapshot folder and workspace
     wallpapers, re-applying when the backend or a fit changed) into `service/settings.rs`;
     `serve` methods `settings.options` (backends, transitions, fits per backend, Konachan sites,
     slideshow intervals, colors: from core), `settings.save` (announcing `settings-changed`),
     `settings.conflicts`, `settings.stop_program`, `slideshow.status` / `now`,
     `noctalia.colors` / `set_colors`, `workspaces.rows` / `set`, `backup.*` (export, import,
     snapshots, snapshot_dir, restore), `app.usage` (the interface's and serve's memory).
  2. **File dialogs**: the XDG desktop portal's FileChooser from Rust (`dialogs.save_file`,
     `open_file`, `folder`): the system's own dialog, no GTK in `wallpicker`.
  3. **The Settings dialog**: the cog opens it; a sidebar of sections (App, Providers), the
     pane, Save (most fields) at the bottom; Settings, Library, Downloads, Collections and Theme
     reload on `settings-changed`. Sections in their own files (`views/settings/`).
  4. **General, Downloads, providers**: folders, subfolders, backend (conflicts and Stop it,
     awww's transition, Noctalia's note), fit and per-monitor fits, thumbnail width; the
     download folder; Wallhaven key and username, Konachan's site, yande.re's note.
  5. **Appearance**: dark mode (system, light, dark) and the color themes (Blossom, Matcha,
     Peach, Lagoon, Lavender, Honey) in Theme, remembered in Prefs; colors from the wallpaper
     and from which monitor; Noctalia's colors following it.
  6. **Slideshow**: on, interval, the day set (favorites, color, tags all/any, folder), monitors
     and one each, the night set and its hours, status and Change now.
  7. **Workspaces**: the switches, each workspace with its wallpaper, Choose (a picker over the
     library in place), Clear.
  8. **Performance and Backup**: memory and CPU of the interface and serve; glass, backdrop,
     animations (Prefs), full-size previews; export / import through the portal, the snapshots,
     their folder, restore (asking first).
  Not ported: "Keep running in the tray" (the tray is phase 5's call).
- [x] **5. Background** (done 2026-10-07): `wallpicker daemon` (workspace listener, tray or not), following the
  wallpaper's colors live, Noctalia colors, toasts for what happens in the background.
  The plan (2026-10-07), in this order, each step checked and committed:
  1. **Live from anywhere**: `serve` watches state.json, config.json and library.json and
     announces changes made by other programs (the slideshow, the plugin, the command line,
     the workspace listener): the picker's monitors, drawer, colors and library follow while
     it's open.
  2. **Toasts for the background**: a monitor's wallpaper changed while the picker is open and
     it didn't do it: say so (which monitor, which file).
  3. **`wallpicker daemon`**: what must keep running without the window: the workspace
     listener (it already guards against a second copy) and the slideshow's timer (decided
     2026-10-07: the daemon does it, guarded so only one timer acts with the plugin's ticks).
     One autostart line, replacing `wallpicker --tray` at the switch (phase 6).
  4. **Noctalia's colors with awww or hyprpaper**: Noctalia only follows the wallpaper it
     draws itself. A custom palette (`noctalia msg color-scheme-set custom wallpicker`,
     `~/.config/noctalia/palettes/wallpicker.json`, Noctalia's palette format) made from the
     wallpaper shown, made again and switched to by the daemon (and an open picker) as it
     changes: Noctalia doesn't watch the file.
  No tray (decided 2026-10-07): the picker opens in ~0.5 s from a keybind or the plugin.
- [ ] **6. Switch over**: `wallpicker` opens the Quickshell UI; the Noctalia plugin's handoffs
  (`--open`) go through it; remove the Tauri app, WebKit and Leptos (`ui/`, `src-tauri/`: the
  window's arguments go to Quickshell instead of `wallpicker-app`); README, install script, scripts/memory and scripts/open-timing redone for QML.

### Left over from the phases

- [ ] **Settings changes** (with phase 4): reload Downloads (the download folder may move) and
  Collections (the key or username may change) after Settings saves, as the app does.

- [x] **Tooltips** (done 2026-10-07): `Tip` (components/Tip.qml; Btn's `tip`), the app's tips
  on the monitor pills, All, the drawer's chevron and cards, Previous, Settings, ⋯, Random,
  Flat/Folders, Color, Up a folder, All/Any, and a tile's size when it's smaller than a chosen
  monitor (amber). Online's come with phase 3.

### To decide on the way

- ~~Tray~~: dropped (2026-10-07).
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
