//! Settings → Backup: export and import (to and from a file the user picked),
//! and the daily snapshots: where they go, which there are, restoring one.

use std::path::PathBuf;

use wallpicker_core::backup::ImportReport;
use wallpicker_core::catalog::Catalog;
use wallpicker_core::store::Snapshot;

use crate::backup::{self, Exported};
use crate::{AppState, CmdResult, blocking, err, rt};

/// Where an export is suggested (the dialog starts there).
pub fn default_export_path() -> PathBuf {
    backup::default_export_path()
}

pub async fn export(state: &AppState, path: String) -> CmdResult<Exported> {
    let store = state.store.clone();
    rt::spawn_blocking(move || backup::export(&store, Some(&path))).await.map_err(err)?
}

/// Merged into the library, or replacing it.
pub async fn import(state: &AppState, path: String, replace: bool) -> CmdResult<ImportReport> {
    let store = state.store.clone();
    let lock = state.catalog_lock.clone();
    rt::spawn_blocking(move || {
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        backup::import(&store, &path, replace)
    })
    .await
    .map_err(err)?
}

pub async fn snapshots(state: &AppState) -> CmdResult<Vec<Snapshot>> {
    let store = state.store.clone();
    blocking(move || store.snapshots()).await
}

/// Where the snapshots go now.
pub async fn snapshot_dir(state: &AppState) -> CmdResult<PathBuf> {
    let store = state.store.clone();
    blocking(move || Ok(store.snapshots_dir())).await
}

/// Move the snapshots to `dir` (copying them), or back to the default
/// folder (None); returns where they are now.
pub async fn set_snapshot_dir(state: &AppState, dir: Option<PathBuf>) -> CmdResult<PathBuf> {
    let store = state.store.clone();
    blocking(move || store.set_snapshots_dir(dir)).await
}

/// The library as it was in snapshot `id` (what's there now is kept as a
/// snapshot too).
pub async fn restore(state: &AppState, id: String) -> CmdResult<Catalog> {
    let store = state.store.clone();
    let lock = state.catalog_lock.clone();
    blocking(move || {
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        store.restore_snapshot(&id)
    })
    .await
}
