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
            crate::update_catalog(&state, move |c| c.set_favorite(&p, !off)).await?;
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
            crate::update_catalog(&state, move |c| {
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
        async move { crate::trash_path(&state, &p).await },
        format!("moved {} to the trash", path.display()),
    )
}

/// Relative paths are relative to where the command runs.
fn absolute(path: &str) -> PathBuf {
    let p = PathBuf::from(path);
    std::path::absolute(&p).unwrap_or(p)
}

/// `wallpicker fit <image> auto|cover|contain|tile|fill`: the wallpaper's own
/// fit on any monitor ("auto": the monitor's or the general one). Re-applies
/// it where it's showing.
pub fn fit(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker fit <image> auto|cover|contain|tile|fill";
    let [path, mode] = args else {
        return usage_for("fit", "an image and a fit, please", usage_text);
    };
    let fit = match mode.as_str() {
        "auto" | "default" => None,
        m => match serde_json::from_value(serde_json::Value::String(m.to_string())) {
            Ok(f) => Some(f),
            Err(_) => return usage_for("fit", &format!("{m} isn't a fit"), usage_text),
        },
    };
    let path = absolute(path);
    let state = AppState::new(Store::new(Dirs::from_env()));
    let p = path.to_string_lossy().into_owned();
    run_edit(
        async move { crate::set_fit(&state, &p, fit).await.map(|_| ()) },
        format!("{} now fits as {mode}", path.display()),
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
            let groups = match tauri::async_runtime::block_on(crate::duplicates(&state)) {
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
            match tauri::async_runtime::block_on(crate::resolve_duplicates_with(&state, &keep, &others)) {
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
    match crate::show_in_folder_blocking(&store, &path.to_string_lossy()) {
        Ok(()) => 0,
        Err(e) => fail(&e),
    }
}
