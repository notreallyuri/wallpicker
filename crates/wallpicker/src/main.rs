//! `wallpicker`: the command line and the Quickshell interface's backend.
//! The window's own arguments (none, `--tray`, `--toggle`, `--open`) go to
//! the old Tauri window, `wallpicker-app`, until the Quickshell interface
//! takes its place.

use std::os::unix::process::CommandExt;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("list") => std::process::exit(wallpicker::cli::list(&args[1..])),
        Some("apply") => std::process::exit(wallpicker::cli::apply(&args[1..])),
        Some("search") => std::process::exit(wallpicker::cli::search(&args[1..])),
        Some("filters") => std::process::exit(wallpicker::cli::filters(&args[1..])),
        Some("settings") => std::process::exit(wallpicker::cli::settings(&args[1..])),
        Some("slideshow") => std::process::exit(wallpicker::cli::slideshow(&args[1..])),
        Some("theme") => std::process::exit(wallpicker::cli::theme(&args[1..])),
        Some("noctalia-colors") => std::process::exit(wallpicker::cli::noctalia_colors(&args[1..])),
        Some("previous") => std::process::exit(wallpicker::cli::previous(&args[1..])),
        Some("random") => std::process::exit(wallpicker::cli::random(&args[1..])),
        Some("favorite") => std::process::exit(wallpicker::cli::favorite(&args[1..])),
        Some("tag") => std::process::exit(wallpicker::cli::tag(&args[1..])),
        Some("colors") => std::process::exit(wallpicker::cli::colors(&args[1..])),
        Some("backup") => std::process::exit(wallpicker::cli::backup(&args[1..])),
        Some("duplicates") => std::process::exit(wallpicker::cli::duplicates(&args[1..])),
        Some("ui") => std::process::exit(wallpicker::cli::ui(&args[1..])),
        Some("serve") => std::process::exit(wallpicker::serve()),
        Some("daemon") => std::process::exit(wallpicker::cli::daemon(&args[1..])),
        Some("workspaces") => std::process::exit(wallpicker::cli::workspaces(&args[1..])),
        Some("adjust") => std::process::exit(wallpicker::cli::adjust(&args[1..])),
        Some("show-in-folder") => std::process::exit(wallpicker::cli::show_in_folder(&args[1..])),
        Some("trash") => std::process::exit(wallpicker::cli::trash(&args[1..])),
        Some("tags") => std::process::exit(wallpicker::cli::tags(&args[1..])),
        Some("collections") => std::process::exit(wallpicker::cli::collections(&args[1..])),
        Some("collection") => std::process::exit(wallpicker::cli::collection(&args[1..])),
        Some("preview") => std::process::exit(wallpicker::cli::preview(&args[1..])),
        Some("details") => std::process::exit(wallpicker::cli::details(&args[1..])),
        Some("apply-online") => std::process::exit(wallpicker::cli::apply_online(&args[1..])),
        _ => {}
    }

    if args.iter().any(|a| a == "-h" || a == "--help") {
        println!(
            "wallpicker - wallpaper picker for hyprpaper and Noctalia\n\n\
             USAGE:\n  \
             wallpicker                         open the picker\n  \
             wallpicker list [--json] [--thumbs]\n                                     \
             monitors and the local library (favorites first);\n                                     \
             --thumbs adds cached thumbnail paths\n  \
             wallpicker apply <image> [--monitor NAME]...\n                                     \
             apply through the configured backend (default:\n                                     \
             every connected monitor)\n  \
             wallpicker search <provider> [query...] [--page N] [--sort S] [--json] [--thumbs]\n                                     \
             search wallhaven, konachan or yandere with the saved filters\n  \
             wallpicker details <handle> [--preview]  tags, artist, uploader and source, as JSON\n  \
             wallpicker collections [--json]    your Wallhaven collections (API key)\n  \
             wallpicker tags <konachan|yandere> <text...> [--json]  the real tag names a search means\n  \
             wallpicker collection <id> [--page N] [--json] [--thumbs]  a page of one\n  \
             wallpicker preview <handle>        just the details image (cached), as JSON\n  \
             wallpicker apply-online <handle> [--monitor NAME]... [--download-only]\n                                     \
             download and apply a result from `search --json`\n  \
             wallpicker random [--favorites] [--tag T]... [--any] [--folder DIR] [--monitor NAME]... [--each]\n                                     \
             apply a random wallpaper (for keybinds)\n  \
             wallpicker previous [--monitor NAME]...  back to the wallpaper before (every monitor by default)\n  \
             wallpicker theme [--json]          the colors from the current wallpaper (the app uses them)\n  \
             wallpicker noctalia-colors on|off|status  Noctalia's colors follow the wallpaper, or not\n  \
             wallpicker slideshow tick|run|status  change it every few minutes (Settings → Slideshow)\n  \
             wallpicker favorite <image> [--off]  pin to the top, or unpin\n  \
             wallpicker tag <image> add|remove <tag>\n  \
             wallpicker adjust <monitor> [--flip h|v|both|none] [--fit F] [--frame Z,X,Y] [--reset]\n                                     \
             adjust what a monitor shows, until another wallpaper goes there\n  \
             wallpicker workspaces [list | on | off | remember on|off | set WS IMAGE | clear WS | run]\n                                     \
             a wallpaper per Hyprland workspace\n  \
             wallpicker ui [--toggle]           the new interface (Quickshell; in progress)\n  \
             wallpicker daemon                  follow workspaces and keep the slideshow on time\n                                     \
             (for an autostart; one at a time)\n  \
             wallpicker duplicates [--json] | duplicates keep <image> <other>...\n                                     \
             find copies and resized versions; keep one, trash the rest\n  \
             wallpicker backup export [FILE] | import FILE [--replace] | snapshots | restore ID\n                                     \
             back up favorites, tags and fits; daily snapshots are automatic\n  \
             wallpicker colors [COLOR] [--json]  the library's colors, or the images that are COLOR\n                                     \
             (red orange yellow green teal blue purple pink, dark light warm muted)\n  \
             wallpicker show-in-folder <image>  open its folder, with it selected\n  \
             wallpicker trash <image>           move to the desktop trash (wallpaper/download folders only)\n  \
             wallpicker filters [<provider> set KEY=VALUE...|reset]  read or change the online filters\n  \
             wallpicker settings [set KEY=VALUE...]  read or change the settings\n  \
             wallpicker --restore               re-apply the saved wallpapers (hyprpaper) and exit\n  \
             wallpicker --tray                  start in the tray, window hidden (for an autostart)\n  \
             wallpicker --toggle                show or hide the window (opens it if not running)\n  \
             wallpicker --open local|online|collections|settings|duplicates\n                                     \
             show the window at that tab or dialog (opens it if not running)\n  \
             wallpicker --open tags <image>     edit an image's tags in the window\n\n\
             Running `wallpicker` while it's open brings up that window. Closing the window\n\
             keeps it in the tray (Settings → General); quit from the tray menu."
        );
        return;
    }

    if args.iter().any(|a| a == "--restore") {
        std::process::exit(wallpicker::restore_cli());
    }

    // The rest is the window's: none, --tray, --toggle, --open.
    std::process::exit(window(&args));
}

/// Hand over to the Tauri window, `wallpicker-app`: next to this binary
/// (installed, or a build), else on PATH. It checks `--open`'s arguments
/// itself, before a running window gets them.
fn window(args: &[String]) -> i32 {
    let beside = std::env::current_exe().ok().and_then(|exe| Some(exe.parent()?.join("wallpicker-app")));
    let app = beside.filter(|p| p.is_file()).unwrap_or_else(|| PathBuf::from("wallpicker-app"));
    let e = std::process::Command::new(&app).args(args).exec();
    eprintln!("wallpicker: could not start the window ({}): {e}", app.display());
    1
}
