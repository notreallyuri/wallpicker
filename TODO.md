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
5. [ ] **A local wallpaper's details**: right-click → **Details…** (today's Edit tags…
   grows into it): the artists (**More by <artist> on Wallhaven**: the Online tab, its
   provider, the stored query), the uploader (**More from**), **Open source**, **Open on
   Wallhaven** (from the origin), the tags as today; artists can be added or removed by hand.
6. [ ] **Noctalia plugin** (by the app-and-plugin decision): the artist in its details, the
   name filter matching it. No artist filter there.

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
