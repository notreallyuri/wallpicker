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

## Next up (2026-10-06)

- [x] **Backup**: export / import the library (favorites, tags, fits) as one file, finding moved
  wallpapers; daily automatic snapshots (14 kept) with restore.
- [x] **Filter by color**: a profile per image (share of eight hues, lightness, how colorful),
  from the thumbnail, cached; Local's Color button; `wallpicker colors`. (The duplicates grid was
  too coarse: a 4×4 average loses small colored subjects.)
- [x] **Day and night sets**: the slideshow's night set (its own favorites, tags, folder,
  color) in a local time window that may cross midnight; it switches right away at the edges.
  Each set can filter by color (light by day, dark by night).

## Planned (each needs a plan before starting)

- [ ] **Frame a wallpaper per monitor (zoom and pan)**. Neither hyprpaper nor awww can zoom or
  offset (awww only anchors its crop: `--crop-gravity`), so wallpicker makes the image itself.
  Its home is the Adjust dialog (mirror and fit are there: per monitor, temporary), and the
  changed-copy step exists (`transform.rs`, used for mirroring): add a crop to `Adjust`.
  - In the app: a live preview in the monitor's own proportions; a zoom slider, drag (or
    horizontal / vertical sliders) to move. All in the browser (CSS transform), so it's instant.
  - On release: crop and scale to the monitor's exact size, save in the cache (keyed by image,
    monitor size and frame), and apply that file. Works with every backend, Noctalia too.
  - Stored per wallpaper and monitor (or monitor size) in the library, as a normalized rectangle
    (0..1), so it survives resolution changes; "Reset" goes back to the plain fit.
  - Not live on the desktop while dragging: re-encoding a 4K image and re-applying dozens of
    times a second would stutter. Apply on release (awww's transition makes it smooth).
  - Open questions: per monitor or per monitor size; what Previous and the slideshow do with
    framed wallpapers (use the frame, presumably); cache pruning.
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
- [ ] **A wallpaper per workspace**: listen to Hyprland's event socket (`workspace>>`) and apply
  that workspace's wallpaper; the set is chosen in the app. Best with awww's transitions.
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
