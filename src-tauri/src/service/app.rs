//! The app's own data: settings, saved state, the palette.

use wallpicker_core::settings::{Settings, State as SavedState};

use crate::{AppState, CmdResult, blocking, err, theme};

pub(crate) async fn settings(state: &AppState) -> CmdResult<Settings> {
    let store = state.store.clone();
    blocking(move || store.load_settings()).await
}

/// What each monitor shows, its history and adjustments.
pub(crate) async fn saved_state(state: &AppState) -> CmdResult<SavedState> {
    let store = state.store.clone();
    blocking(move || store.load_state()).await
}

/// The app's colors from the current wallpaper (None: its own).
pub(crate) async fn palette(state: &AppState) -> CmdResult<Option<theme::Palette>> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || theme::palette(&store)).await.map_err(err)?
}
