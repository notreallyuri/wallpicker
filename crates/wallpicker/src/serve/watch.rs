//! Changes other programs make while the interface is open (the slideshow,
//! the Noctalia plugin, the command line, the workspace listener): the
//! settings, what the monitors show, the favorites and tags. Their files are
//! checked a few times a second and a change is announced like `serve`'s
//! own: `settings-changed`, `state-changed`, `catalog-changed`. (Its own
//! changes are announced again then; reloading twice is harmless.)

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use serde_json::json;
use wallpicker_core::store::Dirs;

use super::Out;

const EVERY: Duration = Duration::from_millis(700);

pub(super) fn start(dirs: &Dirs, out: Out) {
    let files: Vec<(PathBuf, &'static str)> = vec![
        (dirs.config.join("config.json"), "settings-changed"),
        (dirs.state.join("state.json"), "state-changed"),
        (dirs.data.join("library.json"), "catalog-changed"),
    ];
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
                }
            }
        }
    });
}
