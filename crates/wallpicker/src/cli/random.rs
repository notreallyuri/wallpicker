//! `wallpicker random`: apply a random wallpaper from the library, narrowed
//! like the app's filters. Meant for keybinds.

use std::path::{Path, PathBuf};

use wallpicker_core::catalog::TagMatch;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::library;
use wallpicker_core::settings::{Settings, expand_tilde};
use wallpicker_core::store::{Dirs, Store};

use super::{fail, usage_for};

const USAGE: &str = "usage: wallpicker random [--favorites] [--tag TAG]... [--any] [--folder DIR] [--color C] [--monitor NAME]... [--each]\n\
     --favorites  only favorites\n\
     --color C    only wallpapers with this color (a hue, dark, light, warm or muted; see `wallpicker colors`)\n\
     --tag TAG    only wallpapers with this tag (repeat: all of them; with --any: any of them)\n\
     --folder DIR only wallpapers inside DIR\n\
     --monitor    where to apply (repeat; default: every monitor)\n\
     --each       a different wallpaper on each monitor (default: the same one)";

/// A random number below `n` (n > 0). The standard library's hasher keys are
/// random per process and differ for each `RandomState`, which is plenty for
/// picking a wallpaper.
fn random_below(n: usize) -> usize {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    );
    (h.finish() % n as u64) as usize
}

/// Pick from `candidates`, avoiding `avoid` (what's on now) when there's any
/// other choice.
fn pick<'a>(candidates: &'a [PathBuf], avoid: &[&Path]) -> Option<&'a PathBuf> {
    let fresh: Vec<&PathBuf> = candidates.iter().filter(|p| !avoid.contains(&p.as_path())).collect();
    if fresh.is_empty() {
        candidates.get(random_below(candidates.len().max(1)))
    } else {
        Some(fresh[random_below(fresh.len())])
    }
}

/// `wallpicker random [--favorites] [--tag T]... [--any] [--folder DIR] [--monitor M]... [--each]`.
pub fn random(args: &[String]) -> i32 {
    let (mut favorites, mut any, mut each) = (false, false, false);
    let (mut tags, mut monitors) = (Vec::new(), Vec::new());
    let mut folder: Option<PathBuf> = None;
    let mut color: Option<String> = None;
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--favorites" => favorites = true,
            "--any" => any = true,
            "--each" => each = true,
            "--tag" => match rest.next() {
                Some(t) => tags.push(t.clone()),
                None => return usage_for("random", "--tag needs a tag", USAGE),
            },
            "--folder" => match rest.next() {
                Some(d) => {
                    let home = std::env::var_os("HOME").map(PathBuf::from);
                    let d = expand_tilde(Path::new(d), home.as_deref());
                    folder = Some(std::path::absolute(&d).unwrap_or(d));
                }
                None => return usage_for("random", "--folder needs a folder", USAGE),
            },
            "--color" => match rest.next() {
                Some(c) if wallpicker_core::settings::known_color(Some(c.clone())).is_some() => color = Some(c.clone()),
                _ => {
                    return usage_for(
                        "random",
                        "--color needs a color: red orange yellow green teal blue purple pink dark light warm muted",
                        USAGE,
                    );
                }
            },
            "--monitor" | "-m" => match rest.next() {
                Some(m) => monitors.push(m.clone()),
                None => return usage_for("random", "--monitor needs a monitor name", USAGE),
            },
            other => return usage_for("random", &format!("unknown option {other}"), USAGE),
        }
    }

    let store = Store::new(Dirs::from_env());
    let settings = match store.load_settings() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let choice = Choice {
        favorites,
        tags,
        any,
        folder,
        color,
        monitors,
        each,
    };
    match apply_random(&store, &settings, &choice) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => fail(&e),
    }
}

/// What to pick from and where to apply it (`random`'s options, or the
/// slideshow's settings).
pub(super) struct Choice {
    pub favorites: bool,
    pub tags: Vec<String>,
    pub any: bool,
    pub folder: Option<PathBuf>,
    /// A color (see `colors`).
    pub color: Option<String>,
    /// Empty: every connected monitor.
    pub monitors: Vec<String>,
    pub each: bool,
}

/// Apply random wallpapers as `choice` says, printing what went where.
/// Ok(false) when some monitor failed; Err when nothing could be tried.
pub(super) fn apply_random(store: &Store, settings: &Settings, choice: &Choice) -> Result<bool, String> {
    let catalog = store.load_catalog().unwrap_or_default();
    let mode = if choice.any { TagMatch::Any } else { TagMatch::All };
    let candidates: Vec<PathBuf> = library::scan(&settings.folders, settings.recursive)
        .into_iter()
        .map(|i| i.path)
        .filter(|p| !choice.favorites || catalog.is_favorite(p))
        .filter(|p| choice.tags.is_empty() || catalog.matches(p, &choice.tags, mode))
        .filter(|p| choice.folder.as_ref().is_none_or(|f| p.starts_with(f)))
        .collect();
    // By color: read (or computed, then cached) only when asked for.
    let candidates: Vec<PathBuf> = match &choice.color {
        None => candidates,
        Some(c) => {
            let profiles = crate::service::library::color_profiles(store).map_err(|e| e.to_string())?;
            candidates
                .into_iter()
                .filter(|p| {
                    profiles
                        .get(p)
                        .is_some_and(|cp| wallpicker_core::colors::matches(cp, c))
                })
                .collect()
        }
    };
    if candidates.is_empty() {
        return Err("no wallpapers match".into());
    }

    let connected: Vec<String> = wallpicker_core::monitors::list(&SystemRunner)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|m| m.name)
        .collect();
    // Asked-for monitors that aren't connected are skipped.
    let monitors: Vec<String> = if choice.monitors.is_empty() {
        connected
    } else {
        choice.monitors.iter().filter(|m| connected.contains(m)).cloned().collect()
    };
    if monitors.is_empty() {
        return Err("none of those monitors is connected".into());
    }
    let current = store.load_state().unwrap_or_default().current;
    let now_on = |ms: &[String]| -> Vec<&Path> {
        ms.iter().filter_map(|m| current.get(m).map(PathBuf::as_path)).collect()
    };

    // One wallpaper for all, or one per monitor.
    let plan: Vec<(Vec<String>, &PathBuf)> = if choice.each {
        let mut taken: Vec<&Path> = Vec::new();
        monitors
            .iter()
            .filter_map(|m| {
                let mut avoid = now_on(std::slice::from_ref(m));
                avoid.extend(taken.iter().copied());
                let p = pick(&candidates, &avoid)?;
                taken.push(p.as_path());
                Some((vec![m.clone()], p))
            })
            .collect()
    } else {
        pick(&candidates, &now_on(&monitors)).map(|p| (monitors.clone(), p)).into_iter().collect()
    };

    let mut ok = true;
    for (targets, path) in plan {
        match hyprpaper::apply_and_remember_with(&SystemRunner, store, settings.wallpaper_backend, &targets, path) {
            Ok(report) => {
                for m in &report.applied {
                    println!("{m}: {}", path.display());
                }
                for (m, why) in &report.failed {
                    eprintln!("{m}: {why}");
                    ok = false;
                }
            }
            Err(e) => {
                eprintln!("wallpicker: {e}");
                ok = false;
            }
        }
    }
    Ok(ok)
}
