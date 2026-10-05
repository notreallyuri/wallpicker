//! `wallpicker slideshow`: change the wallpaper every few minutes, as set in
//! Settings → Slideshow (or `wallpicker settings set slideshow.*`).
//!
//! `tick` changes it if it's due and does nothing otherwise, so calling it
//! often is harmless: the Noctalia plugin calls it every minute. `run` does
//! the same in a loop, for setups without Noctalia (a Hyprland autostart).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;

use wallpicker_core::store::{Dirs, Store};

use super::random::{Choice, apply_random};
use super::{fail, print_json, usage_for};

const USAGE: &str = "usage: wallpicker slideshow tick [--now]   change it if due (--now: change it now)\n       \
     wallpicker slideshow run               keep changing it (Ctrl+C to stop)\n       \
     wallpicker slideshow status            JSON: on/off, minutes, seconds until the next change";

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub fn slideshow(args: &[String]) -> i32 {
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["tick"] => tick(false),
        ["tick", "--now"] => tick(true),
        ["run"] => run(),
        ["status"] => status(),
        _ => usage_for("slideshow", "tick, run or status", USAGE),
    }
}

fn tick(force: bool) -> i32 {
    match tick_with(&Store::new(Dirs::from_env()), force) {
        Ok(true) => 0,
        Ok(false) => 1,
        Err(e) => fail(&e),
    }
}

/// Change the wallpaper if the slideshow is on and it's time (or `force`).
/// Ok(false) when some monitor failed. Shared with the app's "Change now".
pub(crate) fn tick_with(store: &Store, force: bool) -> Result<bool, String> {
    let settings = store.load_settings().map_err(|e| e.to_string())?;
    let show = &settings.slideshow;
    if !show.enabled && !force {
        return Ok(true);
    }
    let mut state = store.load_state().unwrap_or_default();
    let now = now();
    if !force && show.due_in(now, state.slideshow_changed) > 0 {
        return Ok(true);
    }
    let choice = Choice {
        favorites: show.favorites,
        tags: show.tags.clone(),
        any: show.any,
        folder: show.folder.clone(),
        monitors: show.monitors.clone(),
        each: show.each,
    };
    let result = apply_random(store, &settings, &choice);
    // Counted as a change even when it failed, so a broken setup (hyprpaper
    // not running, nothing matching) is retried next interval, not every
    // minute.
    state = store.load_state().unwrap_or(state);
    state.slideshow_changed = now;
    store.save_state(&state).map_err(|e| e.to_string())?;
    result
}

fn run() -> i32 {
    eprintln!("wallpicker: slideshow running (Ctrl+C to stop)");
    loop {
        // Errors are already printed; keep going.
        let _ = tick(false);
        std::thread::sleep(Duration::from_secs(30));
    }
}

#[derive(Serialize)]
pub(crate) struct Status {
    enabled: bool,
    minutes: u32,
    /// Seconds until the next change, if it's on.
    next_in: Option<u64>,
}

fn status() -> i32 {
    match status_of(&Store::new(Dirs::from_env())) {
        Ok(s) => print_json(&s),
        Err(e) => fail(&e),
    }
}

/// On or off, how often, and when it changes next. Shared with the app.
pub(crate) fn status_of(store: &Store) -> Result<Status, String> {
    let settings = store.load_settings().map_err(|e| e.to_string())?;
    let last = store.load_state().unwrap_or_default().slideshow_changed;
    let show = &settings.slideshow;
    Ok(Status {
        enabled: show.enabled,
        minutes: show.minutes,
        next_in: show.enabled.then(|| show.due_in(now(), last)),
    })
}
