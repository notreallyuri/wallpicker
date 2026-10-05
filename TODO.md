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

## Noctalia

- [x] Wallpaper backend setting: hyprpaper or Noctalia (`noctalia msg wallpaper-set`).
- [x] Command line for everything the app does (`wallpicker --help`).
- [x] Noctalia plugin (`noctalia-plugin/`): bar icon + a panel that's a full client (local,
  online, collections, filters, settings, slideshow clock).
- [x] Tray icon for the app; closing keeps it running, `--toggle` / `--tray`.
- [x] Colors from the wallpaper: the app (Noctalia's generator, `noctalia theme`) and Noctalia
  itself (`color-scheme-set wallpaper`, a switch in Settings → Appearance).
  With the hyprpaper backend Noctalia isn't drawing the wallpaper, so its own colors can't follow;
  the app's still do (it reads the image).
- [ ] Use the current wallpaper for hyprlock and Noctalia's greeter (`noctalia msg greeter-sync`).

## Monitors

- [ ] Span one wallpaper across monitors: cut it to each monitor's place and size in the layout
  (mixed resolutions too).
- [ ] When the QHD/HDR monitor arrives (mid-October 2026): check three monitors end to end, and
  how HDR affects wallpapers under hyprpaper.

## Library

- [x] History per monitor: `wallpicker previous`, "Previous" in the app, tray and panel.
- [ ] Export / import favorites, tags and fits (one file), for backups and moving machines.
- [ ] Wallhaven: skip what your account blacklists (tags and uploaders, from `/api/v1/settings`).

## Project

- [x] Version control (`git init`).
- [ ] Packaging: an AUR PKGBUILD or install script; a systemd user service for
  `wallpicker slideshow run` when Noctalia isn't running.
