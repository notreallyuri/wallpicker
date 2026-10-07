//! Backups of the library from Settings: export and import through native
//! file dialogs, and the daily snapshots.

use std::path::PathBuf;
use tauri::Manager;
use wallpicker_core::catalog::Catalog;

use crate::backup;
use crate::service;
use crate::{AppState, CmdResult, err};

/// A native file dialog over the window: what was picked, or None.
pub(crate) fn file_dialog(
    app: &tauri::AppHandle,
) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    use tauri_plugin_dialog::DialogExt;
    let mut d = app.dialog().file();
    if let Some(w) = app.get_webview_window("main") {
        d = d.set_parent(&w);
    }
    d
}

pub(crate) fn picked(p: Option<tauri_plugin_dialog::FilePath>) -> Option<PathBuf> {
    p.and_then(|f| f.into_path().ok())
}

/// Export the library to a file picked in a save dialog (None: cancelled).
#[tauri::command]
pub(crate) async fn backup_export_pick(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> CmdResult<Option<backup::Exported>> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let suggested = backup::default_export_path();
        let mut d = file_dialog(&app)
            .set_title("Export the wallpicker library")
            .add_filter("wallpicker backup", &["json"]);
        if let (Some(dir), Some(name)) = (suggested.parent(), suggested.file_name()) {
            d = d.set_directory(dir).set_file_name(name.to_string_lossy());
        }
        let Some(path) = picked(d.blocking_save_file()) else {
            return Ok(None);
        };
        backup::export(&store, Some(&path.to_string_lossy())).map(Some)
    })
    .await
    .map_err(err)?
}

/// Import a backup picked in an open dialog (None: cancelled).
#[tauri::command]
pub(crate) async fn backup_import_pick(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    replace: bool,
) -> CmdResult<Option<wallpicker_core::backup::ImportReport>> {
    let store = state.store.clone();
    let lock = state.catalog_lock.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default();
        let d = file_dialog(&app)
            .set_title("Import a wallpicker backup")
            .add_filter("wallpicker backup", &["json"])
            .set_directory(home);
        let Some(path) = picked(d.blocking_pick_file()) else {
            return Ok(None);
        };
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        backup::import(&store, &path.to_string_lossy(), replace).map(Some)
    })
    .await
    .map_err(err)?
}

/// Where the daily snapshots go now.
#[tauri::command]
pub(crate) async fn snapshot_dir(state: tauri::State<'_, AppState>) -> CmdResult<PathBuf> {
    service::backup::snapshot_dir(&state).await
}

/// Pick another folder for the snapshots (copying them there); None:
/// cancelled. `reset`: back to the default folder, no dialog.
#[tauri::command]
pub(crate) async fn snapshot_dir_pick(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    reset: bool,
) -> CmdResult<Option<PathBuf>> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dir = if reset {
            None
        } else {
            let d = file_dialog(&app)
                .set_title("A folder for the daily snapshots")
                .set_directory(store.snapshots_dir());
            match picked(d.blocking_pick_folder()) {
                Some(p) => Some(p),
                None => return Ok(None),
            }
        };
        store
            .set_snapshots_dir(dir)
            .map(Some)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(err)?
}

#[tauri::command]
pub(crate) async fn backup_snapshots(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<wallpicker_core::store::Snapshot>> {
    service::backup::snapshots(&state).await
}

#[tauri::command]
pub(crate) async fn backup_restore(
    state: tauri::State<'_, AppState>,
    id: String,
) -> CmdResult<Catalog> {
    service::backup::restore(&state, id).await
}
