//! What the monitors show: applying, Previous, adjusting, workspaces, and the
//! wallpaper files themselves (trash, show in folder).

use std::path::PathBuf;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper::{self, ApplyReport};
use wallpicker_core::monitors::Monitor;
use wallpicker_core::settings::{State as SavedState, WallpaperBackend};

use crate::service::wallpapers::{show_in_folder_blocking, trash_path};
use crate::service::{self};
use crate::{AppState, CmdResult, blocking, err, wallpaper_path};

#[tauri::command]
pub(crate) async fn list_monitors() -> CmdResult<Vec<Monitor>> {
    service::wallpapers::monitors().await
}

/// Apply `path` to the given monitors and remember it for `--restore`.
#[tauri::command]
pub(crate) async fn apply_wallpaper(state: tauri::State<'_, AppState>, monitors: Vec<String>, path: String) -> CmdResult<ApplyReport> {
    service::wallpapers::apply(&state, monitors, path).await
}

/// Back to what each of `monitors` showed before.
#[tauri::command]
pub(crate) async fn previous_wallpaper(state: tauri::State<'_, AppState>, monitors: Vec<String>) -> CmdResult<ApplyReport> {
    service::wallpapers::previous(&state, monitors).await
}

/// Hyprland's workspaces and those with a wallpaper, for Settings.
#[tauri::command]
pub(crate) async fn workspace_rows(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<wallpicker_core::workspaces::WorkspaceRow>> {
    let store = state.store.clone();
    blocking(move || wallpicker_core::workspaces::rows(&SystemRunner, &store)).await
}

/// Set or clear (`path` None) a workspace's wallpaper.
#[tauri::command]
pub(crate) async fn set_workspace_wallpaper(
    state: tauri::State<'_, AppState>,
    workspace: String,
    path: Option<String>,
) -> CmdResult<()> {
    let store = state.store.clone();
    let path = path.map(|p| wallpaper_path(&p)).transpose()?;
    blocking(move || wallpicker_core::workspaces::assign(&store, &workspace, path)).await
}

/// Move a wallpaper to the trash (recoverable from the file manager), and
/// forget its favorite, tags and any monitor's "current" entry for it. Only
/// images inside the wallpaper or download folders are accepted.
#[tauri::command]
pub(crate) async fn trash_wallpaper(
    state: tauri::State<'_, AppState>,
    path: String,
) -> CmdResult<()> {
    trash_path(&state, &path).await
}



#[tauri::command]
pub(crate) async fn show_in_folder(
    state: tauri::State<'_, AppState>,
    path: String,
) -> CmdResult<()> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || show_in_folder_blocking(&store, &path))
        .await
        .map_err(err)?
}

/// Put `text` on the clipboard (see [`service::app::copy_text`]).
#[tauri::command]
pub(crate) async fn copy_text(text: String) -> CmdResult<()> {
    service::app::copy_text(text).await
}

/// Other programs drawing wallpapers besides the chosen backend (they'd
/// fight over the screen), for Settings.
#[tauri::command]
pub(crate) async fn wallpaper_conflicts(backend: WallpaperBackend) -> CmdResult<Vec<String>> {
    blocking(move || Ok(hyprpaper::conflicts(&SystemRunner, backend))).await
}

/// Stop a program [`wallpaper_conflicts`] found (hyprpaper, awww-daemon...).
#[tauri::command]
pub(crate) async fn stop_wallpaper_program(name: String) -> CmdResult<()> {
    blocking(move || Ok(hyprpaper::stop_program(&SystemRunner, &name)?)).await
}

/// The picture the Adjust dialog frames on (see [`service::wallpapers::framing_image`]).
#[tauri::command]
pub(crate) async fn framing_image(
    state: tauri::State<'_, AppState>,
    path: String,
) -> CmdResult<PathBuf> {
    service::wallpapers::framing_image(&state, path).await
}

#[tauri::command]
pub(crate) async fn set_monitor_adjust(
    state: tauri::State<'_, AppState>,
    monitor: String,
    adjust: wallpicker_core::settings::Adjust,
) -> CmdResult<SavedState> {
    service::wallpapers::set_adjust(&state, monitor, adjust).await
}
