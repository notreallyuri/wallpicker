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

## Spanning: chosen monitors, a physical canvas (2026-10-07)

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
2. [ ] **Picker**: spanned is the span's monitors showing its picture; the drawer has the
   Spanned card (the monitors in their arrangement, one continuous picture) plus a card per
   monitor outside it, and All; the pills likewise; selecting the span is a target like a
   monitor; Alt+N on a spanned monitor picks the span.
3. [ ] **The Span dialog**: "Span across…" in the tile menus (Online downloads first) and the
   Spanned card's Adjust; Arrange (click a monitor in or out, drag with snapping to edges at the
   bezel width and to aligned edges and centres, the bezel width) and Frame (zoom, move,
   mirror); bezels inside each monitor, names on chips; Settings → General's gap goes.

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
