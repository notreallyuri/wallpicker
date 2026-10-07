//! `wallpicker daemon`: what keeps running without the window, for an
//! autostart: the wallpaper per workspace (Hyprland's events) and the
//! slideshow's timer. One at a time; a second says so and leaves. The
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
    // hyprpaper.conf sources this file: make sure it exists (and matches).
    if let Err(e) = store.write_hyprpaper_conf() {
        eprintln!("wallpicker: could not write the hyprpaper config: {e}");
    }
    let listener = store.clone();
    std::thread::spawn(move || workspaces::listen(&SystemRunner, &listener, &|| true, &|| {}));
    loop {
        // Errors are printed by the tick; keep going.
        if let Err(e) = slideshow::tick_with(&store, false) {
            eprintln!("wallpicker: slideshow: {e}");
        }
        std::thread::sleep(Duration::from_secs(30));
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
