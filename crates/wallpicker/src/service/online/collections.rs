//! Your Wallhaven collections and their pages.

use wallpicker_core::online::{Page, Provider};
use wallpicker_core::wallhaven;

use super::{get_text, wallhaven_key};
use crate::{AppState, CmdResult, blocking, err};

/// Your Wallhaven collections: they need the API key (private ones too).
pub async fn wallhaven_collections(state: &AppState) -> CmdResult<Vec<wallhaven::Collection>> {
    let Some(key) = wallhaven_key(state).await? else {
        return Err("Your collections need a Wallhaven API key (Settings → Wallhaven).".into());
    };
    let req = state.http.get(wallhaven::API_COLLECTIONS_URL).header("X-API-Key", key);
    let text = get_text(Provider::Wallhaven, req).await?;
    wallhaven::parse_collections(&text).map_err(err)
}

/// A page of one of your collections. Opening one needs your username too:
/// the API doesn't tell whose key it is.
pub async fn wallhaven_collection_page(state: &AppState, id: u64, page: u32) -> CmdResult<Page> {
    let store = state.store.clone();
    let username = blocking(move || Ok(store.load_settings()?.wallhaven_username)).await?;
    let Some(username) = username else {
        return Err("Add your Wallhaven username in Settings → Wallhaven to open your collections.".into());
    };
    let url = wallhaven::collection_url(&username, id, page).ok_or("That Wallhaven username isn't valid.")?;
    let mut req = state.http.get(url);
    if let Some(key) = wallhaven_key(state).await? {
        req = req.header("X-API-Key", key);
    }
    let text = get_text(Provider::Wallhaven, req).await?;
    Ok(Page::from(wallhaven::parse_search_response(&text).map_err(err)?))
}
