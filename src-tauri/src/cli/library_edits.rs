//! `wallpicker favorite`, `tag` and `trash`: the app's library edits.

use std::path::PathBuf;

use wallpicker_core::catalog;
use wallpicker_core::store::{Dirs, Store};

use super::{fail, usage_for};
use crate::AppState;

/// Run a library change and report it; `what` is the success line.
fn run_edit(future: impl std::future::Future<Output = Result<(), String>>, what: String) -> i32 {
    match tauri::async_runtime::block_on(future) {
        Ok(()) => {
            println!("{what}");
            0
        }
        Err(e) => fail(&e),
    }
}

/// `wallpicker favorite <path> [--off]`.
pub fn favorite(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker favorite <image> [--off]";
    let off = args.iter().any(|a| a == "--off");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
    if let Some(bad) = args.iter().find(|a| a.starts_with('-') && *a != "--off") {
        return usage_for("favorite", &format!("unknown option {bad}"), usage_text);
    }
    let [path] = paths.as_slice() else {
        return usage_for("favorite", "one image, please", usage_text);
    };
    let path = absolute(path);
    let state = AppState::new(Store::new(Dirs::from_env()));
    let p = path.clone();
    run_edit(
        async move {
            crate::service::library::update_catalog(&state, move |c| c.set_favorite(&p, !off)).await?;
            Ok(())
        },
        format!("{} {}", if off { "unfavorited" } else { "favorited" }, path.display()),
    )
}

/// `wallpicker tag <path> add|remove <tag>`.
pub fn tag(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker tag <image> add|remove <tag>";
    let [path, action, tag @ ..] = args else {
        return usage_for("tag", "need an image, add or remove, and a tag", usage_text);
    };
    // A tag can have spaces; the rest of the arguments are it.
    let tag = tag.join(" ");
    let Some(tag) = catalog::normalize_tag(&tag) else {
        return usage_for(
            "tag",
            &format!("tags are 1 to {} characters, without control characters", catalog::MAX_TAG_LEN),
            usage_text,
        );
    };
    let add = match action.as_str() {
        "add" => true,
        "remove" | "rm" => false,
        other => return usage_for("tag", &format!("{other}: use add or remove"), usage_text),
    };
    let path = absolute(path);
    let state = AppState::new(Store::new(Dirs::from_env()));
    let (p, t) = (path.clone(), tag.clone());
    run_edit(
        async move {
            crate::service::library::update_catalog(&state, move |c| {
                if add {
                    c.add_tag(&p, &t);
                } else {
                    c.remove_tag(&p, &t);
                }
            })
            .await?;
            Ok(())
        },
        format!("{} {tag:?} {} {}", if add { "added" } else { "removed" }, if add { "to" } else { "from" }, path.display()),
    )
}

/// `wallpicker trash <path>`: to the desktop trash, with the app's checks
/// (images inside your wallpaper or download folders only) and cleanup.
pub fn trash(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker trash <image>";
    let [path] = args else {
        return usage_for("trash", "one image, please", usage_text);
    };
    let path = absolute(path);
    let state = AppState::new(Store::new(Dirs::from_env()));
    let p = path.to_string_lossy().into_owned();
    run_edit(
        async move { crate::service::wallpapers::trash_path(&state, &p).await },
        format!("moved {} to the trash", path.display()),
    )
}

/// Relative paths are relative to where the command runs.
fn absolute(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    std::path::absolute(&p).unwrap_or(p)
}

/// `wallpicker adjust <monitor> [--flip h|v|both|none] [--fit F] [--frame ZOOM,X,Y | --no-frame] [--reset]`: adjust
/// what a monitor shows (wallpicker makes the changed copy), until another
/// wallpaper goes there. Without options: what it is now.
pub fn adjust(args: &[String]) -> i32 {
    use wallpicker_core::settings::{Adjust, Flip};
    let usage_text = "usage: wallpicker adjust <monitor> [--flip h|v|both|none] [--fit auto|cover|contain|tile|fill]\n\
                      [--frame ZOOM,X,Y | --no-frame] [--reset]\n\
     --frame  the part shown, cut to the monitor's size: ZOOM in % (100-400), the center X and Y\n\
              in % of the picture (50,50 is the middle). e.g. --frame 200,70,40";
    let Some((monitor, opts)) = args.split_first() else {
        return usage_for("adjust", "which monitor?", usage_text);
    };
    let store = Store::new(Dirs::from_env());
    let mut adjust = match store.load_state() {
        Ok(s) => s.adjust.get(monitor).copied().unwrap_or_default(),
        Err(e) => return fail(&e),
    };
    let mut changed = false;
    let mut rest = opts.iter();
    while let Some(o) = rest.next() {
        match o.as_str() {
            "--reset" => adjust = Adjust::default(),
            "--flip" => {
                adjust.flip = match rest.next().map(String::as_str) {
                    Some("h") => Flip { h: true, v: false },
                    Some("v") => Flip { h: false, v: true },
                    Some("both" | "hv") => Flip { h: true, v: true },
                    Some("none") => Flip::default(),
                    other => return usage_for("adjust", &format!("--flip {}? use h, v, both or none", other.unwrap_or("")), usage_text),
                }
            }
            "--fit" => {
                adjust.fit = match rest.next().map(String::as_str) {
                    Some("auto") => None,
                    Some(m) => match serde_json::from_value(serde_json::Value::String(m.to_string())) {
                        Ok(f) => Some(f),
                        Err(_) => return usage_for("adjust", &format!("--fit {m}? use auto, cover, contain, tile or fill"), usage_text),
                    },
                    None => return usage_for("adjust", "--fit needs a fit", usage_text),
                }
            }
            "--no-frame" => adjust.frame = None,
            "--frame" => {
                let parts: Vec<f64> = rest
                    .next()
                    .map(|v| v.split(',').filter_map(|n| n.trim().parse().ok()).collect())
                    .unwrap_or_default();
                let [zoom, x, y] = parts[..] else {
                    return usage_for("adjust", "--frame needs ZOOM,X,Y (e.g. 200,50,50)", usage_text);
                };
                // Cut to this monitor's resolution.
                let Some(m) = wallpicker_core::hyprpaper::list_monitors(&wallpicker_core::cmd::SystemRunner)
                    .ok()
                    .and_then(|ms| ms.into_iter().find(|m| m.name == *monitor))
                else {
                    return fail(&format!("{monitor} isn't connected"));
                };
                adjust.frame = Some(wallpicker_core::settings::Frame {
                    zoom: zoom.clamp(100.0, 400.0) as u16,
                    x: (x.clamp(0.0, 100.0) * 10.0) as u16,
                    y: (y.clamp(0.0, 100.0) * 10.0) as u16,
                    width: m.width,
                    height: m.height,
                });
            }
            other => return usage_for("adjust", &format!("unknown option {other}"), usage_text),
        }
        changed = true;
    }
    let describe = move |a: Adjust| {
        format!(
            "{monitor}: flip {}, fit {}, frame {}",
            if a.flip.is_none() { "none" } else { a.flip.as_str() },
            a.fit.map_or("auto", |f| f.as_str()),
            a.frame.map_or("none".to_string(), |f| format!(
                "{}% at {},{} for {}×{}",
                f.zoom,
                f64::from(f.x) / 10.0,
                f64::from(f.y) / 10.0,
                f.width,
                f.height
            ))
        )
    };
    if !changed {
        println!("{}", describe(adjust));
        return 0;
    }
    let state = AppState::new(store);
    let m = monitor.clone();
    run_edit(
        async move { crate::commands::wallpapers::set_adjust(&state, m, adjust).await.map(|_| ()) },
        describe(adjust),
    )
}

/// `wallpicker duplicates [--json]`: exact copies and similar pictures, best
/// first in each group. `wallpicker duplicates keep <image> <other>...`:
/// keep one, trash the others (their favorite, tags and fit move to it).
pub fn duplicates(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker duplicates [--json]\n       wallpicker duplicates keep <image> <other>...";
    let state = AppState::new(Store::new(Dirs::from_env()));
    match args {
        [] | [_] if args.iter().all(|a| a == "--json") => {
            let json = !args.is_empty();
            let groups = match tauri::async_runtime::block_on(crate::service::library::duplicates(&state)) {
                Ok(g) => g,
                Err(e) => return fail(&e),
            };
            if json {
                return super::print_json(&groups);
            }
            if groups.is_empty() {
                println!("no duplicates");
            }
            for g in &groups {
                println!("{:?}:", g.likeness);
                for (i, item) in g.items.iter().enumerate() {
                    println!(
                        "  {} {}×{}  {}",
                        if i == 0 { "keep " } else { "     " },
                        item.width,
                        item.height,
                        item.path.display()
                    );
                }
            }
            0
        }
        [action, keep, others @ ..] if action == "keep" && !others.is_empty() => {
            let keep = absolute(keep).to_string_lossy().into_owned();
            let others: Vec<String> = others.iter().map(|o| absolute(o).to_string_lossy().into_owned()).collect();
            match tauri::async_runtime::block_on(crate::service::library::resolve_duplicates_with(&state, &keep, &others)) {
                Ok(r) => {
                    for t in &r.trashed {
                        println!("trashed {t}");
                    }
                    for m in &r.reapplied {
                        println!("{m}: now {keep}");
                    }
                    for (p, why) in &r.failed {
                        eprintln!("{p}: {why}");
                    }
                    i32::from(!r.failed.is_empty())
                }
                Err(e) => fail(&e),
            }
        }
        _ => usage_for("duplicates", "list them, or keep one", usage_text),
    }
}

/// `wallpicker show-in-folder <image>`: the file manager, with it selected
/// when the file manager supports that.
pub fn show_in_folder(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker show-in-folder <image>";
    let [path] = args else {
        return usage_for("show-in-folder", "one image, please", usage_text);
    };
    let path = absolute(path);
    let store = Store::new(Dirs::from_env());
    match crate::service::wallpapers::show_in_folder_blocking(&store, &path.to_string_lossy()) {
        Ok(()) => 0,
        Err(e) => fail(&e),
    }
}

/// `wallpicker colors [COLOR] [--json]`: the library's colors, or the images
/// that are COLOR (a hue, dark, light, warm or muted), strongest first.
pub fn colors(args: &[String]) -> i32 {
    use wallpicker_core::colors::{self, HUES, TONES};
    let store = Store::new(Dirs::from_env());
    let json = args.iter().any(|a| a == "--json");
    let key = args.iter().find(|a| !a.starts_with("--")).map(String::as_str);
    let known: Vec<&str> = HUES.iter().chain(TONES.iter()).map(|(k, _)| *k).collect();
    if let Some(k) = key
        && !known.contains(&k)
    {
        return fail(&format!("colors: {k}? one of {}", known.join(", ")));
    }
    let profiles = match crate::service::library::color_profiles(&store) {
        Ok(p) => p,
        Err(e) => return fail(&e),
    };
    match key {
        None if json => super::print_json(&profiles),
        None => {
            for (path, p) in &profiles {
                let mut tops: Vec<(f32, &str)> =
                    HUES.iter().enumerate().map(|(i, (k, _))| (p.hues[i], *k)).filter(|(_, k)| colors::matches(p, k)).collect();
                tops.sort_by(|a, b| b.0.total_cmp(&a.0));
                let tones: Vec<&str> = TONES.iter().map(|(k, _)| *k).filter(|k| colors::matches(p, k)).collect();
                let hues: Vec<String> = tops.iter().map(|(s, k)| format!("{k} {:.0}%", s * 100.0)).collect();
                println!(
                    "{}  {}{}",
                    path.file_name().map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                    hues.join(", "),
                    if tones.is_empty() { String::new() } else { format!("  [{}]", tones.join(", ")) }
                );
            }
            0
        }
        Some(k) => {
            let mut hits: Vec<(&std::path::PathBuf, f32)> =
                profiles.iter().filter(|(_, p)| colors::matches(p, k)).map(|(path, p)| (path, colors::score(p, k))).collect();
            hits.sort_by(|a, b| b.1.total_cmp(&a.1));
            if json {
                return super::print_json(&hits.iter().map(|(p, _)| p).collect::<Vec<_>>());
            }
            for (path, s) in hits {
                println!("{:>4.0}%  {}", s * 100.0, path.display());
            }
            0
        }
    }
}
