//! The command line: the library, applying, online search and the settings,
//! without the window, for scripts, keybinds and the Noctalia plugin. It
//! uses the same settings, backend and saved state as the app.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::library;
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::thumbs;

mod config;
mod library_edits;
mod online;
mod random;
mod backup;
pub(crate) mod slideshow;

pub use config::{filters, settings};
pub use library_edits::{duplicates, favorite, fit, show_in_folder, tag, trash};
pub use online::{apply_online, collection, collections, details, preview, search, tags};
pub use backup::backup;
pub use random::random;
pub use slideshow::slideshow;

#[derive(Serialize)]
struct ListOutput {
    /// "hyprpaper" or "noctalia".
    backend: &'static str,
    monitors: Vec<MonitorOut>,
    /// Favorites first, then newest first (the app's order).
    images: Vec<ImageOut>,
    /// Every tag in use, with how many images have it (for suggestions).
    all_tags: Vec<TagCount>,
    /// The wallpaper folders, in the settings' order; images say which one
    /// holds them.
    roots: Vec<RootOut>,
}

#[derive(Serialize)]
struct RootOut {
    path: PathBuf,
    /// The folder's own name, for display.
    name: String,
}

#[derive(Serialize)]
struct TagCount {
    name: String,
    count: usize,
}

#[derive(Serialize)]
struct MonitorOut {
    name: String,
    width: u32,
    height: u32,
    focused: bool,
    /// What wallpicker last applied there.
    current: Option<PathBuf>,
}

#[derive(Serialize)]
struct ImageOut {
    path: PathBuf,
    name: String,
    modified: u64,
    favorite: bool,
    tags: Vec<String>,
    /// A small JPEG for grids, with `--thumbs` (made on first use, cached).
    thumb: Option<PathBuf>,
    /// Index into `roots` of the folder that holds it.
    root: Option<usize>,
    /// Its folder inside that root: "" at the top, "a/b" further down.
    folder: String,
    /// Its own fit ("cover", ...), or "" for the monitor's.
    fit: &'static str,
    /// Pixel size (0 if unknown).
    width: u32,
    height: u32,
}

/// `wallpicker list [--json] [--thumbs]`.
pub fn list(args: &[String]) -> i32 {
    let json = args.iter().any(|a| a == "--json");
    let want_thumbs = args.iter().any(|a| a == "--thumbs");
    let store = Store::new(Dirs::from_env());
    let settings = match store.load_settings() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let catalog = store.load_catalog().unwrap_or_default();
    let state = store.load_state().unwrap_or_default();
    // No Hyprland (say, over ssh): an empty monitor list, not an error.
    let monitors = hyprpaper::list_monitors(&SystemRunner).unwrap_or_default();

    let mut images = library::scan(&settings.folders, settings.recursive);
    let all_tags = catalog
        .tag_counts(images.iter().map(|i| i.path.as_path()))
        .into_iter()
        .map(|(name, count)| TagCount { name, count })
        .collect();
    // `sort_by_key` is stable, so newest-first holds within each group.
    images.sort_by_key(|i| !catalog.is_favorite(&i.path));

    let images: Vec<ImageOut> = images
        .into_iter()
        .map(|i| {
            let (root, folder) = match library::folder_in(&i.path, &settings.folders) {
                Some((r, f)) => (Some(r), f),
                None => (None, String::new()),
            };
            ImageOut {
                root,
                folder,
                fit: catalog.fit(&i.path).map_or("", |f| f.as_str()),
                favorite: catalog.is_favorite(&i.path),
                tags: catalog.tags(&i.path).to_vec(),
                thumb: want_thumbs
                    .then(|| thumbs::ensure_thumb(&store.dirs().cache, &i.path, settings.thumb_width).ok())
                    .flatten(),
                width: i.width,
                height: i.height,
                path: i.path,
                name: i.name,
                modified: i.modified,
            }
        })
        .collect();
    let out = ListOutput {
        backend: settings.wallpaper_backend.as_str(),
        monitors: monitors
            .into_iter()
            .map(|m| MonitorOut {
                current: state.current.get(&m.name).cloned(),
                name: m.name,
                width: m.width,
                height: m.height,
                focused: m.focused,
            })
            .collect(),
        images,
        all_tags,
        roots: settings
            .folders
            .iter()
            .map(|p| RootOut {
                name: p
                    .file_name()
                    .map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned()),
                path: p.clone(),
            })
            .collect(),
    };

    // Written by hand, not with `println!`: that panics when the reader
    // stops early (`wallpicker list | head`), which should just stop us.
    let mut stdout = std::io::stdout().lock();
    let written = if json {
        match serde_json::to_string(&out) {
            Ok(s) => writeln!(stdout, "{s}"),
            Err(e) => return fail(&e),
        }
    } else {
        out.monitors
            .iter()
            .try_for_each(|m| {
                let current = m.current.as_deref().map(Path::display);
                writeln!(stdout, "{}: {}", m.name, current.map_or("-".to_string(), |c| c.to_string()))
            })
            .and_then(|()| {
                out.images.iter().try_for_each(|i| {
                    let star = if i.favorite { "*" } else { " " };
                    writeln!(stdout, "{star} {}", i.path.display())
                })
            })
    };
    match written {
        Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => fail(&e),
        _ => 0,
    }
}

/// `wallpicker apply <path> [--monitor NAME]...`. No `--monitor`: every
/// connected monitor.
pub fn apply(args: &[String]) -> i32 {
    let mut path = None;
    let mut monitors = Vec::new();
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--monitor" | "-m" => match rest.next() {
                Some(m) => monitors.push(m.clone()),
                None => return usage("--monitor needs a monitor name"),
            },
            flag if flag.starts_with('-') => return usage(&format!("unknown option {flag}")),
            p if path.is_none() => path = Some(PathBuf::from(p)),
            _ => return usage("only one image at a time"),
        }
    }
    let Some(path) = path else {
        return usage("which image?");
    };
    // Relative paths are relative to where the command runs.
    let path = std::path::absolute(&path).unwrap_or(path);

    let store = Store::new(Dirs::from_env());
    let settings = match store.load_settings() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    if monitors.is_empty() {
        match hyprpaper::list_monitors(&SystemRunner) {
            Ok(ms) => monitors = ms.into_iter().map(|m| m.name).collect(),
            Err(e) => return fail(&e),
        }
    }
    match hyprpaper::apply_and_remember_with(
        &SystemRunner,
        &store,
        settings.wallpaper_backend,
        &monitors,
        &path,
    ) {
        Ok(report) => {
            for m in &report.applied {
                println!("applied to {m}");
            }
            for (m, why) in &report.failed {
                eprintln!("{m}: {why}");
            }
            i32::from(!report.failed.is_empty())
        }
        Err(e) => fail(&e),
    }
}


/// `wallpicker previous [--monitor NAME]...`: back to what each monitor
/// showed before (default: every connected monitor).
pub fn previous(args: &[String]) -> i32 {
    let mut monitors = Vec::new();
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--monitor" | "-m" => match rest.next() {
                Some(m) => monitors.push(m.clone()),
                None => return fail(&"--monitor needs a monitor name"),
            },
            other => return fail(&format!("previous: unknown option {other}")),
        }
    }
    let store = Store::new(Dirs::from_env());
    let settings = match store.load_settings() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    if monitors.is_empty() {
        match hyprpaper::list_monitors(&SystemRunner) {
            Ok(ms) => monitors = ms.into_iter().map(|m| m.name).collect(),
            Err(e) => return fail(&e),
        }
    }
    match hyprpaper::apply_previous(&SystemRunner, &store, settings.wallpaper_backend, &monitors) {
        Ok(report) => {
            let state = store.load_state().unwrap_or_default();
            for m in &report.applied {
                println!("{m}: back to {}", state.current.get(m).map_or(String::new(), |p| p.display().to_string()));
            }
            for (m, why) in &report.failed {
                eprintln!("{m}: {why}");
            }
            // Nothing went back anywhere: a failure; some did: fine.
            i32::from(report.applied.is_empty())
        }
        Err(e) => fail(&e),
    }
}

/// `wallpicker theme [--json]`: the palette from the current wallpaper.
/// `wallpicker noctalia-colors on|off|status`: Noctalia's colors follow the
/// wallpaper, or go back to its earlier scheme.
pub fn theme(args: &[String]) -> i32 {
    let store = Store::new(Dirs::from_env());
    match crate::theme::palette(&store) {
        Ok(Some(p)) if args.iter().any(|a| a == "--json") => print_json(&p),
        Ok(Some(p)) => {
            println!("{} ({})", p.image.display(), p.scheme);
            for k in ["primary", "secondary", "tertiary", "surface", "on_surface"] {
                println!("  {k:<10} dark {}  light {}", p.dark.get(k).map_or("-", String::as_str), p.light.get(k).map_or("-", String::as_str));
            }
            0
        }
        Ok(None) => fail(&"no palette: turned off, nothing applied yet, or no Noctalia to make one"),
        Err(e) => fail(&e),
    }
}

pub fn noctalia_colors(args: &[String]) -> i32 {
    let store = Store::new(Dirs::from_env());
    let result = match args.first().map(String::as_str) {
        Some("on") => crate::theme::set_noctalia_follow(&store, true),
        Some("off") => crate::theme::set_noctalia_follow(&store, false),
        Some("status") | None => Ok(crate::theme::noctalia_colors()),
        Some(other) => return fail(&format!("noctalia-colors: {other}? use on, off or status")),
    };
    match result {
        Ok(s) => {
            println!(
                "{}",
                if !s.available {
                    "Noctalia isn't running".to_string()
                } else {
                    format!("{} ({})", if s.following { "follows the wallpaper" } else { "its own scheme" }, s.scheme.unwrap_or_default())
                }
            );
            0
        }
        Err(e) => fail(&e),
    }
}

/// A value and its label, for the plugin's selects.
#[derive(Serialize)]
struct Choice {
    value: &'static str,
    label: &'static str,
}

fn choices<'a>(pairs: impl IntoIterator<Item = &'a (&'static str, &'static str)>) -> Vec<Choice> {
    pairs.into_iter().map(|&(value, label)| Choice { value, label }).collect()
}

/// Print `value` as one line of JSON; a closed pipe isn't an error.
fn print_json(value: &impl Serialize) -> i32 {
    match serde_json::to_string(value) {
        Ok(json) => match writeln!(std::io::stdout().lock(), "{json}") {
            Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => fail(&e),
            _ => 0,
        },
        Err(e) => fail(&e),
    }
}

fn usage_for(command: &str, why: &str, usage: &str) -> i32 {
    eprintln!("wallpicker {command}: {why}\n{usage}");
    2
}

fn fail(e: &dyn std::fmt::Display) -> i32 {
    eprintln!("wallpicker: {e}");
    1
}

fn usage(why: &str) -> i32 {
    eprintln!("wallpicker apply: {why}\nusage: wallpicker apply <image> [--monitor NAME]...");
    2
}
