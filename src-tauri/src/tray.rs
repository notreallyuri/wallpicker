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

pub fn show(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn toggle(app: &AppHandle) {
    if let Some(w) = main_window(app) {
        if w.is_visible().unwrap_or(false) {
            let _ = w.hide();
        } else {
            show(app);
        }
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

/// Closing hides to the tray when that's on and the tray icon is up.
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
