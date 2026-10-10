//! Changes other programs make while the interface is open (the slideshow,
//! the Noctalia plugin, the command line, the workspace listener): the
//! settings, what the monitors show, the favorites and tags. Their files are
//! checked a few times a second and a change is announced like `serve`'s
//! own: `settings-changed`, `state-changed`, `catalog-changed`. (Its own
//! changes are announced again then; reloading twice is harmless.) A new
//! wallpaper or settings also make Noctalia's palette again, when Noctalia
//! uses wallpicker's (noctalia_colors.rs; the daemon does the same).

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde_json::json;
use wallpicker_core::store::{Dirs, Store};

use super::Out;

const EVERY: Duration = Duration::from_millis(700);

pub(super) fn start(dirs: &Dirs, out: Out) {
    let files: Vec<(PathBuf, &'static str)> = vec![
        (dirs.config.join("config.json"), "settings-changed"),
        (dirs.state.join("state.json"), "state-changed"),
        (dirs.data.join("library.json"), "catalog-changed"),
        (dirs.data.join("presets.json"), "presets-changed"),
    ];
    let store = Store::new(dirs.clone());
    std::thread::spawn(move || {
        let stamp = |p: &PathBuf| -> Option<(SystemTime, u64)> {
            let m = std::fs::metadata(p).ok()?;
            Some((m.modified().ok()?, m.len()))
        };
        let mut last: Vec<_> = files.iter().map(|(p, _)| stamp(p)).collect();
        loop {
            std::thread::sleep(EVERY);
            for (i, (path, event)) in files.iter().enumerate() {
                let now = stamp(path);
                if now != last[i] {
                    last[i] = now;
                    out.event(event, json!(null));
                    if matches!(*event, "state-changed" | "settings-changed") {
                        let store = store.clone();
                        std::thread::spawn(move || {
                            if let Err(e) = crate::noctalia_colors::refresh(&store) {
                                eprintln!("wallpicker: Noctalia's colors: {e}");
                            }
                        });
                    }
                }
            }
        }
    });
}
