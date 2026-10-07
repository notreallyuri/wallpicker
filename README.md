# wallpicker

A visual wallpaper picker for Hyprland, drawing through **hyprpaper**, **awww** (formerly swww)
or **Noctalia**'s wallpaper manager (Settings → General → Wallpaper backend).

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
- **Favorites and tags**: star a local wallpaper to pin it to the top of every view. Give it
  tags with right-click → **Edit tags…**, then filter by one or more tags above the grid (one
  line, selected tags first; **Expand** shows them all),
  matching **All** or **Any** of them. Downloads arrive tagged with their artist, when the
  provider lists one. The name filter also matches tags.
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
  Artists (Wallhaven's "Artists" category, Konachan's and yande.re's artist tags) are shown
  first, with a **More by <artist>** button.
- **Collections**: your Wallhaven collections (private ones too), shown like search results:
  apply, download, details. Needs your API key and your Wallhaven username (Settings →
  Wallhaven): opening a collection needs the name, and the API doesn't say whose key it is.
  Wallhaven's API is read-only, so collections can be browsed but not changed from here.
- **Per monitor**: the bar at the top picks which monitors a click applies to, and shows what each
  one currently has (with a thumbnail).
- **Fit**: a general fit in Settings, and a fit per monitor under it (say, Contain on a 4:3
  screen). With the Noctalia backend, Noctalia's own fill mode applies.
- **Adjust** (the monitor drawer's adjust button, or right-click a monitor pill): **frame** what
  a monitor shows (drag the preview to move, scroll or the sliders to zoom up to 4×; it's cut to
  the monitor's exact resolution), **mirror** it (left to right, top to bottom), or give it its
  own **fit**. The preview follows at once; the monitor a moment after you stop. Temporary:
  another wallpaper on that monitor goes on as it is. No backend frames or mirrors, so wallpicker
  makes the changed copy (cached, deleted when unused).
  `wallpicker adjust <monitor> --frame 200,70,40 --flip h --fit contain`.
- The search bar (and the tags and folder path under it) stays at the top while you scroll.
- **Keys**: Ctrl+F search field, Ctrl+R random, Alt+Up parent folder, Alt+Left/Right online pages,
  Ctrl+1/2/3 Local/Online/Collections.
- **Duplicates** (Local → **⋯** → **Find duplicates…**): exact copies and the same picture at another size or
  re-saved, a group at a time. The largest copy is picked to keep; the others go to the trash, and
  their favorite and tags move to the kept one (a monitor showing one of them gets it instead).
  Pictures that only look alike (the same artist's figure on white) aren't matched.
- **Slideshow** (Settings → Slideshow): a random wallpaper every few minutes, from favorites,
  some tags, a folder or a color, on all or some monitors, the same one or a different one on
  each. **Something else at night**: a second set for a time window (say 20:00–07:00, Dark), and
  it changes right away when night starts and ends. `wallpicker random --color dark` too. The
  Noctalia plugin keeps time for it while Noctalia runs; without Noctalia, run
  `wallpicker slideshow run` (say, from Hyprland's autostart). **Change now** changes it at once.
- **A wallpaper per workspace** (Settings → Workspaces): when a monitor switches to a Hyprland
  workspace that has a wallpaper, it goes on that monitor. With **Remember what I put on each
  workspace**, applying a wallpaper is all it takes: it becomes the wallpaper of the workspace its
  monitor shows (Random and the slideshow don't change them). Switching doesn't go into the
  history, so Previous still means the one you chose before. Or pick one per workspace right
  there: click its picture (or the empty "+ Choose" slot) to open your library in place,
  searchable by name and tag.
  wallpicker follows the workspaces
  while it runs (the tray is enough); without it, `wallpicker workspaces run`. Also
  `wallpicker workspaces list | on | off | set 3 image.png | clear 3`.
- **Tray**: closing the window keeps wallpicker in the tray (Settings → General). The window is
  closed for real then, so the tray costs ~30 MB instead of ~340 MB; it opens again in a moment. The tray menu
  shows or hides the window, applies a random wallpaper, or quits. Running `wallpicker` again
  brings the window back instead of starting a second app; `wallpicker --toggle` shows or hides it
  (for a keybind), and `wallpicker --tray` starts with only the tray icon (for an autostart).
  The tray needs a tray host in the bar (Noctalia has one) and libappindicator.
- **Color themes** (Settings → Appearance): Blossom, Matcha, Peach, Lagoon, Lavender and Honey,
  each with a light and a dark half; **Dark mode** is on, off or like the system.
- **Colors from the wallpaper** (Settings → Appearance): the app takes its colors from the
  wallpaper on the monitor you pick (the first one by default), made by Noctalia's palette generator with its wallpaper scheme,
  so it matches Noctalia. **Noctalia's colors follow the wallpaper** switches Noctalia's color
  source to the wallpaper too (and back to the scheme it had). The panel uses Noctalia's colors, so
  it follows along. `wallpicker theme` shows the palette; `wallpicker noctalia-colors on|off`.
- **Color** (Local, next to the sort): show the wallpapers with a color (red, orange, yellow, green,
  teal, blue, purple, pink) or tone (Dark, Light, Warm: golden, brown, firelight; Muted: mostly
  white, grey or black), the most of it first. Each pixel is named by the nearest of a set of
  reference colors, so skin and wood aren't "red", gold is yellow and light pink is pink. Read
  from the thumbnails once, then cached. `wallpicker colors [COLOR]`.
- **Backup** (Settings → Backup): your favorites, tags and fits are copied before the first change
  of each day (the last 14 kept; restore any of them), in a folder you can choose (a synced one
  makes them off-machine backups). **Export…** and **Import…** use the system's file dialog; an
  import is merged (or replaces the library) and finds wallpapers that moved, by their place
  inside your wallpaper folders or by name and size.
  `wallpicker backup export|import|snapshots|restore`, `settings set snapshot_dir=…`.
- **Monitors** (the chevron after "All" in the top bar): a drawer with each monitor bigger, showing
  its wallpaper in the monitor's own shape and the file; click one to make it the target, or ↶ to
  go back on that monitor only. Remembered open or closed.
- **Previous**: each monitor remembers its last 30 wallpapers; **↶ Previous** (in the monitor
  bar, the tray menu, the panel, or `wallpicker previous` for a keybind) goes back through them.
- **Saved choices**: every applied wallpaper is written to `state.json`, and
  `wallpicker --restore` re-applies them at login.
- **Settings**: the cog at the top right.

Built with Tauri 2 + Leptos 0.8 (CSR), with the UI on
[nocomponents](https://crates.io/crates/nocomponents) (styled layer installed with `cargo nocli`).

## Requirements

On Arch:

```bash
sudo pacman -S --needed base-devel webkit2gtk-4.1 hyprpaper   # or awww instead of hyprpaper
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cargo install tauri-cli --version "^2" --locked
```

With hyprpaper, it must be running with IPC enabled, which is its default. With awww, wallpicker
starts its daemon when needed.

## Run and build

wallpicker is two binaries:

- `wallpicker` (`crates/wallpicker`): the command line, and `wallpicker serve`, the backend of the
  new Quickshell interface (`wallpicker ui`). No Tauri, WebKit or GTK: a plain cargo build.
- `wallpicker-app` (`src-tauri`): the old Tauri window, kept working until the Quickshell
  interface replaces it, no longer developed. `wallpicker` with no arguments, `--tray`, `--toggle`
  or `--open` hands over to it.

```bash
cargo build --release -p wallpicker   # target/release/wallpicker
cargo tauri build --no-bundle         # target/release/wallpicker-app (the window)
scripts/install-local.sh              # both in ~/.local/bin, the interface, a launcher entry
cargo tauri dev                       # the window, with trunk serving its UI on :1420
```

`install-local.sh` also adds wallpicker to your app launcher (`wallpicker.desktop`, with Random
and Previous as quick actions). Its entry uses the full path to the binary, since launchers
often run with a PATH that doesn't include `~/.local/bin`. It refuses a build older than its
sources, so a new interface never goes in with an old backend.

### Starting it

The app doesn't need to run all the time: the Noctalia panel is the quick picker, the plugin keeps
the slideshow going, and anything bigger in the panel opens the app where it's needed
(`wallpicker --open local|online|collections|settings|duplicates`, `--open tags <image>`).

- Open it from the launcher, the tray menu, the panel, or a keybind on `wallpicker --toggle`. Any
  of these brings up the one running app.
- Optionally autostart `wallpicker --tray` (in Hyprland's autostart) to keep it in the tray from
  login: it opens instantly then, at the cost of keeping it in memory.
- Closing the window keeps it in the tray; quit from the tray menu.

Build the window with `cargo tauri build`, not `cargo build --release`: the plain cargo build of
`wallpicker-app` is a development build whose window loads the UI from the dev server on :1420
instead of embedding it. Started without `cargo tauri dev` running, it says so and exits.
`--no-bundle` skips making .deb/AppImage packages.

The first trunk build downloads the Tailwind binary (pinned to 4.3.0 in `ui/Trunk.toml`).

**Measuring memory**: `cargo tauri build --no-bundle --features automation`, then
`scripts/memory/run.sh [default|nofx|noimg]`. It replays a browsing session (Online pages,
details, another provider, Local) in a headless copy with a throwaway profile, and records each
process's memory every second. The `automation` feature only exists for this; normal builds
don't have it.

## The new interface (Quickshell, in progress)

The interface is moving to [Quickshell](https://quickshell.org) (QML) for a lighter, faster
window; the plan is in TODO.md ("Quickshell frontend"). `wallpicker ui` opens it (the Local tab
so far), `wallpicker ui --toggle` opens or closes it (a keybind). Closing exits: nothing stays in
memory between opens. It needs `qs` (the quickshell package).

- `quickshell/`: the interface, split by focus: `services/` (state and the connection to Rust),
  `components/`, `bar/`, `views/`, `windows/`.
- `wallpicker serve`: its backend, JSON lines over stdin/stdout (`src-tauri/src/serve/`), on the
  same logic as the Tauri app's commands (`src-tauri/src/service/`).
- `scripts/ui-check.sh`: runs it headless with a throwaway profile, clicks and scrolls through
  test hooks, screenshots, and fails on QML errors.



The same library, settings and backend as the app, without the window, for scripts and keybinds:

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
wallpicker slideshow tick        # change it if it's due (the Noctalia plugin calls this every minute)
wallpicker slideshow tick --now  # change it now
wallpicker slideshow run         # keep it going without Noctalia (Ctrl+C to stop)
```

Library edits, the same as the app's:

```bash
wallpicker favorite ~/Pictures/wpp/a.jpg [--off]
wallpicker tag ~/Pictures/wpp/a.jpg add "dark mode"     # or remove
wallpicker trash ~/Pictures/wpp/a.jpg                   # desktop trash; wallpaper/download folders only
wallpicker show-in-folder ~/Pictures/wpp/a.jpg          # file manager, with it selected
wallpicker duplicates                                   # copies and resized versions, best first
wallpicker duplicates keep ~/Pictures/wpp/a.png ~/Pictures/wpp/a-small.jpg   # keep one, trash the rest
wallpicker fit ~/Pictures/wpp/a.jpg contain             # its own fit everywhere (auto: the monitor's)
wallpicker settings set monitor_fit.DP-2=contain        # a monitor's fit ("" for the general one)
```

Filters and settings, the same files as the app's Filters dialog and Settings (JSON out):

```bash
wallpicker filters                                          # every provider's filters + options
wallpicker filters wallhaven set sort=toplist range=1w atleast=1280x720 nsfw=true
wallpicker filters wallhaven set resolutions=1920x1080,2560x1440 ratios=16x9,4x3   # "" for any
wallpicker filters yandere set questionable=true            # or: filters yandere reset
wallpicker settings
wallpicker settings set backend=noctalia thumb_width=480 "folders=~/Pictures/wpp
~/Pictures/more"                                            # one folder per line
```

Wrong keys or values change nothing. The Wallhaven API key is never printed and can only be set
in the app, so it never sits on a command line.

Online, with each provider's saved filters (the app's Filters dialog):

```bash
wallpicker search konachan landscape --page 2 --json --thumbs
wallpicker search wallhaven --sort toplist        # also saves the sort for the app
wallpicker details '<handle>'                     # tags, artist, uploader, source (JSON)
wallpicker tags konachan mitsu                     # the real tag names a Konachan/yande.re search means
wallpicker collections                             # your Wallhaven collections (API key)
wallpicker collection 2117134 --page 2 --json      # a page of one (needs your username in Settings)
wallpicker preview '<handle>'                     # its sharpest image, cached (JSON)
wallpicker apply-online '<handle>' --monitor DP-1  # a result's "handle" from search --json
wallpicker apply-online '<handle>' --download-only
```

`apply` and `apply-online` go through the configured backend and are recorded like an apply from
the app; `apply-online` downloads (or reuses) the file first, with the same host and id checks.
`--thumbs` caches result thumbnails in `~/.cache/wallpicker/online/` (and `details --preview`
a bigger image in `preview/` there); cached images unused for 14 days are removed. Exit codes: 0 on success,
1 if anything failed, 2 for usage mistakes.

## Noctalia plugin

`noctalia-plugin/` holds a Noctalia v5 plugin: a bar icon and a panel that works without the app
window: the quick picker. **Local**: your library (favorites first, filter by name or tag);
right-click a caption for apply/favorite/fit/trash, or click it for the wallpaper's own view.
**Online**: search Wallhaven, Konachan or yande.re with a sort, filters and paging;
already-downloaded results are marked. Pick a monitor, click to apply. Settings, Duplicates,
Collections and tag editing open in the app. It's built on the command line above.
See `noctalia-plugin/README.md` to install it.

## Noctalia as the backend

With **Noctalia** picked in Settings → General, wallpicker applies through
`noctalia msg wallpaper-set <monitor> <path>` (Noctalia v5). Noctalia saves it and restores it
itself, so nothing else is needed after a reboot, and its colors-from-wallpaper theming follows.
Its fill mode and transitions are Noctalia's own settings; wallpicker's fit mode doesn't apply.

- Turn Noctalia's wallpaper on in its settings. While it's off, wallpicker refuses to apply rather
  than appear to work.
- Stop hyprpaper (take it out of your Hyprland autostart): both draw the background.

## awww (swww) as the backend

With **awww (swww)** picked in Settings → General, wallpicker applies through
`awww img <image> --outputs <monitor> --resize <fit> --transition-type <…>` (or `swww`, on an
install from before the rename). Its daemon keeps the wallpapers across restarts.

- **Daemon**: when `awww-daemon` isn't running, wallpicker starts it and applies. To have it from
  login, add it to Hyprland's autostart (`exec-once = awww-daemon`), or run `wallpicker --restore`
  there, which starts it and puts your wallpapers back.
- **Fit**: wallpicker's fit modes map to awww's: Cover → crop, Contain → fit, Fill → stretch.
  awww can't tile, so Tile shows the image centered at its own size.
- **Transition**: Settings → General (Transition, Duration, Frames per second), or
  `wallpicker settings set transition.kind=wipe transition.duration_ms=800 transition.fps=144`.
- Only one program should draw the wallpaper. Settings warns when another one is running
  (hyprpaper, awww, Noctalia's own wallpaper, swaybg…), with **Stop it** for each program (awww
  with `awww kill`); Noctalia's own wallpaper is turned off in its settings. Take a stopped
  program out of Hyprland's autostart too, or it comes back at login.

## Keeping wallpapers after hyprpaper restarts

wallpicker only *sets* wallpapers through hyprpaper; it doesn't run in the background. So that
hyprpaper comes back with your choices whenever it starts (at login, or restarted by hand),
wallpicker keeps a hyprpaper config of its own up to date:
`~/.local/state/wallpicker/hyprpaper.conf`, one `wallpaper { }` block per monitor, rewritten on
every apply, trash and fit-mode change. Source it from `~/.config/hypr/hyprpaper.conf`:

```ini
source = ~/.local/state/wallpicker/hyprpaper.conf
```

and start hyprpaper from your Hyprland autostart (`hyprpaper` in the Lua config's startup
commands, or `exec-once = hyprpaper` in hyprlang). This needs hyprpaper 0.8's config syntax.
Don't also set wallpapers for the same monitors in `hyprpaper.conf`.

A path containing `$` or a line break can't be written into hyprpaper's config language, so that
monitor is left out of the file (with a comment saying so).

`wallpicker --restore` still works as an alternative (for older hyprpaper versions): run it after
hyprpaper starts. It retries for about 20 seconds, skips monitors that aren't connected and files
that were deleted, and says so on stderr.

**wallpiper**: both it and hyprpaper draw on the background layer, so on a monitor where
wallpiper is running, it covers whatever wallpicker sets.

## Floating window

The window is a fixed 1100×760 (not resizable). Hyprland floats fixed-size windows on its own,
but centering is up to the compositor on Wayland, so a window rule does it. In the Lua config:

```lua
hl.window_rule({
  match = { class = "^(dev\\.wallpicker\\.app|wallpicker)$" },
  float = true,
  center = true,
  size = "1100 760",
})
```

With a hyprlang config, write the same rule (float, center, size, matched on the class) in your
Hyprland version's `windowrule` syntax. The class depends on the Tauri version (the identifier or
the binary name), so the rule matches both. `hyprctl clients` shows the one in use while
wallpicker is open.

## Files

| what | where |
| --- | --- |
| settings (folders, download folder, API key, fit mode) | `~/.config/wallpicker/config.json` (mode 600) |
| current wallpaper per monitor | `~/.local/state/wallpicker/state.json` |
| the same, as hyprpaper config (for `source =`) | `~/.local/state/wallpicker/hyprpaper.conf` |
| favorites and tags | `~/.local/share/wallpicker/library.json` |
| online provider and filters from last session | `~/.local/state/wallpicker/online.json` |
| thumbnails | `~/.cache/wallpicker/thumbs/` (safe to delete) |
| online downloads (`wallhaven-<id>`, `konachan-<id>`, `yandere-<id>`) | `~/Pictures/Wallpapers/wallhaven/` by default (Settings → Downloads) |

XDG variables are respected.

## How applying works

hyprpaper's control interface changed between versions, so wallpicker tries the direct form first:

```
hyprctl hyprpaper wallpaper "DP-1,/path/img.png"
```

and, if hyprpaper refuses it, falls back to the older two-step form
(`preload` then `wallpaper`, then `unload unused` to free memory). If both fail, the error toast
shows hyprpaper's own reply.

The **fit mode** setting adds a third field (`DP-1,/path,cover`), which only newer hyprpaper
versions understand. Leave it on "hyprpaper default" if applying fails with it set.

Paths containing a comma can't be applied: hyprpaper splits its argument on commas.

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
  downloads are tagged with them, as on Wallhaven. Some posts (more on yande.re) name no artist.

## Layout

```
crates/core/        all logic that doesn't need a desktop: hyprpaper commands, monitor parsing,
                    settings/state files, folder scanning, thumbnails, favorites/tags, and the
                    online providers: `online.rs` (shared types), `wallhaven.rs`, `moebooru.rs`.
                    76 unit tests: `cargo test -p wallpicker-core`
src-tauri/          the Tauri backend: thin commands over core, HTTP calls to the providers,
                    --restore
ui/                 the Leptos frontend (trunk), reusing core's types (no native features)
ui/src/app.rs           the shell: header tabs, monitor bar, tab panels
ui/src/ctx.rs           app-wide state (Ctx) and the apply/toast actions
ui/src/views/           local/ (view, tile, tag filter and dialog), online/ (view, pager, tile,
                        details dialog, one filters file per provider), settings (the Settings
                        dialog, one section per topic and provider)
ui/src/tile_menu.rs     the wallpaper grid and its right-click menu
ui/src/components/nc/   nocomponents styled layer, installed by `cargo nocli` (yours to edit)
ui/styles/          nocomponents' globals.css (theme tokens) and animate.css
noctalia-plugin/    the Noctalia v5 plugin (Luau), a panel over `wallpicker list` / `apply`
```

To add another nocomponents component, run `cargo nocli components add <name>` inside `ui/`.

## Troubleshooting

- **"Could not list monitors"**: wallpicker calls `hyprctl monitors -j`; run it in a terminal
  to see the error.
- **"hyprpaper is not running"**: start `hyprpaper`, and add it to your Hyprland autostart
  (see "Restore at login").
- **Applying fails**: the error toast includes hyprpaper's reply. To test by hand, run
  `hyprctl hyprpaper wallpaper "DP-1,/full/path/to/image.png"` with one of your monitor names.
- **Thumbnails show "No preview"**: the image couldn't be decoded (PNG, JPEG and WebP are
  supported).
- **Blank window in a release build only**: the CSP in `src-tauri/tauri.conf.json` applies to
  release builds but not to `cargo tauri dev` (`devCsp` is null), so a CSP mistake only shows up
  after `cargo tauri build`. `connect-src` must keep `'self'`: the UI's WebAssembly is fetched
  from the app itself, and without it nothing starts. To find a blocked request, setting
  `"csp": null` is a quick way to confirm it's the CSP.
- **Blank window in `cargo tauri dev`**: open the web inspector (right-click → Inspect) and look
  at the console.

## Credits

The app's font is [Nunito](https://github.com/googlefonts/nunito), under the SIL Open Font License
(`ui/assets/fonts/OFL.txt`).
