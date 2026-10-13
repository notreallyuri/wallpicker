//! Animated wallpaper policy. The renderer can always play prepared animations
//! (lazily: nothing is allocated until the first one); the `animation` settings
//! decide whether a request does. Off, an animated wallpaper is its first frame.

use std::num::NonZeroUsize;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

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
/// the frame-rate cap right away, and show the saved animated wallpapers again (playing, or as
/// stills) when playback is turned on or off, or the renderer started again.
pub(super) fn follow(drawing: &Drawing) {
    let settings = || drawing.store.load_settings().map(|s| s.animation);
    let mut last = settings().unwrap_or_default();
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
        let restarted = drawing.restarted.swap(false, Ordering::SeqCst);
        if now.enabled != last.enabled || (restarted && now.enabled) {
            restore(drawing);
        }
        last = now;
    }
}

pub(super) fn enabled(drawing: &Drawing) -> bool {
    drawing.store.load_settings().is_ok_and(|s| s.animation.enabled)
}

/// The saved animated wallpapers, through the normal request path: its policy
/// decides whether they play. Static wallpapers are left as they are.
fn restore(drawing: &Drawing) {
    let Ok(state) = drawing.store.load_state() else { return };
    let fits = Fits::load(&drawing.store);
    let backend = Backend::from_settings(&drawing.store).restoring();
    let mut preparation = Preparation::default();
    let mut outputs = Vec::new();
    for (monitor, path) in &state.current {
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
    // A starting renderer finds its monitors in a moment; one that isn't
    // connected has nothing to show.
    let deadline = Instant::now() + Duration::from_secs(5);
    let present = loop {
        let present = screens(drawing);
        if outputs.iter().all(|o| present.contains(&o.monitor)) || Instant::now() >= deadline {
            break present;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    outputs.retain(|o| present.contains(&o.monitor));
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

/// Monitors the renderer has configured.
fn screens(drawing: &Drawing) -> Vec<String> {
    let status = drawing.if_running(|r| r.status()).unwrap_or_default();
    status["screens"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|s| !s["size"].is_null())
        .filter_map(|s| s["monitor"].as_str().map(String::from))
        .collect()
}
