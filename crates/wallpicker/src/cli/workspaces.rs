//! `wallpicker workspaces`: a wallpaper per Hyprland workspace.

use std::sync::atomic::{AtomicBool, Ordering};

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::workspaces;

use super::{fail, usage_for};

const USAGE: &str = "usage: wallpicker workspaces [list [--json] | on | off | remember on|off |\n\
                     \x20                          set <workspace> <image> | clear <workspace> | run]\n\
     on/off    switch to each workspace's wallpaper as you change workspaces\n\
     remember  a wallpaper you apply becomes the wallpaper of the workspace its monitor shows\n\
     run       follow Hyprland in the foreground (the app does it while it runs)";

pub fn workspaces(args: &[String]) -> i32 {
    let store = Store::new(Dirs::from_env());
    let mut settings = match store.load_settings() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["list"] | ["list", "--json"] => {
            let rows = match workspaces::rows(&SystemRunner, &store) {
                Ok(r) => r,
                Err(e) => return fail(&e),
            };
            if args.contains(&"--json") {
                println!("{}", serde_json::to_string(&rows).unwrap_or_default());
                return 0;
            }
            println!(
                "workspace wallpapers: {}{}",
                if settings.workspaces.enabled { "on" } else { "off" },
                if settings.workspaces.remember { ", remembering what you apply" } else { "" }
            );
            for r in rows {
                println!(
                    "{:>6}  {:<8} {}",
                    r.name,
                    r.monitor.as_deref().unwrap_or(if r.open { "" } else { "(closed)" }),
                    r.wallpaper.map_or("-".to_string(), |p| p.display().to_string())
                );
            }
            0
        }
        ["on"] | ["off"] => {
            settings.workspaces.enabled = args[0] == "on";
            save(&store, &settings)
        }
        ["remember", v @ ("on" | "off")] => {
            settings.workspaces.remember = *v == "on";
            save(&store, &settings)
        }
        ["set", ws, image] => {
            let path = std::path::absolute(image).unwrap_or_else(|_| image.into());
            if !wallpicker_core::library::is_supported_image(&path) || !path.is_file() {
                return fail(&format!("{} isn't an image", path.display()));
            }
            match workspaces::assign(&store, ws, Some(path)) {
                Ok(()) => 0,
                Err(e) => fail(&e),
            }
        }
        ["clear", ws] => match workspaces::assign(&store, ws, None) {
            Ok(()) => 0,
            Err(e) => fail(&e),
        },
        ["run"] => {
            if !settings.workspaces.enabled {
                eprintln!("wallpicker: workspace wallpapers are off (wallpicker workspaces on); waiting");
            }
            static STOP: AtomicBool = AtomicBool::new(false);
            workspaces::listen(&SystemRunner, &store, &|| !STOP.load(Ordering::Relaxed), &|| {});
            0
        }
        _ => usage_for("workspaces", "what to do?", USAGE),
    }
}

fn save(store: &Store, settings: &wallpicker_core::settings::Settings) -> i32 {
    match store.save_settings(settings) {
        Ok(()) => 0,
        Err(e) => fail(&e),
    }
}
