//! `wallpicker daemon`: what keeps running without the window, for an
//! autostart. At start it puts the saved wallpapers back through the chosen
//! backend (`--restore`: awww's daemon is started if needed; hyprpaper is
//! waited for; Noctalia restores its own). Then the wallpaper per workspace
//! (Hyprland's events), the
//! slideshow's timer, and Noctalia's colors following the wallpaper (when
//! they're wallpicker's palette: noctalia_colors.rs). One at a time; a second says so and leaves. The
//! workspace listener also waits while another program has it, and the
//! slideshow's ticks take turns with the Noctalia plugin's.
//!
//! With the wallpicker backend it draws the wallpapers too (renderer.rs):
//! running while that's the backend (each monitor showing the saved state,
//! so no restore), stopped when another is picked. Its socket answers the
//! backend (core's daemon.rs): `show` and `status`, a JSON line each way.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use serde_json::{Value, json};
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::settings::{FitMode, WallpaperBackend};
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::workspaces;

use super::{slideshow, usage_for};
use crate::renderer::Renderer;

const USAGE: &str = "usage: wallpicker daemon   follow workspaces and keep the slideshow on time (for an autostart)";

pub fn daemon(args: &[String]) -> i32 {
    if !args.is_empty() {
        return usage_for("daemon", "no arguments", USAGE);
    }
    let Some(socket) = claim() else {
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
    let drawing = Arc::new(Drawing { store: store.clone(), renderer: Mutex::new(None) });
    if backend_of(&store) == WallpaperBackend::Wallpicker {
        // Each monitor shows its saved wallpaper as the renderer finds it.
        if let Err(e) = drawing.start() {
            eprintln!("wallpicker: can't draw the wallpapers: {e}");
        }
    } else {
        // Retries for a while: at login the backend may still be starting.
        std::thread::spawn(crate::restore_cli);
    }
    let answering = drawing.clone();
    std::thread::spawn(move || answer_all(&socket, &answering));
    let following = drawing.clone();
    std::thread::spawn(move || follow_backend(&following));
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
fn claim() -> Option<UnixListener> {
    UnixListener::bind_addr(&wallpicker_core::daemon::address().ok()?).ok()
}

fn backend_of(store: &Store) -> WallpaperBackend {
    store.load_settings().map(|s| s.wallpaper_backend).unwrap_or_default()
}

/// The renderer, while the backend is wallpicker.
struct Drawing {
    store: Store,
    renderer: Mutex<Option<Renderer>>,
}

impl Drawing {
    /// `f` with the renderer, started first if it isn't running (or its
    /// connection went).
    fn with<T>(&self, f: impl FnOnce(&Renderer) -> Result<T, String>) -> Result<T, String> {
        let mut renderer = self.renderer.lock().unwrap_or_else(|e| e.into_inner());
        if !renderer.as_ref().is_some_and(Renderer::running) {
            *renderer = None;
            let store = self.store.clone();
            *renderer = Some(Renderer::start(Arc::new(move |m: &str| hyprpaper::saved_for(&store, m)))?);
        }
        f(renderer.as_ref().expect("just started"))
    }

    fn start(&self) -> Result<(), String> {
        self.with(|_| Ok(()))
    }

    /// Its surfaces go: what's underneath shows.
    fn stop(&self) {
        self.renderer.lock().unwrap_or_else(|e| e.into_inner()).take();
    }
}

/// The renderer started when the backend becomes wallpicker, stopped when
/// it becomes another (checked when the settings file changes).
fn follow_backend(drawing: &Drawing) {
    let file = drawing.store.dirs().config.join("config.json");
    let stamp = || std::fs::metadata(&file).and_then(|m| m.modified()).ok();
    let mut last = stamp();
    loop {
        std::thread::sleep(Duration::from_secs(2));
        let now = stamp();
        if now == last {
            continue;
        }
        last = now;
        if backend_of(&drawing.store) == WallpaperBackend::Wallpicker {
            if let Err(e) = drawing.start() {
                eprintln!("wallpicker: can't draw the wallpapers: {e}");
            }
        } else {
            drawing.stop();
        }
    }
}

fn answer_all(socket: &UnixListener, drawing: &Arc<Drawing>) {
    for stream in socket.incoming().flatten() {
        let drawing = drawing.clone();
        std::thread::spawn(move || {
            if let Err(e) = answer(stream, &drawing) {
                eprintln!("wallpicker: daemon socket: {e}");
            }
        });
    }
}

/// One request, one answer.
fn answer(stream: UnixStream, drawing: &Drawing) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut line = String::new();
    BufReader::new((&stream).take(64 * 1024)).read_line(&mut line)?;
    let reply = match serde_json::from_str::<Value>(&line) {
        Ok(request) => respond(&request, drawing).unwrap_or_else(|e| json!({ "error": e })),
        Err(e) => json!({ "error": format!("not a request: {e}") }),
    };
    writeln!(&stream, "{reply}")
}

fn respond(request: &Value, drawing: &Drawing) -> Result<Value, String> {
    match request.get("cmd").and_then(Value::as_str) {
        Some("show") => {
            let monitor = request.get("monitor").and_then(Value::as_str).ok_or("no monitor")?.to_string();
            let path = PathBuf::from(request.get("path").and_then(Value::as_str).ok_or("no path")?);
            if !path.is_absolute() {
                return Err(format!("not an absolute path: {}", path.display()));
            }
            let fit: Option<FitMode> = serde_json::from_value(request.get("fit").cloned().unwrap_or(Value::Null))
                .map_err(|e| format!("fit: {e}"))?;
            // The wait is outside the lock: other monitors draw meanwhile.
            let answer = drawing.with(|r| r.show(monitor, (path, fit)))?;
            match answer.recv_timeout(Duration::from_secs(25)) {
                Ok(result) => result.map(|()| json!({ "ok": true })),
                Err(mpsc::RecvTimeoutError::Timeout) => Err("drawing it took too long".into()),
                Err(mpsc::RecvTimeoutError::Disconnected) => Err("the renderer stopped".into()),
            }
        }
        // Whether it draws, and what each monitor shows when it does.
        Some("status") => match drawing.renderer.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            Some(r) if r.running() => r.status(),
            _ => Ok(json!({ "ok": true, "screens": null })),
        },
        _ => Err("unknown request".into()),
    }
}
