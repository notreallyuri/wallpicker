//! Native wallpaper daemon: renderer, workspace listener, slideshow and Noctalia colors.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::time::Duration;

use serde_json::{Value, json};
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::settings::FitMode;
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::wallpapers;
use wallpicker_core::workspaces;

use super::{slideshow, usage_for};
use crate::renderer::{Renderer, Transition};

mod playback;
mod render;

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
    let (settings_tx, settings_rx) = mpsc::channel();
    let drawing = Arc::new(Drawing::new(store.clone(), settings_tx));
    if let Err(e) = drawing.start() {
        eprintln!("wallpicker: can't draw the wallpapers: {e}");
    }
    let following = drawing.clone();
    std::thread::spawn(move || playback::follow(&following, &settings_rx));
    let answering = drawing.clone();
    std::thread::spawn(move || answer_all(&socket, &answering));
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
    let files = [
        store.dirs().state.join("state.json"),
        store.dirs().config.join("config.json"),
    ];
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

/// The native wallpaper renderer.
struct Drawing {
    store: Store,
    renderer: Mutex<Option<Renderer>>,
    /// Decoder admission for animated wallpapers, shared by every request.
    preparer: crate::preparation::Preparer,
    /// The renderer started (again): its saved animations start as stills.
    restarted: AtomicBool,
    /// Held from deciding a change until it's queued (see [`Drawing::change`]).
    changing: Mutex<()>,
    /// Wakes [`playback::follow`]: the settings were just saved.
    settings_changed: mpsc::Sender<()>,
}

impl Drawing {
    fn new(store: Store, settings_changed: mpsc::Sender<()>) -> Self {
        Drawing {
            store,
            renderer: Mutex::new(None),
            preparer: playback::preparer(),
            restarted: AtomicBool::new(false),
            changing: Mutex::new(()),
            settings_changed,
        }
    }

    /// One wallpaper change at a time, until it's queued (not drawn: the
    /// renderer takes its messages in order, and status then shows it on its
    /// way). The daemon's own changes read what's shown and queue theirs with
    /// it held, so a request can't come in between and be undone. Taken
    /// before the renderer's lock, never while holding it.
    fn change(&self) -> MutexGuard<'_, ()> {
        self.changing.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// `f` with the renderer, started first if it isn't running (or its
    /// connection went).
    fn with<T>(&self, f: impl FnOnce(&Renderer) -> Result<T, String>) -> Result<T, String> {
        let mut renderer = self.renderer.lock().unwrap_or_else(|e| e.into_inner());
        if !renderer.as_ref().is_some_and(Renderer::running) {
            *renderer = None;
            let store = self.store.clone();
            let application = wallpicker_core::render::application_id();
            let animation = self.store.load_settings().map(|s| s.animation).unwrap_or_default();
            let started = Renderer::start_with_playback(
                Arc::new(move |m: &str| wallpapers::saved_for_application(&store, m, &application)),
                playback::renderer_options(animation),
            )?;
            started.pause(animation.paused)?;
            self.restarted.store(true, Ordering::SeqCst);
            *renderer = Some(started);
        }
        f(renderer.as_ref().expect("just started"))
    }

    /// `f` with the renderer, only if it's running (None otherwise).
    fn if_running<T>(&self, f: impl FnOnce(&Renderer) -> Result<T, String>) -> Option<T> {
        let renderer = self.renderer.lock().unwrap_or_else(|e| e.into_inner());
        f(renderer.as_ref().filter(|r| r.running())?).ok()
    }

    fn start(&self) -> Result<(), String> {
        self.with(|_| Ok(()))
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
        Some("capabilities") => Ok(json!({"render_version": wallpicker_core::render::VERSION,
            "animated_playback": playback::enabled(drawing)})),
        Some("render") => render::render(request, drawing),
        // Answered at once; the follower reads them on its own thread.
        Some("settings_changed") => {
            let _ = drawing.settings_changed.send(());
            Ok(json!({ "ok": true }))
        }
        Some("show") => {
            let monitor = request
                .get("monitor")
                .and_then(Value::as_str)
                .ok_or("no monitor")?
                .to_string();
            let path = PathBuf::from(request.get("path").and_then(Value::as_str).ok_or("no path")?);
            if !path.is_absolute() {
                return Err(format!("not an absolute path: {}", path.display()));
            }
            if wallpicker_core::render::Preparation::default()
                .is_animated(&path)
                .map_err(|e| e.to_string())?
            {
                return Err(wallpicker_core::render::PLAYBACK_UNAVAILABLE.into());
            }
            let fit: Option<FitMode> = serde_json::from_value(request.get("fit").cloned().unwrap_or(Value::Null))
                .map_err(|e| format!("fit: {e}"))?;
            // {kind, duration_ms}, the settings' (none: a cut).
            let transition = request.get("transition").and_then(|t| {
                let kind = t.get("kind").and_then(Value::as_str)?;
                let ms = t.get("duration_ms").and_then(Value::as_u64)?;
                Transition::from_setting(kind, ms.min(10_000) as u32)
            });
            // The wait is outside the lock: other monitors draw meanwhile.
            let answer = {
                let _change = drawing.change();
                drawing.with(|r| r.show(monitor, (path, fit), transition))?
            };
            match answer.recv_timeout(Duration::from_secs(25)) {
                Ok(result) => result.map(|()| json!({ "ok": true })),
                Err(mpsc::RecvTimeoutError::Timeout) => Err("drawing it took too long".into()),
                Err(mpsc::RecvTimeoutError::Disconnected) => Err("the renderer stopped".into()),
            }
        }
        // Whether it draws, and what each monitor shows when it does.
        // Its pid: the picker reads the daemon's (and decoders') GPU and CPU use.
        Some("status") => {
            let mut status = match drawing.renderer.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
                Some(r) if r.running() => r.status()?,
                _ => json!({ "ok": true, "screens": null }),
            };
            status["pid"] = json!(std::process::id());
            Ok(status)
        }
        _ => Err("unknown request".into()),
    }
}
