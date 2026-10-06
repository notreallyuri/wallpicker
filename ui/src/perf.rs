//! Timing marks for measuring how long the window takes to open
//! (`performance.mark`: free, and nothing reads them in normal use; the
//! automation build's open-timing script collects them). Names start with
//! "wp:".

use std::sync::atomic::{AtomicBool, Ordering};

pub(crate) fn mark(name: &str) {
    if let Some(p) = web_sys::window().and_then(|w| w.performance()) {
        let _ = p.mark(&format!("wp:{name}"));
    }
}

/// Once per page load: `name`, then `name-painted` at the next frame.
pub(crate) fn once_and_painted(name: &'static str) {
    static SEEN: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    let Ok(mut seen) = SEEN.lock() else { return };
    if seen.contains(&name) {
        return;
    }
    seen.push(name);
    mark(name);
    leptos::prelude::request_animation_frame(move || mark(&format!("{name}-painted")));
}

/// A thumbnail drawn in the grid: the first one, and each (the last of
/// these is when the ones on screen are all in).
pub(crate) fn thumbnail() {
    static FIRST: AtomicBool = AtomicBool::new(false);
    if !FIRST.swap(true, Ordering::Relaxed) {
        mark("first-thumbnail");
    }
    mark("thumbnail");
}
