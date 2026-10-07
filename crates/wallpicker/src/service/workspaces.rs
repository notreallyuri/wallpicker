//! A wallpaper per Hyprland workspace, from Settings → Workspaces.

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::workspaces::{self, WorkspaceRow};

use crate::{AppState, CmdResult, blocking, wallpaper_path};

/// Hyprland's workspaces and those with a wallpaper.
pub async fn rows(state: &AppState) -> CmdResult<Vec<WorkspaceRow>> {
    let store = state.store.clone();
    blocking(move || workspaces::rows(&SystemRunner, &store)).await
}

/// Set or clear (`path` None) a workspace's wallpaper.
pub async fn set(state: &AppState, workspace: String, path: Option<String>) -> CmdResult<()> {
    let store = state.store.clone();
    let path = path.map(|p| wallpaper_path(&p)).transpose()?;
    blocking(move || workspaces::assign(&store, &workspace, path)).await
}
