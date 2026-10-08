//! `wallpicker daemon`: what keeps running without the window, for an
//! autostart. At start it puts the saved wallpapers back through the chosen
//! backend (`--restore`: awww's daemon is started if needed; hyprpaper is
//! waited for; Noctalia restores its own). Then the wallpaper per workspace
//! (Hyprland's events), the
//! slideshow's timer, and Noctalia's colors following the wallpaper (when
//! they're wallpicker's palette: noctalia_colors.rs). One at a time; a second says so and leaves. The
//! workspace listener also waits while another program has it, and the
//! slideshow's ticks take turns with the Noctalia plugin's.

use std::time::Duration;

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::workspaces;

use super::{slideshow, usage_for};

const USAGE: &str = "usage: wallpicker daemon   follow workspaces and keep the slideshow on time (for an autostart)";

pub fn daemon(args: &[String]) -> i32 {
    if !args.is_empty() {
        return usage_for("daemon", "no arguments", USAGE);
    }
    let Some(_held) = claim() else {
        println!("wallpicker daemon: already running");
        return 0;
    };
    let store = Store::new(Dirs::from_env());
    if let Err(e) = store.upgrade_catalog() {
        eprintln!("wallpicker: could not update library.json's format: {e}");
    }
    // hyprpaper.conf sources this file: make sure it exists (and matches).
    if let Err(e) = store.write_hyprpaper_conf() {
        eprintln!("wallpicker: could not write the hyprpaper config: {e}");
    }
    // Retries for a while: at login the backend may still be starting.
    std::thread::spawn(|| crate::restore_cli());
    let listener = store.clone();
    std::thread::spawn(move || workspaces::listen(&SystemRunner, &listener, &|| true, &|| {}));
    let colors = store.clone();
    std::thread::spawn(move || follow_colors(&colors));
    loop {
        // Errors are printed by the tick; keep going.
        if let Err(e) = slideshow::tick_with(&store, false) {
            eprintln!("wallpicker: slideshow: {e}");
        }
        std::thread::sleep(Duration::from_secs(30));
    }
}

/// Noctalia's palette made again when what's shown (or the settings: the
/// monitor the colors come from) changes; once at start too.
fn follow_colors(store: &Store) {
    let files = [store.dirs().state.join("state.json"), store.dirs().config.join("config.json")];
    let stamp = |p: &std::path::PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let mut last: Vec<_> = vec![None; files.len()];
    loop {
        let now: Vec<_> = files.iter().map(stamp).collect();
        if now != last {
            last = now;
            if let Err(e) = crate::noctalia_colors::refresh(store) {
                eprintln!("wallpicker: Noctalia's colors: {e}");
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

/// This session's daemon: an abstract socket named for the session, held
/// for as long as the process lives (gone with it, even if it's killed).
fn claim() -> Option<std::os::unix::net::UnixListener> {
    use std::os::linux::net::SocketAddrExt;
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap_or_default();
    let addr = std::os::unix::net::SocketAddr::from_abstract_name(format!("wallpicker-daemon:{runtime}:{sig}")).ok()?;
    std::os::unix::net::UnixListener::bind_addr(&addr).ok()
}
