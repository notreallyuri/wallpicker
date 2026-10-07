# TODO

What's done is in the README and git history (this file's earlier versions hold the finished
plans: the Quickshell port's phases, spanning, framing, backup, color filter).

## Checks on the desktop

Things the headless checks can't reach (cage has no portal and no layer-shell panel):

- [ ] Settings → Backup: export and import through the system's file dialog, round trip.
- [ ] Settings → Appearance: the "Noctalia's colors follow the wallpaper" switch, by click.
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

- [ ] **Animated wallpapers** (needs a plan). awww plays GIFs itself; hyprpaper and Noctalia
  don't.
  - Library: accept .gif (and maybe animated .webp); thumbnails from the first frame with a "▶"
    badge; the details preview animates.
  - A backend that can't animate: warn, or apply the first frame.
  - Video (mp4/webm) means mpvpaper as a fourth backend: decide separately (CPU/GPU cost,
    pausing under fullscreen).
- [ ] **Lock and login screens**: the current wallpaper for hyprlock, and Noctalia's greeter
  (`noctalia msg greeter-sync`).
- [ ] **Wallhaven blacklist**: skip the tags and uploaders your account blacklists (from
  `/api/v1/settings`).
- [ ] **matugen hook** (optional): run matugen on each applied wallpaper, for setups themed with
  it.

## Going public (if ever)

- [ ] **AUR package** (`wallpicker-git`): a plain cargo build; installs the binary, the picker
  (`quickshell/` to /usr/share/wallpicker/ui), the desktop entry, the icon and the Noctalia
  plugin; depends on quickshell.
- [ ] **A systemd user service** for `wallpicker daemon`, as an alternative to Hyprland's
  autostart.
- [ ] **One-command hyprpaper setup**: add the `source` line for wallpicker's config to
  hyprpaper.conf (asking first), instead of documenting it.

## Cleanup

- [ ] `prototype/` (`gpui-local`, `quickshell-local`): the port's prototypes, no longer used.
  Delete once you're sure nothing there is wanted.

## Decisions to keep

- **App and plugin** (2026-10-05): the Noctalia plugin is the quick picker (applying, random /
  previous, favorites, search, online browsing with filters, favorite, fit, trash); the picker
  is the library manager. New features land in the picker first; the plugin gets them only if
  they fit a quick picker, and hands bigger jobs over with `wallpicker --open <place>`.
- **No tray** (2026-10-07): the picker opens in ~0.3 s from a keybind or the plugin.
- **The daemon keeps time** (2026-10-07): `wallpicker daemon` runs the slideshow's ticks
  (guarded so the plugin's ticks don't double them), the workspace listener, the restore at
  login and Noctalia's palette.
- **Closing** (2026-10-07): Settings → Performance → Keep the picker ready after closing (Off to
  30 min, 1 min by default): hidden and instant to reopen meanwhile, then it exits (0 MB).
- **Picker size** (2026-10-07): Large, Comfortable or Compact as a share of the screen, keeping
  the old window's 1100×760 shape, so 720p screens work too.
