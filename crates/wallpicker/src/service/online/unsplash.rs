//! Unsplash's requests (core's `unsplash.rs` builds and reads them): every
//! one carries the Access Key. Downloads are reported, as its guidelines
//! ask.

use wallpicker_core::online::{Page, Provider};
use wallpicker_core::unsplash::{self, UnsplashQuery};

use super::http_error;
use crate::{AppState, CmdResult, blocking, err};

async fn access_key(state: &AppState) -> CmdResult<String> {
    let store = state.store.clone();
    blocking(move || Ok(store.load_settings()?.unsplash_access_key))
        .await?
        .ok_or_else(|| "Unsplash needs an Access Key (free at unsplash.com/developers; Settings → Unsplash).".to_string())
}

fn authorized(req: reqwest::RequestBuilder, key: &str) -> reqwest::RequestBuilder {
    // A header, not a URL parameter, so the key never lands in logs.
    req.header("Authorization", format!("Client-ID {key}")).header("Accept-Version", "v1")
}

pub(super) async fn search(state: &AppState, q: UnsplashQuery) -> CmdResult<Page> {
    let key = access_key(state).await?;
    let (url, params) = q.request();
    let resp = authorized(state.http.get(&url).query(&params), &key).send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(http_error(Provider::Unsplash, resp.status()));
    }
    // Lists (the topic, a photographer) say their size in a header.
    let total = resp.headers().get("x-total").and_then(|v| v.to_str().ok()?.parse().ok());
    let text = resp.text().await.map_err(err)?;
    if url.ends_with("/search/photos") {
        unsplash::parse_search(&text, q.page).map_err(err)
    } else {
        unsplash::parse_list(&text, q.page, total).map_err(err)
    }
}

/// Tell Unsplash `id` was downloaded. Best effort: the file is there either way.
pub(super) async fn report_download(state: &AppState, id: &str) {
    let Ok(key) = access_key(state).await else { return };
    if let Err(e) = authorized(state.http.get(unsplash::download_url(id)), &key).send().await {
        eprintln!("wallpicker: could not report the Unsplash download: {e}");
    }
}
