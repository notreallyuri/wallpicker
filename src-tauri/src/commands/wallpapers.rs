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

/// Put `text` on the Wayland clipboard (`wl-copy`): the webview's own
/// clipboard API isn't reliable in WebKitGTK.
#[tauri::command]
pub(crate) async fn copy_text(text: String) -> CmdResult<()> {
    if text.len() > 64 * 1024 {
        return Err("That's too long to copy.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Write as _;
        let mut child = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Could not run wl-copy (is wl-clipboard installed?): {e}"))?;
        child
            .stdin
            .take()
            .ok_or("wl-copy has no input")?
            .write_all(text.as_bytes())
            .map_err(err)?;
        // wl-copy forks to serve the clipboard; the first process exits soon.
        match child.wait() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("wl-copy failed ({s})")),
            Err(e) => Err(err(e)),
        }
    })
    .await
    .map_err(err)?
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

/// Adjust what `monitor` shows (mirrored...), and show it there now. Every
/// backend: the changed copy is a file of its own. Temporary: another
/// wallpaper on that monitor clears it.
pub(crate) async fn set_adjust(
    state: &AppState,
    monitor: String,
    adjust: wallpicker_core::settings::Adjust,
) -> CmdResult<SavedState> {
    let store = state.store.clone();
    blocking(move || {
        let saved = store.set_adjust(&monitor, adjust)?;
        let report = hyprpaper::reapply(&SystemRunner, &store, &|m, _| m == monitor)?;
        if let Some((_, why)) = report.failed.first() {
            return Err(wallpicker_core::error::Error::Invalid(format!(
                "Could not show it on {monitor}: {why}"
            )));
        }
        let _ = store.write_hyprpaper_conf();
        wallpicker_core::transform::prune(&store.dirs().cache, &saved);
        Ok(saved)
    })
    .await
}

/// The picture the Adjust dialog frames on: sharp enough to zoom into
/// (fitted inside 1600 px, cached), not the whole original.
#[tauri::command]
pub(crate) async fn framing_image(
    state: tauri::State<'_, AppState>,
    path: String,
) -> CmdResult<PathBuf> {
    let path = wallpaper_path(&path)?;
    let cache = state.store.dirs().cache.join("framing");
    blocking(move || wallpicker_core::thumbs::fit_within(&cache, &path, 1600)).await
}

#[tauri::command]
pub(crate) async fn set_monitor_adjust(
    state: tauri::State<'_, AppState>,
    monitor: String,
    adjust: wallpicker_core::settings::Adjust,
) -> CmdResult<SavedState> {
    set_adjust(&state, monitor, adjust).await
}
