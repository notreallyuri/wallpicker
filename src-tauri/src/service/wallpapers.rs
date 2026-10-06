//! What the monitors show (listing them, applying, Previous), and the
//! wallpaper files themselves (trash, show in folder).

use std::path::PathBuf;

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper::{self, ApplyReport};
use wallpicker_core::library;
use wallpicker_core::monitors::Monitor;

use wallpicker_core::store::Store;

use crate::service::library::update_catalog;
use crate::{AppState, CmdResult, blocking, err, wallpaper_path};

pub(crate) async fn monitors() -> CmdResult<Vec<Monitor>> {
    blocking(|| hyprpaper::list_monitors(&SystemRunner)).await
}

/// `path` on `monitors`, remembered for `--restore` (and, with workspaces
/// remembering, for the workspace each monitor shows).
pub(crate) async fn apply(state: &AppState, monitors: Vec<String>, path: String) -> CmdResult<ApplyReport> {
    let store = state.store.clone();
    blocking(move || {
        let settings = store.load_settings()?;
        let report = hyprpaper::apply_and_remember_with(&SystemRunner, &store, settings.wallpaper_backend, &monitors, &PathBuf::from(path))?;
        wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
        Ok(report)
    })
    .await
}

/// Back to what each of `monitors` showed before.
pub(crate) async fn previous(state: &AppState, monitors: Vec<String>) -> CmdResult<ApplyReport> {
    let store = state.store.clone();
    blocking(move || {
        let backend = store.load_settings()?.wallpaper_backend;
        let report = hyprpaper::apply_previous(&SystemRunner, &store, backend, &monitors)?;
        wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
        Ok(report)
    })
    .await
}

pub(crate) async fn show_in_folder(state: &AppState, path: String) -> CmdResult<()> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || show_in_folder_blocking(&store, &path)).await.map_err(err)?
}

/// [`trash_wallpaper`] without Tauri (the app and `wallpicker trash`).
pub(crate) async fn trash_path(state: &AppState, path: &str) -> CmdResult<()> {
    let given = wallpaper_path(path)?;
    let store = state.store.clone();
    let real = blocking(move || {
        let settings = store.load_settings()?;
        let mut roots = settings.folders.clone();
        roots.push(settings.download_dir.clone());
        library::trash_target(&given, &roots)
    })
    .await?;

    let file = real.clone();
    tauri::async_runtime::spawn_blocking(move || trash::delete(&file))
        .await
        .map_err(err)?
        .map_err(|e| format!("Could not move it to the trash: {e}"))?;

    // The scan's path and the resolved one can differ (a symlinked folder).
    let (given, resolved) = (PathBuf::from(path), real);
    let forget = (given.clone(), resolved.clone());
    update_catalog(state, move |c| {
        c.forget(&forget.0);
        c.forget(&forget.1);
    })
    .await?;
    let store = state.store.clone();
    blocking(move || {
        store.forget_current(&given)?;
        store.forget_current(&resolved)
    })
    .await
}

/// Show a library image in the file manager: selected, through the standard
/// FileManager1 D-Bus call (Thunar, Nautilus, Dolphin, Nemo...), or else its
/// folder opened with `xdg-open`. Shared by the app and the command line.
pub(crate) fn show_in_folder_blocking(store: &Store, path: &str) -> CmdResult<()> {
    let given = wallpaper_path(path)?;
    let settings = store.load_settings().map_err(err)?;
    let mut roots = settings.folders.clone();
    roots.push(settings.download_dir.clone());
    let real = library::library_file(&given, &roots).map_err(err)?;

    let shown = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--reply-timeout=3000",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{}", library::file_uri(&real)))
        .arg("string:")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if shown {
        return Ok(());
    }
    let dir = real.parent().ok_or("it has no folder")?;
    let mut child = std::process::Command::new("xdg-open")
        .arg(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not open the folder: {e}"))?;
    // Reaped in the background, so it doesn't linger as a zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}
