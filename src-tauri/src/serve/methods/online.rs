//! online.*: search, details, preview, tag_suggestions, the saved filters
//! (filters, save_filters), downloaded and download (pushing
//! `download-progress {key, done, total}` while it runs, then announcing
//! `library-changed`), open_url, and Wallhaven's collections (collections,
//! collection).

use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{Value, json as value};
use wallpicker_core::online::OnlineWallpaper;

use super::super::Out;
use super::{json, param};
use crate::service::online;
use crate::{AppState, CmdResult};

/// Progress events at most this often per download (and its end).
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

pub(super) async fn call(state: &AppState, out: &Out, name: &str, params: &Value) -> Option<CmdResult<Value>> {
    let wallpaper = || param::<OnlineWallpaper>(params, "wallpaper");
    Some(match name {
        "search" => match param(params, "query") {
            Ok(query) => json(online::search(state, query).await),
            Err(e) => Err(e),
        },
        "details" => match wallpaper() {
            Ok(w) => json(online::details(state, &w).await),
            Err(e) => Err(e),
        },
        // A local file, or null when it can't be had.
        "preview" => match wallpaper() {
            Ok(w) => json(Ok(online::preview(state, &w).await)),
            Err(e) => Err(e),
        },
        "tag_suggestions" => match (param(params, "provider"), param::<String>(params, "text")) {
            (Ok(provider), Ok(text)) => json(online::tag_suggestions(state, provider, &text).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        "filters" => json(online::filters(state).await),
        "save_filters" => match param(params, "filters") {
            Ok(filters) => json(online::save_filters(state, filters).await),
            Err(e) => Err(e),
        },
        "downloaded" => json(online::downloaded(state).await),
        "download" => match wallpaper() {
            Ok(w) => download(state, out, &w).await,
            Err(e) => Err(e),
        },
        "open_url" => match param::<String>(params, "url") {
            Ok(url) => json(online::open_url(&url)),
            Err(e) => Err(e),
        },
        "collections" => json(online::wallhaven_collections(state).await),
        "collection" => match (param(params, "id"), param(params, "page")) {
            (Ok(id), Ok(page)) => json(online::wallhaven_collection_page(state, id, page).await),
            (Err(e), _) | (_, Err(e)) => Err(e),
        },
        _ => return None,
    })
}

/// Where it was saved. Its progress goes out as events, keyed like
/// `online.downloaded`'s list ("provider:id").
async fn download(state: &AppState, out: &Out, w: &OnlineWallpaper) -> CmdResult<Value> {
    let key = w.key();
    let last = Mutex::new(None::<Instant>);
    let progress = |done: usize, total: Option<usize>| {
        let finished = total == Some(done);
        let mut last = last.lock().unwrap_or_else(|p| p.into_inner());
        if !finished && last.is_some_and(|t| t.elapsed() < PROGRESS_EVERY) {
            return;
        }
        *last = Some(Instant::now());
        out.event("download-progress", value!({ "key": key, "done": done, "total": total }));
    };
    let result = json(online::download_and_tag(state, w, progress).await);
    if result.is_ok() {
        out.event("library-changed", value!(null));
    }
    result
}
