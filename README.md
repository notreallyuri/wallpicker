<p align="center">
  <img src="assets/logo.png" alt="wallpicker's logo: a sleepy cat curled along the bottom of a picture frame" width="180">
</p>

# wallpicker

A visual wallpaper picker for Hyprland with its own native Wayland renderer.
Noctalia is optional, for wallpaper-derived colors.

- **Local**: a thumbnail grid of the images in your wallpaper folders, 60 a page (only one page
  is on screen, so memory stays the same however far you go). Click one to apply it.
  Right-click any wallpaper (Local or Online) to apply it to a specific monitor. Right-click →
  **Move to trash…** (or the **Delete** key on a tile) moves a local wallpaper to your trash, after
  a confirmation; restore it from your file manager. Only images inside your wallpaper or download
  folders can be trashed. Right-click also has **Show in folder** (the file manager, with it
  selected) and **Copy path**.
  - **Favorites** shows only favorites; the sort (Newest, Oldest, Name, Random order) keeps
    favorites first.
  - **Flat / Folders**: everything at once, or browse folder by folder (Up, the path, folder
    cards). A filter in Folders searches the current folder and below.
  - **Random** applies a random one of the wallpapers shown, avoiding what's already on screen.
  - Each tile shows the image's size, in amber when it's smaller than a monitor you're applying
    to (it would be scaled up there). Only a hint: nothing is hidden.
  - Sort and layout are remembered.
- **Favorites, tags and artists**: star a local wallpaper to pin it to the top of every view.
  Give it tags with right-click → **Details…**, then filter by one or more tags with
  **Filters** next to the search box (it says how many are picked): a popover with the tags
  and the artists as chips (picked ones first, then by name or the most used first as set in
  Settings → General; **All N** shows them all, the field finds one), matching all the tags
  picked or, with **Match all tags** off, any of them. Settings → Appearance can show them as
  rows under the search bar instead, opened by **Filters**. Downloads keep where they came
  from, their artists, uploader and source link (as the provider lists them) apart from the
  tags, and arrive tagged only with the tags you keep in **Details**. The artists filter
  alongside the tags (any of those picked) and show on the tiles (a brush). Right-click → **Details…** shows where a
  wallpaper came from: its artists (**More by <artist>** searches the provider it came from
  for exactly them), uploader (**More from**), **Open source**, **Open on Wallhaven**; and
  edits its artists (add one by hand, or remove one) and tags. The name filter also matches tags, artists
  and uploaders. `scripts/wallhaven-metadata.py` gives Wallhaven wallpapers downloaded before
  this their metadata, and takes their artist names out of the tags.
- **Online**: search a provider, picked with the select next to the search box:
  - **Wallhaven** (wallhaven.cc): sorting, top range, minimum size, exact resolutions and
    aspect ratios (any number of each; your monitors' sizes are listed first, never picked for
    you), categories, purity. Any size can be typed in, small ones too. Exact resolutions replace
    the minimum (Wallhaven takes one or the other).
  - **Konachan** (anime): sorting, minimum size (a preset or any size typed in), ratings. konachan.net (safe-for-work only, the
    default) or konachan.com (all ratings), set in Settings → Konachan.
  - **yande.re** (anime, high-resolution scans): sorting, minimum size, ratings.

  Sorting and filters are in the dialog behind the **Filters** button (it shows how many differ
  from the defaults). The picture's border shows the rating: green SFW, yellow sketchy, red NSFW. Then **Apply** (download + set) or just **Download**. Results already in the download folder
  are marked **Downloaded** and only offer Apply. **Details** (or a click on the picture) shows
  the uploader, source link and tags; click a tag or "More from <uploader>" to search for it.
  The **+** on a tag keeps it: Download or Apply from Details adds the kept ones to your library
  as tags (the artists go in its metadata), even when it's downloaded already. Tags your
  library already uses start kept.
  Artists (Wallhaven's "Artists" category, Konachan's and yande.re's artist tags) are shown
  first, with a **More by <artist>** button.
- **Collections**: your Wallhaven collections (private ones too), shown like search results:
  apply, download, details. Needs your API key and your Wallhaven username (Settings →
  Wallhaven): opening a collection needs the name, and the API doesn't say whose key it is.
  Wallhaven's API is read-only, so collections can be browsed but not changed from here.
- **Per monitor**: the bar at the top picks which monitors a click applies to, and shows what each
  one currently has (with a thumbnail).
- **Fit**: a general fit in Settings, and a fit per monitor under it (say, Contain on a 4:3
  screen).
- **The control panel** (the tab on the picker's right edge, or right-click a monitor's pill):
  beside the grid, everything about the wallpaper where a click applies, following the pills.
  On a monitor: **frame** what it shows (drag the preview to move, scroll or the sliders to zoom
  up to 4×; it's cut to the monitor's exact resolution), **mirror** it (left to right, top to
  bottom), or give it its own **fit**; About says what the picture is; the footer goes back
  (**Previous**), stars it, opens its **Details** or resets it. The preview follows at once; the
  monitor a moment after you stop. Temporary: another wallpaper on that monitor goes on as it
  is. On **All**: Mirror and Fit for every monitor. Applying from the grid keeps it open, so you
  can try wallpapers and adjust them in one go. Drag its left edge to resize it (320–560 px,
  remembered); the grid makes room, except at Compact sizes where it floats over it (Esc
  closes it). wallpicker makes the changed copy (cached, deleted when unused).
  `wallpicker adjust <monitor> --frame 200,70,40 --flip h --fit contain`.
- The search bar (and the tags and folder path under it) stays at the top while you scroll.
- **Keys**: Ctrl+F search field, Ctrl+R random, Alt+Up parent folder, Alt+Left/Right pages,
  Ctrl+1/2/3/4 Local/Online/Collections/Presets, Ctrl+B the control panel, Alt+1…N where a
  click applies (a monitor, in the top bar's order) and Alt+N+1 All, Delete a tile's trash,
  Esc the top menu or dialog, the panel floating over the grid, then the picker.
- **Span across monitors** (a tile's right-click menu → **Span across…**, or `wallpicker span
  <image>`): one picture across the monitors you choose, each showing its part. The Span dialog
  has a canvas of your monitors at their real sizes (from what they report, so a 27" QHD next
  to a 24" 1080p gets the share it really covers): click a monitor in or out, drag it to where
  it stands on your desk (it snaps to the others, the bezel width apart, or lined up with
  them; anywhere along an edge), then **Span**. Your Hyprland layout isn't touched: the
  arrangement is wallpicker's own, kept for next time (**From Hyprland** starts it over). The
  span's monitors then act as one: one **Spanned** pill (them as arranged, one picture);
  picked, a click on a tile spans that picture across them; the other monitors carry on on
  their own. Picked (or right-clicked), the control panel frames the span shown: zoom, move and
  mirror the picture over them all, changes going on the monitors as you make them;
  **Arrange monitors…** reopens the dialog to change which ones and where. Its unlink button
  gives each monitor back to itself, keeping its part;
  **Previous** goes back on the span's monitors. Another wallpaper on one of them ends the
  span. From the command line: `--monitors A,B`, `--frame ZOOM,X,Y`, `--flip h|v|both`,
  `wallpicker span --separate`, `wallpicker settings set span_bezel=MM`.
- **Live adjustments**: zoom, panning, mirroring and edits to the current span update
  immediately. Wallpaper selections, Previous and preset application use the configured transition.
- **Wallpaper presets**: the Presets tab saves, applies, updates, renames and deletes named
  setups, with responsive preview cards and reports for missing files/displays. Search by name
  with Ctrl+F; each card's ⋯ menu holds Update, Rename, Reassign monitors and Delete. Ctrl+4 opens it;
  `wallpicker --open presets` opens the picker there. From the command line, save a named setup of
  wallpaper assignments, fit, zoom, pan, flips and span canvas layout with
  `wallpicker presets save "Night"`. `wallpicker presets list` shows its ID;
  `wallpicker presets apply p1` restores it using your current transition.
  `update p1` replaces its snapshot, `rename p1 "Work"` changes its name and `delete p1`
  removes it. **Reassign monitors…** previews a one-to-one mapping for renamed or replaced
  connectors, retaining wallpapers, adjustments and span positions. Saving edits the preset;
  Apply shows it afterward. Connector swaps are supported. From the CLI, preview with
  `wallpicker presets reassign p1 DP-1=DP-3 --preview`, then remove `--preview` to save.
  Unspecified monitors stay unchanged; destinations may be disconnected for future setups.
  Add `--json` for scripts. Disconnected monitors are skipped; an incomplete span
  is skipped as a group. Missing/unreadable images needed by connected assignments prevent
  any wallpaper changes. Backend failures can leave a partial application and are reported;
  only successful assignments are remembered. The compositor's monitor layout is untouched.
  Slideshow and workspace automation continue normally after applying a preset.
- **Duplicates** (Local → **⋯** → **Find duplicates…**): exact copies and the same picture at another size or
  re-saved, a group at a time. The largest copy is picked to keep; the others go to the trash, and
  their favorite and tags move to the kept one (a monitor showing one of them gets it instead).
  Pictures that only look alike (the same artist's figure on white) aren't matched.
- **Slideshow** (Settings → Slideshow): a random wallpaper every few minutes, from favorites,
  some tags, a folder or a color, on all or some monitors, the same one or a different one on
  each. **Something else at night**: a second set for a time window (say 20:00–07:00, Dark), and
  it changes right away when night starts and ends. `wallpicker random --color dark` too.
  `wallpicker daemon` keeps time for it. **Change now** changes it at once.
- **A wallpaper per workspace** (Settings → Workspaces): when a monitor switches to a Hyprland
  workspace that has a wallpaper, it goes on that monitor. With **Remember what I put on each
  workspace**, applying a wallpaper is all it takes: it becomes the wallpaper of the workspace its
  monitor shows (Random and the slideshow don't change them). Switching doesn't go into the
  history, so Previous still means the one you chose before. Or pick one per workspace right
  there: click its picture (or the empty "+ Choose" slot) to open your library in place,
  searchable by name and tag.
  `wallpicker daemon` follows the workspaces (or `wallpicker workspaces run`). Also
  `wallpicker workspaces list | on | off | set 3 image.png | clear 3`.
- **The picker** is a floating panel over the focused monitor (Quickshell): it opens in about
  half a second (~220 MB while open). Closed, it stays loaded for a while (Settings →
  Performance → Keep the picker ready after closing: Off to 30 minutes, 1 minute by default),
  so opening it again then is instant and where you left it; then it frees all its memory. Its size is a margin
  round it, in the old window's shape (Settings → Appearance → Picker size: Large, Comfortable,
  Compact). `wallpicker --toggle` opens or closes it (for a keybind). What must keep running
  (workspaces, the slideshow, Noctalia's colors) is `wallpicker daemon`, a small process for the
  autostart. No tray.
- **Color themes** (Settings → Appearance): Blossom, Matcha, Peach, Lagoon, Lavender and Honey,
  each with a light and a dark half; **Dark mode** is on, off or like the system.
- **Colors from the wallpaper** (Settings → Appearance): the picker takes its colors from the
  wallpaper on the monitor you pick (the first one by default), made by Noctalia's palette
  generator with its wallpaper scheme, so it matches Noctalia. **Noctalia's colors follow the
  wallpaper** makes Noctalia's colors follow too (and goes back to the scheme it had): with
  a custom palette (`~/.config/noctalia/palettes/wallpicker.json`, `custom wallpicker`),
  kept current by the daemon. `wallpicker theme`
  shows the palette; `wallpicker noctalia-colors on|off`.
- **Color** (Local, next to the sort): show the wallpapers with a color (red, orange, yellow, green,
  teal, blue, purple, pink) or tone (Dark, Light, Warm: golden, brown, firelight; Muted: mostly
  white, grey or black), the most of it first. Each pixel is named by the nearest of a set of
  reference colors, so skin and wood aren't "red", gold is yellow and light pink is pink. Read
  from the thumbnails once, then cached. `wallpicker colors [COLOR]`.
- **Backup** (Settings → Backup): your favorites, tags, fits and presets are copied before the first change
  of each day (the last 14 kept; restore any of them), in a folder you can choose (a synced one
  makes them off-machine backups). **Export…** and **Import…** use the system's file dialog; an
  import is merged (or replaces the library and presets) and finds wallpapers that moved, by their place
  inside your wallpaper folders or by name and size.
  Older library-only backups leave presets untouched. Missing preset images retain their
  saved paths and are reported; exports contain configuration, not wallpaper files.
  `wallpicker backup export|import|snapshots|restore`, `settings set snapshot_dir=…`.
- **Monitors**: a pill per monitor in the top bar (and **All**), showing its wallpaper in the
  monitor's own shape; click one to make it the target, right-click to adjust what it shows.
- **Previous**: each monitor remembers its last 30 wallpapers; **↶ Previous** (in the monitor
  bar, or `wallpicker previous` for a keybind) goes back through them.
- **Saved choices**: every applied wallpaper is written to `state.json`, and
  `wallpicker --restore` re-applies them at login.
- **Settings**: the cog at the top right.

The picker is [Quickshell](https://quickshell.org) (QML) over a Rust backend; the logic is a
Rust library with unit tests. (Until October 2026 it was a Tauri + Leptos window.)

## Requirements

On Arch:

```bash
sudo pacman -S --needed base-devel quickshell
```

Rust (stable) to build. `wl-clipboard` for Copy path, `xdg-desktop-portal` (with a backend such
as KDE's or GTK's) for Settings → Backup's file dialogs, Noctalia for colors from the wallpaper.

## Build and install

```bash
cargo build --release -p wallpicker   # target/release/wallpicker
scripts/install-local.sh              # ~/.local/bin/wallpicker, the picker, a launcher entry
```

`install-local.sh` puts the picker (`quickshell/`) in `~/.local/share/wallpicker/ui`, and adds
wallpicker to your app launcher (`wallpicker.desktop`, with Random and Previous as quick actions;
the full path to the binary, since launchers often run with a PATH without `~/.local/bin`). It
refuses a build older than its sources, so a new picker never goes in with an old backend.

While working on it, `cargo build && ./target/debug/wallpicker` runs the picker from the
checkout (a binary built inside it finds its `quickshell/`; `WALLPICKER_UI` points elsewhere).

### Starting it

- **The picker**: a keybind on `wallpicker --toggle` (in the Lua config:
  `hl.bind("SUPER + SHIFT + W", hl.dsp.exec_cmd("/home/you/.local/bin/wallpicker --toggle"))`),
  the launcher. `wallpicker --open local|online|collections|presets|settings|
  duplicates` and `--open details <image>` (a wallpaper's details: its tags and artists) open it there.
- **The daemon**, in Hyprland's autostart: `wallpicker daemon`. It puts the saved wallpapers back
  through its renderer, then follows workspaces, keeps the slideshow's time and Noctalia's colors
  following the wallpaper. One per session; a second one
  says so and leaves. `wallpicker --tray`, from the old app's autostarts, runs it too.

### How it fits together

- `quickshell/`: the picker, run as `qs -p <its folder>`, split by focus: `services/` (state and
  the connection to Rust), `components/`, `bar/`, `views/` (local, online, collections, presets, settings:
  a file per section), `dialogs/`, `windows/`.
- `wallpicker serve`: the picker's backend, one per open picker, JSON lines over stdin/stdout
  (`crates/wallpicker/src/serve/`); it also announces changes other programs make (the
  slideshow and command line) so the open picker follows.
- `wallpicker ui` starts Quickshell with Vulkan (where there's a driver) and a leaner allocator:
  ~40 MB less than the defaults.
- Checks: `scripts/ui-check.sh` runs the picker headless with a throwaway profile, clicks and
  scrolls through test hooks, screenshots, and fails on QML errors. `scripts/memory.sh` and
  `scripts/open-timing.sh` measure the real thing on your desktop.

## Command line

The same library, settings and backend as the picker, for scripts and keybinds:

```bash
wallpicker list                       # what's on each monitor, then the library (* = favorite)
wallpicker list --json --thumbs       # the same as JSON, with cached thumbnail paths
wallpicker apply ~/Pictures/x.png     # on every connected monitor
wallpicker apply x.png --monitor DP-1 --monitor DP-2
```

A random wallpaper, e.g. for a keybind:

```bash
wallpicker random                           # one random wallpaper on every monitor
wallpicker random --each                    # a different one on each monitor
wallpicker random --favorites -m DP-1       # a random favorite on DP-1
wallpicker random --tag mitsu --tag dark    # with both tags (add --any for either)
wallpicker random --folder ~/Pictures/wpp/anime
wallpicker previous                         # back to the wallpaper before (-m DP-1 for one monitor)
```

It avoids what's already showing when there's anything else to pick.

The slideshow, as set in Settings → Slideshow (or with `settings set slideshow.*`):

```bash
wallpicker settings set slideshow.enabled=true slideshow.minutes=30 slideshow.favorites=true
wallpicker slideshow status      # {"enabled":true,"minutes":30,"next_in":1740}
wallpicker slideshow tick        # change it if it's due (the daemon checks this regularly)
wallpicker slideshow tick --now  # change it now
wallpicker slideshow run         # keep it going without the daemon (Ctrl+C to stop)
```

Library edits, the same as the picker's:

```bash
wallpicker favorite ~/Pictures/wpp/a.jpg [--off]
wallpicker tag ~/Pictures/wpp/a.jpg add "dark mode"     # or remove
wallpicker artist ~/Pictures/wpp/a.jpg add Mitsu        # or remove
wallpicker trash ~/Pictures/wpp/a.jpg                   # desktop trash; wallpaper/download folders only
wallpicker show-in-folder ~/Pictures/wpp/a.jpg          # file manager, with it selected
wallpicker duplicates                                   # copies and resized versions, best first
wallpicker duplicates keep ~/Pictures/wpp/a.png ~/Pictures/wpp/a-small.jpg   # keep one, trash the rest
wallpicker fit ~/Pictures/wpp/a.jpg contain             # its own fit everywhere (auto: the monitor's)
wallpicker settings set monitor_fit.DP-2=contain        # a monitor's fit ("" for the general one)
```

Filters and settings, the same files as the picker's Filters dialog and Settings (JSON out):

```bash
wallpicker filters                                          # every provider's filters + options
wallpicker filters wallhaven set sort=toplist range=1w atleast=1280x720 nsfw=true
wallpicker filters wallhaven set resolutions=1920x1080,2560x1440 ratios=16x9,4x3   # "" for any
wallpicker filters yandere set questionable=true            # or: filters yandere reset
wallpicker settings
wallpicker settings set thumb_width=480 "folders=~/Pictures/wpp
~/Pictures/more"                                            # one folder per line
wallpicker settings set animation.enabled=true              # play GIF/animated WebP (experimental)
wallpicker settings set animation.paused=true               # hold every animation's frame
wallpicker settings set animation.max_fps=30                # new frames a second at most (0: the display's)
```

Wrong keys or values change nothing. The Wallhaven API key is never printed and can only be set
in the picker (Settings → Wallhaven), so it never sits on a command line.

Online, with each provider's saved filters (the picker's Filters dialog):

```bash
wallpicker search konachan landscape --page 2 --json --thumbs
wallpicker search wallhaven --sort toplist        # also saves the sort for the picker
wallpicker details '<handle>'                     # tags, artist, uploader, source (JSON)
wallpicker tags konachan mitsu                     # the real tag names a Konachan/yande.re search means
wallpicker collections                             # your Wallhaven collections (API key)
wallpicker collection 2117134 --page 2 --json      # a page of one (needs your username in Settings)
wallpicker preview '<handle>'                     # its sharpest image, cached (JSON)
wallpicker apply-online '<handle>' --monitor DP-1  # a result's "handle" from search --json
wallpicker apply-online '<handle>' --download-only --tag landscape --tag "digital art"
```

`apply` and `apply-online` use the native renderer and are recorded like an apply from
the picker; `apply-online` downloads (or reuses) the file first, with the same host and id checks,
keeping its metadata (artists, uploader, source) and tagging it with any `--tag`s.
`--thumbs` caches result thumbnails in `~/.cache/wallpicker/online/` (and `details --preview`
a bigger image in `preview/` there); cached images unused for 14 days are removed. Exit codes: 0 on success,
1 if anything failed, 2 for usage mistakes.

## Native wallpaper renderer

`wallpicker daemon` draws the wallpapers on Hyprland's background layer (wlr-layer-shell), at each
monitor's own pixels whatever its scale.

- **Daemon**: applying starts it when it isn't running. To have your wallpapers from login, add
  it to Hyprland's autostart (`wallpicker daemon`); it shows each monitor's saved wallpaper,
  including a monitor plugged in later.
- **Fit**: Cover (the default), Contain (black bars), Fill and Tile (at the picture's own size,
  from the top left). Adjusted and spanned wallpapers are cut to the monitor's size first, before applying.
- **Memory**: about 8 MB for the shared-memory renderer before shader initialization. A picture
  is read and fitted when it changes (or a monitor's resolution or scale does), then let go:
  Hyprland keeps its own copy. Shader effects add the driver's EGL/GLES context and temporary
  GPU textures; no image buffers are blended on the CPU each frame.
- **Transitions**: Fade, wipe, wave, circle grow/shrink, slides from all four directions, and
  random selection. Fade uses compositor opacity; the other effects use GPU shaders at the
  monitor's refresh rate. The configured duration applies to each effect. Shader transitions
  load EGL and Wayland EGL libraries at runtime and fall back to Fade if hardware GLES is
  unavailable; without compositor alpha-modifier support, changes are instant.
- **Animated wallpapers** (experimental; Settings → Animated wallpapers, or `wallpicker settings
  set animation.enabled=true`): GIF and animated WebP play on the GPU, shared by every monitor
  showing them, and keep playing through zoom, pan and flips. Off, or without hardware EGL, they
  show their first frame. The grid marks them with a film badge (thumbnails stay the first
  frame); **Details** plays one while it's open. The control panel's **Playback** shows what a
  monitor's animation is doing and pauses that wallpaper alone (everywhere it's shown, one
  clock; paused, it stays so until played or replaced, a restart too: `wallpicker adjust
  MONITOR --pause|--play`); its pill shows a play or pause mark. **Pause all** (Settings →
  Animated wallpapers, or `animation.paused=true`) holds every frame at once. **Frame rate** caps new frames per
  second on each monitor (`animation.max_fps`, default 60; 0: the display's refresh). The
  daemon follows these at once (edits of the settings file by hand within a second) and plays
  the saved animations again when it starts. That section shows what each monitor's
  animation is doing (playing, fps, paused, resting, a still and why). A monitor
  plugged in later plays its animation too. While
  no monitor showing an animation asks for frames (locked, powered off) it rests: nothing is
  decoded, and it continues from the same frame afterwards. Limits are provisional: up to four decoders, each a separate process
  capped at 64 MiB plus four frames of address space (97 MiB at 1080p, 196 MiB at 4K), and
  files up to 64 MiB.
- **Renderer usage** (Settings → Performance): the daemon's share of the GPU's time and the GPU
  memory it holds (from the kernel's DRM client statistics: amdgpu, Intel and others; NVIDIA's
  own driver doesn't report them), the whole GPU's busy percentage (AMD), its CPU and memory,
  and the animation decoders'.
- Only one program should draw the wallpaper: stop hyprpaper, awww and Noctalia's own wallpaper
  (Settings warns about them).

## Migrating from external wallpaper backends

wallpicker always uses its native renderer. Existing `wallpaper_backend` settings are ignored
and removed when settings are next saved; monitor assignments, adjustments, history and presets
stay intact. Supported transition kinds and durations are preserved; Simple fade falls back to
Fade. The old frame-rate setting is ignored because transitions follow the monitor's refresh rate.

Stop hyprpaper, awww/swww and Noctalia's wallpaper manager, and remove their wallpaper daemons
from autostart. Settings warns about competing wallpaper programs. Keep Noctalia running if you
use its color integration; enable **Noctalia's colors follow the wallpaper** in Appearance.
The former Noctalia picker plugin is retired; open the picker with `wallpicker --toggle`.
The old `~/.local/state/wallpicker/hyprpaper.conf` is no longer generated or used by wallpicker.

## The floating panel

The picker is a layer-shell overlay (namespace `wallpicker`) on the focused monitor, not a window:
no window rule is needed. Its size follows the screen (Settings → Appearance → Picker size), with
a light dim around it; Esc or a click outside closes it. A file dialog (Settings → Backup) is a
normal window, so the panel steps aside while one is open. `WALLPICKER_WINDOW=1` runs it as a
normal window instead (the headless checks do).

## Files

| what | where |
| --- | --- |
| settings (folders, download folder, API key, fit mode) | `~/.config/wallpicker/config.json` (mode 600) |
| the picker's own choices (size, dark mode, theme, sort, layout) | `~/.local/state/wallpicker/ui.json` |
| current wallpaper per monitor | `~/.local/state/wallpicker/state.json` |
| saved wallpaper presets | `~/.local/share/wallpicker/presets.json` |
| favorites and tags | `~/.local/share/wallpicker/library.json` |
| online provider and filters from last session | `~/.local/state/wallpicker/online.json` |
| thumbnails | `~/.cache/wallpicker/thumbs/` (safe to delete) |
| online downloads (`wallhaven-<id>`, `konachan-<id>`, `yandere-<id>`) | `~/Pictures/Wallpapers/wallhaven/` by default (Settings → Downloads) |

XDG variables are respected.

## How applying works

The CLI and picker prepare adjusted images, then ask `wallpicker daemon` to show them on the
requested monitors. The daemon starts when needed. Successful applications update state and
history. `wallpicker --restore` re-applies saved wallpapers, skipping disconnected monitors
and missing files; the daemon also restores them when it starts.

## Online notes

- The provider and each provider's filters are remembered between sessions; the search text
  isn't (it's shared between providers, so a term can be tried on each). Sorting starts on
  **Latest**.
- Results come in pages of 24, with a pager under the grid; **Jump to** takes a page number
  (Local has it too). Pages re-run the last search, so
  changing the filters takes effect on the next search (the dialog's **Search** button runs it).
- Downloads are only accepted from each provider's image host (`https://w.wallhaven.cc/`,
  `https://konachan.net/image/` or `.com`, `https://files.yande.re/image/`), must be images, and
  are capped at 64 MiB. Files are named
  `<provider>-<id>.<ext>`, and an existing file is reused instead of downloaded again.

### Wallhaven

- No account is needed. An **API key** (wallhaven.cc → Settings → Account) is only needed for
  NSFW results. It's sent as the `X-API-Key` header, never in the URL.
- The search box takes Wallhaven's syntax: `+tag -tag`, `@uploader`, `id:<tag id>` (the number
  in a `wallhaven.cc/tag/<id>` link). Artists are tags in the "Artists" category.
- `like:<id>` (similar wallpapers) doesn't work, on the website either: Cloudflare in front of
  Wallhaven answers any search containing "like" with a browser check, which shows up as a 403.
- Opening the details dialog makes one extra request, to fetch the tags.
- Wallhaven allows 45 requests per minute; past that you get a "rate limit" toast.

### Konachan and yande.re (Moebooru)

Both run Moebooru, so they share one module (`crates/core/src/moebooru.rs`) and one filters form;
each is its own provider with its own saved filters.

- No account or key.
- Konachan: Settings → Konachan picks **konachan.net** (safe-for-work only, the default) or
  **konachan.com** (all ratings). Same posts and ids on both, so downloads count as downloaded
  either way. On konachan.net the ratings filter is hidden and Safe is always searched.
- Ratings (Safe / Questionable / Explicit) start on Safe only. Any combination becomes one
  metatag (`rating:s`, `-rating:e`, ...).
- The search box takes tags (`long_hair night`, `-rain` to exclude, `user:<name>` for an
  uploader). Sorting, ratings and minimum size are added as `order:`, `rating:` and
  `width:`/`height:` metatags.
- Each search makes two requests: the results, and the result count for the pager (only the
  XML API reports it). Without the count, the pager still knows whether there's a next page.
- yande.re has scans of 100+ MiB. Originals over 64 MiB are downloaded as the site's
  full-resolution JPEG instead.
- Searches match **exact tag names** (Konachan's artist "mitsu" is `mitsu_(mitsu_art)`). As you
  type, the real tags it may mean show under the search box, most used first; click one to search
  with it. Separate tags with commas to type them with spaces ("flandre scarlet, night" is
  `flandre_scarlet night`); without commas, spaces separate tags as on the sites.
- Details come with the search results, so the dialog makes no request. The search asks for
  each tag's kind (API version 2), so artists are marked: they get **More by <artist>** and
  downloads keep them as their artists, as on Wallhaven. Some posts (more on yande.re) name no artist.

## Layout

```
crates/core/        all logic that doesn't need a desktop: native wallpaper application,
                    the daemon's socket and fitting a picture for its renderer, monitor parsing, settings/state files, folder scanning, thumbnails, colors,
                    duplicates, favorites/tags, backups, and the online providers: `online.rs`
                    (shared types), `wallhaven.rs`, `moebooru.rs`. `cargo test -p wallpicker-core`
crates/wallpicker/  the `wallpicker` binary: service/ (what it does, by area), serve/ (the
                    picker's backend), cli/ (the command line, the daemon, `ui`), the renderer
                    (the wallpicker backend), the HTTP calls to the providers, Noctalia's colors,
                    the file dialogs (XDG portal)
quickshell/         the picker (QML), see "How it fits together"
packaging/          the launcher entry and the icon
scripts/            install-local.sh, ui-check.sh, memory.sh, open-timing.sh
```

The shared `wallpicker_core::media` foundation detects JPEG/PNG/GIF/WebP by content and
provides metadata, a representative RGBA frame and sequential composited frames:

```rust
let source = media::Source::open(path, media::Limits::default())?;
let first_frame = source.representative()?;
let frames = source.frames()?; // One pass; reopen to repeat with a clean canvas.
```

GIF disposal and WebP blending/disposal have deterministic fixtures, including lossy WebP
and alpha patches. Metadata caching uses file identity and subsecond timestamps. Input,
dimensions, frame bytes and frame count have limits; codec scratch-memory limits remain
best effort when calling the core API directly. `wallpicker::decoding` adds isolated Linux
decoder processes with hard address-space limits, admission reservations, bounded frame
queues, pause and cancellation. It repeats from a fresh decoder at loop boundaries and
preserves source timing. `wallpicker::preparation` validates complete batches and owns one
paused worker/first frame per shared animated source, with identity checks before handoff
and cleanup on preflight failure. Static daemon requests use the same preparation path.
The daemon plays animations when the experimental `animation.enabled` setting is on; off,
an animated wallpaper is shown as its first frame.
See the [implementation design](docs/animated-wallpapers.md) and
[worker qualification](docs/decoder-workers.md) for integration and resource measurements.

`renderer/gpu/playback` provides persistent EGL surface primitives, shared source textures
and GPU fitting/framing/flips with alpha-correct filtering. Headless EGL pixel checks pass;
the renderer host also passes actual Wayland/GPU playback and transition lifecycle checks on private
virtual outputs. See [Wayland playback host](docs/wayland-playback.md) and
[GPU playback primitives](docs/gpu-playback.md) for the tested scope and memory accounting.

`wallpicker::playback` owns prepared source sessions with monotonic timing, bounded catch-up,
independent pause reasons and late-error freezing. Per-output pacing decisions combine
callback backpressure and explicit rate caps without retaining frame histories. Deterministic
clock tests and real-worker tests pass. Its calloop adapter drives deadlines and callback
tickets through a host draw hook and retires workers off-loop with bounded admission.
The Wayland host connects those hooks for qualified playback and live geometry adjustments
that reuse the source clock and texture. Incoming transitions hold frame zero until the exact
endpoint callback on each output, then start the shared clock. Without hardware EGL, the
qualified path displays a budgeted representative still and releases its decoder. Public
daemon acceptance and playback policy remain open. See
[playback scheduling](docs/playback-scheduling.md).

`wallpicker_core::render` carries original sources and portable geometry through versioned
batch requests. Apply, presets, spans, Previous, adjustments and restoration use this path;
static transformed files remain cached. Resized framed outputs rebuild from their original
source. Span state/history updates only for successful outputs, with partial spans retained
as standalone cuts. Application IDs and apply/adjust/restore intent are runtime metadata.
The new client requires a daemon with render protocol version 1 and reports when it needs
a restart. Animated requests retain their original identities; with playback off the
renderer shows their first frame. Static preflight reads image headers (presets decode every image completely); a
source that can't be prepared fails only its own monitor.

## Troubleshooting

- **"Could not list monitors"**: output discovery needs access to your Wayland session
  (`WAYLAND_DISPLAY` and `XDG_RUNTIME_DIR`). Hyprland adds workspace/focus metadata when available.
- **Applying fails**: run `wallpicker daemon` in a terminal to see renderer errors. The compositor
  needs wlr-layer-shell; stop other wallpaper programs if they cover wallpicker's surfaces.
- **Thumbnails show "No preview"**: the image couldn't be decoded (PNG, JPEG, WebP and GIF are
  supported).
- **The picker doesn't open**: `qs` (quickshell) must be installed; `qs log -p
  ~/.local/share/wallpicker/ui` shows the last run's log. "the interface isn't installed": run
  `scripts/install-local.sh`.
- **The picker opens but stays empty**: its backend is `wallpicker serve`; the picker shows a
  failure if it stopped, and `qs log` has its messages.

## Credits

The picker's font is [Nunito](https://github.com/googlefonts/nunito), under the SIL Open Font
License (`quickshell/fonts/OFL.txt`).
