//! What the monitors show: listing them, applying, Previous.

use std::path::PathBuf;

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper::{self, ApplyReport};
use wallpicker_core::monitors::Monitor;

use crate::{AppState, CmdResult, blocking};

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
