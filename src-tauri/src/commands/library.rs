//! The local library: scanning, thumbnails, favorites and tags, colors and
//! duplicates.

use std::path::PathBuf;
use wallpicker_core::catalog::Catalog;
use wallpicker_core::duplicates::DuplicateGroup;
use wallpicker_core::library::LocalImage;

use crate::service::library::{self as shared, ResolveReport, color_profiles, duplicates, resolve_duplicates_with};
use crate::service::{self};
use crate::{AppState, CmdResult, blocking};

#[tauri::command]
pub(crate) async fn scan_local(state: tauri::State<'_, AppState>) -> CmdResult<Vec<LocalImage>> {
    service::library::scan(&state).await
}

/// Returns the path of a cached thumbnail for `path`.
#[tauri::command]
pub(crate) async fn thumbnail(state: tauri::State<'_, AppState>, path: String) -> CmdResult<String> {
    service::library::thumbnail(&state, path).await
}

#[tauri::command]
pub(crate) async fn get_catalog(state: tauri::State<'_, AppState>) -> CmdResult<Catalog> {
    service::library::catalog(&state).await
}

#[tauri::command]
pub(crate) async fn set_favorite(state: tauri::State<'_, AppState>, path: String, favorite: bool) -> CmdResult<Catalog> {
    service::library::set_favorite(&state, path, favorite).await
}



#[tauri::command]
pub(crate) async fn image_colors(
    state: tauri::State<'_, AppState>,
) -> CmdResult<std::collections::BTreeMap<PathBuf, wallpicker_core::colors::ColorProfile>> {
    let store = state.store.clone();
    blocking(move || color_profiles(&store)).await
}

#[tauri::command]
pub(crate) async fn find_duplicates(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<DuplicateGroup>> {
    duplicates(&state).await
}



#[tauri::command]
pub(crate) async fn resolve_duplicates(
    state: tauri::State<'_, AppState>,
    keep: String,
    others: Vec<String>,
) -> CmdResult<ResolveReport> {
    resolve_duplicates_with(&state, &keep, &others).await
}

#[tauri::command]
pub(crate) async fn add_tag(state: tauri::State<'_, AppState>, path: String, tag: String) -> CmdResult<Catalog> {
    shared::add_tag(&state, path, tag).await
}

#[tauri::command]
pub(crate) async fn remove_tag(state: tauri::State<'_, AppState>, path: String, tag: String) -> CmdResult<Catalog> {
    shared::remove_tag(&state, path, tag).await
}
