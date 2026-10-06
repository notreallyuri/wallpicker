//! The tray icon, and the window's life around it: closing hides the window
//! to the tray (Settings → General, on by default), running `wallpicker`
//! again brings it back instead of starting a second app, and
//! `wallpicker --toggle` shows or hides it (for a keybind).
//!
//! On Linux a tray icon can't report clicks (only its menu), so the menu's
//! first entry shows or hides the window.

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager, WebviewWindow};

/// Whether the tray icon is up: without it, closing has to really close
/// (there'd be no way back to a hidden window).
static TRAY_UP: AtomicBool = AtomicBool::new(false);

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// The interface's background color, reported by it (`window_background`):
/// a new window starts in it instead of WebKit's white, so opening doesn't
/// flash. Kept in the state folder too, for the first window after login.
static BACKGROUND: std::sync::Mutex<Option<(u8, u8, u8)>> = std::sync::Mutex::new(None);

fn background_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    Some(app.try_state::<crate::AppState>()?.store.dirs().state.join("window-background"))
}

pub fn set_background(app: &AppHandle, rgb: (u8, u8, u8)) {
    if let Ok(mut b) = BACKGROUND.lock() {
        if *b == Some(rgb) {
            return;
        }
        *b = Some(rgb);
    }
    if let Some(f) = background_file(app) {
        let _ = std::fs::write(f, format!("{} {} {}", rgb.0, rgb.1, rgb.2));
    }
}

fn background(app: &AppHandle) -> Option<tauri::window::Color> {
    let known = BACKGROUND.lock().ok().and_then(|b| *b);
    let (r, g, b) = known.or_else(|| {
        let text = std::fs::read_to_string(background_file(app)?).ok()?;
        let v: Vec<u8> = text.split_whitespace().filter_map(|n| n.parse().ok()).collect();
        (v.len() == 3).then(|| (v[0], v[1], v[2]))
    })?;
    Some(tauri::window::Color(r, g, b, 255))
}

/// The window, made from tauri.conf's "main" when there's none: it isn't
/// made at startup (`create: false`), and closing to the tray destroys it,
/// so its WebKit processes go away while it isn't needed.
fn ensure_window(app: &AppHandle) -> Option<WebviewWindow> {
    if let Some(w) = main_window(app) {
        return Some(w);
    }
    let config = app.config().app.windows.iter().find(|w| w.label == "main")?.clone();
    let built = tauri::WebviewWindowBuilder::from_config(app, &config).and_then(|b| {
        match background(app) {
            Some(color) => b.background_color(color),
            None => b,
        }
        .build()
    });
    match built {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("wallpicker: could not open the window: {e}");
            None
        }
    }
}

pub fn show(app: &AppHandle) {
    if let Some(w) = ensure_window(app) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Put the window away: destroyed when it goes to the tray (its memory with
/// it), else only hidden.
fn put_away(app: &AppHandle, w: &WebviewWindow) {
    if hides_on_close(app) {
        let _ = w.destroy();
        give_memory_back();
    } else {
        let _ = w.hide();
    }
}

pub fn toggle(app: &AppHandle) {
    match main_window(app) {
        Some(w) if w.is_visible().unwrap_or(false) => put_away(app, &w),
        _ => show(app),
    }
}

/// A second `wallpicker` started: `--open WHERE` shows the window there,
/// `--toggle` shows or hides it, `--tray` leaves it as it is, anything else
/// shows it. (`main` checked the arguments before handing them over.)
pub fn second_launch(app: &AppHandle, argv: &[String], cwd: &std::path::Path) {
    if let Ok(Some(place)) = crate::open::parse(argv, cwd) {
        crate::open::request(app, place);
    } else if argv.iter().any(|a| a == "--toggle") {
        toggle(app);
    } else if !argv.iter().any(|a| a == "--tray") {
        show(app);
    }
}

/// Once the window has gone (its teardown takes a moment), hand freed memory
/// back to the system: glibc keeps it for reuse otherwise, and the window
/// leaves a lot behind (image buffers). The tray then costs what it should.
pub fn give_memory_back() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(2));
        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        // SAFETY: malloc_trim only releases free memory; any thread may call it.
        unsafe {
            libc::malloc_trim(0);
        }
    });
}

/// Closing goes to the tray (the app keeps running, the window is destroyed)
/// when that's on and the tray icon is up.
pub fn hides_on_close(app: &AppHandle) -> bool {
    TRAY_UP.load(Ordering::Relaxed)
        && app
            .try_state::<crate::AppState>()
            .is_none_or(|s| s.store.load_settings().map(|s| s.close_to_tray).unwrap_or(true))
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    let toggle_item = MenuItem::with_id(app, "toggle", "Show / hide wallpicker", true, None::<&str>)?;
    let random = MenuItem::with_id(app, "random", "Random wallpaper", true, None::<&str>)?;
    let previous = MenuItem::with_id(app, "previous", "Previous wallpaper", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit wallpicker", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&toggle_item, &random, &previous, &separator, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("wallpicker")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle(app),
            // Like `wallpicker random`: every monitor, avoiding what's on.
            "random" => {
                std::thread::spawn(|| crate::cli::random(&[]));
            }
            // Like `wallpicker previous`: every monitor.
            "previous" => {
                std::thread::spawn(|| crate::cli::previous(&[]));
            }
            "quit" => app.exit(0),
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    TRAY_UP.store(true, Ordering::Relaxed);
    Ok(())
}
