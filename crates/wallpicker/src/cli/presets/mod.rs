//! Named wallpaper setups: CRUD and applying through the current backend.

mod reassign;
use super::{fail, print_json, usage_for};
use wallpicker_core::{
    cmd::SystemRunner,
    presets::{self, Preset},
    store::{Dirs, Store},
};

const USAGE: &str = "usage: wallpicker presets list [--json]\n       \
wallpicker presets save NAME [--json]\n       \
wallpicker presets apply ID [--json]\n       \
wallpicker presets update ID [--json]\n       \
wallpicker presets rename ID NAME [--json]\n       \
wallpicker presets delete ID\n       \
wallpicker presets reassign ID OLD=NEW ... [--preview] [--json]\n\n\
Quote names containing spaces. IDs are shown by list. Apply uses the current backend\n\
and transition; slideshow/workspace automation continues normally.";

pub fn presets(args: &[String]) -> i32 {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return 0;
    }
    let store = Store::new(Dirs::from_env());
    let json = args.iter().any(|a| a == "--json");
    let arguments: Vec<_> = args.iter().filter(|a| *a != "--json").map(String::as_str).collect();
    if arguments.first() == Some(&"reassign") {
        return reassign::run(&store, &arguments[1..], json);
    }
    match arguments.as_slice() {
        [] | ["list"] => match store.load_presets() {
            Ok(items) if json => print_json(&items),
            Ok(items) => {
                if items.is_empty() {
                    println!("no wallpaper presets saved");
                }
                for item in items {
                    println!(
                        "{}  {}  ({} monitors, {})",
                        item.id,
                        item.name,
                        item.monitors.len(),
                        if item.span.is_some() { "spanned" } else { "separate" }
                    );
                }
                0
            }
            Err(e) => fail(&e),
        },
        ["save", name] => saved(store.save_current_preset(name), json),
        ["update", id] => saved(store.update_current_preset(id), json),
        ["rename", id, name] => saved(store.rename_preset(id, name), json),
        ["delete", id] => match store.delete_preset(id) {
            Ok(()) => {
                if json {
                    print_json(&serde_json::json!({ "deleted": id }))
                } else {
                    println!("deleted {id}");
                    0
                }
            }
            Err(e) => fail(&e),
        },
        ["apply", id] => match presets::apply(&SystemRunner, &store, id) {
            Ok(report) => {
                let code = i32::from(!report.failed.is_empty() || report.applied.is_empty());
                if json {
                    let printed = print_json(&report);
                    if printed != 0 {
                        return printed;
                    }
                } else {
                    for monitor in &report.applied {
                        println!("applied {id} to {monitor}");
                    }
                    for (monitor, reason) in &report.skipped {
                        println!("skipped {monitor}: {reason}");
                    }
                    for (monitor, reason) in &report.failed {
                        eprintln!("{monitor}: {reason}");
                    }
                }
                code
            }
            Err(e) => fail(&e),
        },
        _ => usage_for("presets", "use list, save, apply, update, rename or delete", USAGE),
    }
}

fn saved(result: wallpicker_core::Result<Preset>, json: bool) -> i32 {
    match result {
        Ok(preset) if json => print_json(&preset),
        Ok(preset) => {
            println!("{}  {}", preset.id, preset.name);
            0
        }
        Err(e) => fail(&e),
    }
}
