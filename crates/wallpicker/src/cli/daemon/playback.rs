//! Animated wallpaper policy. The renderer can always play prepared animations
//! (lazily: nothing is allocated until the first one); the `animation` settings
//! decide whether a request does. Off, an animated wallpaper is its first frame.

use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::time::Duration;

use serde_json::Value;

use wallpicker_core::media::Limits;
use wallpicker_core::render::{Batch, Intent, Preparation};
use wallpicker_core::settings::{Animation, Transition};
use wallpicker_core::wallpapers::{Backend, Fits};

use super::Drawing;
use crate::decoding::{self, Budget};
use crate::preparation::Preparer;
use crate::renderer::PlaybackOptions;

const MIB: u64 = 1024 * 1024;

/// Provisional until measured on the desktop (TODO.md): four decoders, for
/// three outputs plus a replacement, and as many sources on the GPU.
pub(super) fn renderer_options(animation: Animation) -> PlaybackOptions {
    PlaybackOptions {
        capacity: NonZeroUsize::new(4).unwrap(),
        // Four 4K sources are 126 MiB; a replacement briefly holds both.
        texture_bytes: 256 * MIB,
        fallback_bytes: 128 * MIB,
        interval: animation.interval(),
        configure_timeout: Duration::from_secs(3),
    }
}

/// One admission pool for the daemon's lifetime: a playing source keeps its
/// reservation. Each worker is charged its address-space cap and three
/// frames (an admission bound). The cap grows with the picture: measured
/// peaks are about 29 MiB at 1080p and 111 MiB at 4K (GIF), so 64 MiB plus
/// four frames: 97 MiB at 1080p, 196 MiB at 4K.
pub(super) fn preparer() -> Preparer {
    Preparer {
        // The running build, even after a rebuild replaced the file on disk.
        executable: "/proc/self/exe".into(),
        budget: Budget::new(1280 * MIB, 4),
        options: decoding::Options {
            address_space_bytes: 64 * MIB,
            frame_allowance: 4,
            frame_timeout: Duration::from_secs(5),
        },
        limits: Limits::default(),
    }
}

/// Follow the `animation` settings, however they're edited: pause, resume and
/// the frame-rate cap right away. With playback on, an animated wallpaper
/// shown as a still starts playing: the renderer started again, a monitor was
/// connected (it shows its saved wallpaper as a still), or playback was just
/// turned on. Turned off, what plays becomes its first frame.
pub(super) fn follow(drawing: &Drawing) {
    let settings = || drawing.store.load_settings().map(|s| s.animation);
    let mut last = settings().unwrap_or_default();
    // The stills playback was tried for, so one that can't play (it fell
    // back to a still) isn't tried again every second.
    let mut tried: BTreeSet<Still> = BTreeSet::new();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        // A settings file being written can't be read for a moment.
        let Ok(now) = settings() else { continue };
        if now.paused != last.paused {
            drawing.if_running(|r| r.pause(now.paused));
        }
        if now.max_fps != last.max_fps {
            drawing.if_running(|r| r.set_interval(now.interval()));
        }
        if drawing.restarted.swap(false, Ordering::SeqCst) || now.enabled != last.enabled {
            tried.clear();
        }
        let shown = animated(drawing);
        if now.enabled {
            let stills: BTreeSet<Still> = shown
                .into_iter()
                .filter_map(|(still, how)| (how == Shown::Still).then_some(still))
                .collect();
            tried.retain(|still| stills.contains(still));
            let new: Vec<Still> = stills.difference(&tried).cloned().collect();
            if !new.is_empty() {
                show(drawing, &new);
                tried.extend(new);
            }
        } else if last.enabled {
            let playing: Vec<Still> = shown
                .into_iter()
                .filter_map(|(still, how)| (how == Shown::Playing).then_some(still))
                .collect();
            show(drawing, &playing);
        }
        last = now;
    }
}

pub(super) fn enabled(drawing: &Drawing) -> bool {
    drawing.store.load_settings().is_ok_and(|s| s.animation.enabled)
}

#[derive(Debug, PartialEq)]
enum Shown {
    /// Its first frame, though it could play.
    Still,
    /// Playing, paused, being prepared or replaced, or frozen by an error.
    Playing,
}

/// An animated wallpaper on a monitor, while that monitor stays connected
/// (its output's global is new when it's connected again).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Still {
    monitor: String,
    global: u64,
    source: PathBuf,
}

/// Configured monitors showing an animated wallpaper, and how.
fn animated(drawing: &Drawing) -> Vec<(Still, Shown)> {
    let status = drawing.if_running(|r| r.status()).unwrap_or_default();
    animated_in(&status)
}

fn animated_in(status: &Value) -> Vec<(Still, Shown)> {
    let mut shown = Vec::new();
    for screen in status["screens"].as_array().into_iter().flatten() {
        let (Some(monitor), Some(global), Some(source), Some(state)) = (
            screen["monitor"].as_str(),
            screen["global"].as_u64(),
            screen["source"].as_str(),
            screen["playback"]["state"].as_str(),
        ) else {
            continue;
        };
        let how = match state {
            _ if screen["size"].is_null() => continue,
            "still" => Shown::Still,
            // Without hardware EGL nothing plays.
            "unavailable" => continue,
            _ => Shown::Playing,
        };
        let still = Still {
            monitor: monitor.into(),
            global,
            source: source.into(),
        };
        shown.push((still, how));
    }
    shown
}

/// Animated wallpapers on these monitors again, through the normal request
/// path: its policy decides whether they play.
fn show(drawing: &Drawing, shown: &[Still]) {
    let fits = Fits::load(&drawing.store);
    let backend = Backend::from_settings(&drawing.store).restoring();
    let mut preparation = Preparation::default();
    let mut outputs = Vec::new();
    for Still { monitor, source: path, .. } in shown {
        if !preparation.is_animated(path).unwrap_or(false) {
            continue;
        }
        match backend.prepare(&mut preparation, monitor, path, fits.of(monitor, path)) {
            Ok(output) => outputs.push(output),
            Err(e) => eprintln!("wallpicker: {monitor}: {e}"),
        }
    }
    if outputs.is_empty() {
        return;
    }
    let instant = Transition {
        kind: "none".into(),
        duration_ms: 0,
    };
    match super::render::render_batch(Batch::new(Intent::Restore, instant, outputs), drawing) {
        Ok(report) => {
            for (monitor, error) in report.failed {
                eprintln!("wallpicker: animated wallpaper on {monitor}: {error}");
            }
        }
        Err(e) => eprintln!("wallpicker: animated wallpapers: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn animated_wallpapers_and_how_they_show() {
        let screen = |n: u64, size: Value, playback: Value| {
            json!({ "monitor": format!("DP-{n}"), "global": 40 + n, "size": size,
                "source": format!("/w/{n}.gif"), "playback": playback })
        };
        let status = json!({ "screens": [
            screen(1, json!([1920, 1080]), json!({ "state": "still" })),
            screen(2, json!([1920, 1080]), json!({ "state": "playing" })),
            screen(3, json!([1920, 1080]), json!({ "state": "paused" })),
            // Not configured yet, no hardware EGL, a static wallpaper.
            screen(4, Value::Null, json!({ "state": "still" })),
            screen(5, json!([1920, 1080]), json!({ "state": "unavailable" })),
            screen(6, json!([1920, 1080]), Value::Null),
        ]});
        let still = |n: u64| Still {
            monitor: format!("DP-{n}"),
            global: 40 + n,
            source: format!("/w/{n}.gif").into(),
        };
        assert_eq!(
            animated_in(&status),
            [(still(1), Shown::Still), (still(2), Shown::Playing), (still(3), Shown::Playing)]
        );
        assert!(animated_in(&Value::Null).is_empty());
    }
}
