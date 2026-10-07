//! The online providers (Wallhaven, Konachan, yande.re): search, details,
//! collections, tag suggestions, previews and downloads (see
//! [`service::online`]).

use std::path::PathBuf;
use wallpicker_core::moebooru::TagSuggestion;
use wallpicker_core::online::{Details, OnlineFilters, OnlineQuery, OnlineWallpaper, Page, Provider};
use wallpicker_core::wallhaven::Collection;

use crate::service::online;
use crate::{AppState, CmdResult};

#[tauri::command]
pub(crate) async fn online_search(state: tauri::State<'_, AppState>, query: OnlineQuery) -> CmdResult<Page> {
    online::search(&state, query).await
}

#[tauri::command]
pub(crate) async fn online_details(state: tauri::State<'_, AppState>, wallpaper: OnlineWallpaper) -> CmdResult<Details> {
    online::details(&state, &wallpaper).await
}

#[tauri::command]
pub(crate) async fn online_preview(
    state: tauri::State<'_, AppState>,
    wallpaper: OnlineWallpaper,
) -> CmdResult<Option<PathBuf>> {
    Ok(online::preview(&state, &wallpaper).await)
}

#[tauri::command]
pub(crate) async fn tag_suggestions(
    state: tauri::State<'_, AppState>,
    provider: Provider,
    text: String,
) -> CmdResult<Vec<TagSuggestion>> {
    online::tag_suggestions(&state, provider, &text).await
}

#[tauri::command]
pub(crate) async fn get_online_filters(state: tauri::State<'_, AppState>) -> CmdResult<OnlineFilters> {
    online::filters(&state).await
}

#[tauri::command]
pub(crate) async fn save_online_filters(state: tauri::State<'_, AppState>, filters: OnlineFilters) -> CmdResult<()> {
    online::save_filters(&state, filters).await
}

#[tauri::command]
pub(crate) fn open_url(url: String) -> CmdResult<()> {
    online::open_url(&url)
}

#[tauri::command]
pub(crate) async fn online_downloaded(state: tauri::State<'_, AppState>) -> CmdResult<Vec<String>> {
    online::downloaded(&state).await
}

#[tauri::command]
pub(crate) async fn online_download(state: tauri::State<'_, AppState>, wallpaper: OnlineWallpaper) -> CmdResult<String> {
    online::download_and_tag(&state, &wallpaper, |_, _| {}).await
}

#[tauri::command]
pub(crate) async fn wallhaven_collections(state: tauri::State<'_, AppState>) -> CmdResult<Vec<Collection>> {
    online::wallhaven_collections(&state).await
}

#[tauri::command]
pub(crate) async fn wallhaven_collection(state: tauri::State<'_, AppState>, id: u64, page: u32) -> CmdResult<Page> {
    online::wallhaven_collection_page(&state, id, page).await
}
