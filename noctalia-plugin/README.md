# wallpicker for Noctalia

A Noctalia v5 plugin: a bar icon that opens wallpicker's quick picker in a panel. Bigger jobs
are the app's, and the panel hands them over (`wallpicker --open …`, which starts the app if it
isn't running): **Settings** (the cog), **Duplicates** (the copy icon), **Collections** (header)
and **editing tags** (a wallpaper's view, or right-click → Edit tags in wallpicker…).

- **Local**: your library, favorites first; filter by name or tag. The wallpaper on the chosen
  monitor is outlined, and tiles say which monitors show them and the image's size (amber,
  "low-res for DP-1", when it's smaller than the target monitor).
  - **Sort**: Newest, Oldest, Name or Random order (favorites stay first).
  - **Flat / Folders**: everything at once, or browse folder by folder (breadcrumb, Up, "show in
    folder"). Filtering in Folders searches the current folder and below.
  - **Tag chips**: click your tags to filter; "All of" / "Any of" decides how several combine.
    One line (selected first), with **Expand** for the rest.
  - **Now showing** (the monitor button): what each monitor shows; click one to make it the target.
  - **Random**: applies a random one of the wallpapers shown. The history button goes back to
    the wallpaper before.
  - Sort, layout and All/Any are remembered.
- **Online**: Wallhaven, Konachan or yande.re, with a sort and paging, and each provider's other
  filters as saved in the app. Already-downloaded results show ✓; the border shows the rating:
  green SFW, yellow sketchy, red NSFW. Clicking downloads and applies.
- Keys: Esc backs out of a view (then closes), Ctrl+F search field, Ctrl+R random, Alt+Up parent
  folder, Alt+Left/Right online pages, Ctrl+1/2 Local/Online.
- Right-click a caption for a menu: favorite, Edit tags in wallpicker…, Apply to (all, or one monitor), Show in
  folder, Copy path, Move to trash… (local) or
  Apply, Details…, Download only, Open the provider's page (online). Clicking an online caption
  opens its details: a big preview, stats, "More by" the artist, "More from" the uploader, the
  source link, and its tags; click a tag to search for it. Clicking a local caption opens the
  wallpaper's view: favorite, fit, its tags (click one to show that tag's wallpapers; "Edit in
  wallpicker" to change them) and trash, which asks first. Menus need Noctalia's plugin API 28.
- **Filters** (next to the search box, with a count when any are on): the provider's filters,
  the same ones as the app's Filters dialog. Saved as you change them; Done searches again.
- **Fit and mirroring** are per monitor, in the app (the monitor drawer's Adjust); the fits in the
  app's Settings apply here too.
- **Slideshow**: set up in the app (Settings → Slideshow). The plugin's background service asks
  wallpicker every minute whether it's time, so it runs without the app; it does nothing while
  the slideshow is off.
- On Konachan and yande.re, typing shows the real tag names it may mean (they match exact tags
  only); click one to search with it. Commas separate tags typed with spaces ("long hair, night").
- Thumbnails round their corners like the rest of Noctalia (its corner radius scale).
- Pick a monitor (or all) at the top; it applies to both. "Open wallpicker" opens the app on the
  tab you're on.

It goes through the `wallpicker` command (`list`, `apply`, `search`, `apply-online`), so it shares
the app's library, favorites, tags, saved state, filters and wallpaper backend (hyprpaper or
Noctalia). Noctalia is often started with a shorter PATH than your shell's, so the plugin also
looks for the command in `~/.local/bin` and `~/.cargo/bin`.

## Install

1. Build and install the `wallpicker` command somewhere on your PATH:

   ```bash
   cargo build --release -p wallpicker
   install -m755 target/release/wallpicker ~/.local/bin/wallpicker
   ```

   (Or set the command's full path in the plugin's settings.)

2. Add this folder as a plugin source and enable the plugin:

   ```bash
   noctalia msg plugins source add wallpicker path /path/to/wallpicker/noctalia-plugin
   noctalia msg plugins enable wallpicker/wallpicker
   ```

3. Put the **wallpicker** widget on a bar in Noctalia's settings, or open the panel directly:
   `noctalia msg panel-toggle wallpicker/wallpicker:picker` (handy for a keybind).

Check the plugin with `noctalia plugins lint noctalia-plugin/wallpicker`.

## Development

`wallpicker/panel.luau` wires the parts together; each file says what it holds (`shared.luau`
state and helpers, `local_view.luau`, `local_detail.luau`, `online_view.luau`,
`online_detail.luau`, `config.luau`, `library.luau`).

`tests/run.py` runs the panel under plain [Luau](https://github.com/luau-lang/luau/releases)
with a fake Noctalia (`tests/mock.luau`) and canned `wallpicker` output, drives it through every
view (`tests/smoke.luau`), and compares what it renders and runs with `tests/smoke.snap`:

```bash
tests/run.py              # compare (LUAU=/path/to/luau if it's not on PATH)
tests/run.py --update     # accept the new output after an intended change
```

Noctalia logs plugin errors to `~/.cache/noctalia/noctalia.log`. Note that Noctalia's `require`
needs the `.luau` extension (`require("./shared.luau")`) while plain Luau's refuses it; the test
runner bundles the files to get around that.
