//! The online providers (Wallhaven, Konachan, yande.re): search, details,
//! tag suggestions, previews, the saved filters, links. Downloads and
//! Wallhaven collections are in their own files.

mod collections;
mod download;

pub(crate) use collections::{wallhaven_collection_page, wallhaven_collections};
pub(crate) use download::{download_and_tag, downloaded, downloaded_keys};

use std::path::PathBuf;

use wallpicker_core::moebooru::{self, MoebooruQuery, Site};
use wallpicker_core::online::{self, Details, OnlineFilters, OnlineQuery, OnlineWallpaper, Page, Provider};
use wallpicker_core::wallhaven::{self, API_SEARCH_URL, API_WALLPAPER_URL};

use crate::cli;
use crate::{AppState, CmdResult, blocking, err};

/// One page of results from any provider (the app and `wallpicker search`).
pub(crate) async fn search(state: &AppState, query: OnlineQuery) -> CmdResult<Page> {
    match query {
        OnlineQuery::Wallhaven(q) => {
            let key = wallhaven_key(state).await?;
            let mut req = state.http.get(API_SEARCH_URL).query(&q.to_params(key.is_some()));
            if let Some(key) = &key {
                // A header, not a URL parameter, so the key never lands in logs.
                req = req.header("X-API-Key", key);
            }
            let text = get_text(Provider::Wallhaven, req).await?;
            Ok(Page::from(wallhaven::parse_search_response(&text).map_err(err)?))
        }
        OnlineQuery::Konachan(q) => {
            // konachan.net (safe-for-work) or konachan.com, per Settings.
            let site = konachan_site(state).await?;
            // konachan.net only has Safe posts: ratings picked while on
            // konachan.com would otherwise find nothing there.
            let q = match site {
                Site::KonachanNet => MoebooruQuery { ratings: moebooru::Ratings::default(), ..q },
                _ => q,
            };
            moebooru_search(state, site, &q).await
        }
        OnlineQuery::Yandere(q) => moebooru_search(state, Site::Yandere, &q).await,
    }
}

async fn moebooru_search(state: &AppState, site: Site, q: &MoebooruQuery) -> CmdResult<Page> {
    let provider = site.provider();
    let posts = state.http.get(site.posts_json_url()).query(&q.to_params());
    let count = state.http.get(site.posts_xml_url()).query(&q.count_params());
    let (posts, count) = tokio::join!(get_text(provider, posts), get_text(provider, count));
    let (posts, types) = moebooru::parse_posts(&posts?).map_err(err)?;
    // Only the pager needs the count; without it, it still knows whether
    // there's a next page.
    let total = count.ok().and_then(|xml| moebooru::parse_count(&xml));
    Ok(moebooru::page(site, posts, &types, q.page, total))
}

/// Uploader, source and tags. Some providers include them in the search
/// result; Wallhaven needs a request.
pub(crate) async fn details(state: &AppState, wallpaper: &OnlineWallpaper) -> CmdResult<Details> {
    if let Some(d) = &wallpaper.details {
        return Ok(d.clone());
    }
    match wallpaper.provider {
        Provider::Wallhaven => {
            let id = &wallpaper.id;
            if !online::is_valid_id(id) {
                return Err(format!("Invalid wallpaper id: {id:?}"));
            }
            // NSFW wallpapers need the key here too.
            let mut req = state.http.get(format!("{API_WALLPAPER_URL}{id}"));
            if let Some(key) = wallhaven_key(state).await? {
                req = req.header("X-API-Key", key);
            }
            let text = get_text(Provider::Wallhaven, req).await?;
            Ok(Details::from(wallhaven::parse_info_response(&text).map_err(err)?))
        }
        // Moebooru results carry their details, so this is only reached for
        // a result that somehow lost them.
        Provider::Konachan | Provider::Yandere => Ok(Details::default()),
    }
}

/// The image for an online wallpaper's details: downloaded, cached and fitted
/// to the screen (see `cli::online::fetch_preview`); None when it can't be.
pub(crate) async fn preview(state: &AppState, wallpaper: &OnlineWallpaper) -> Option<PathBuf> {
    cli::online::fetch_preview(state, wallpaper).await
}

/// The real tags a Konachan or yande.re search means (they match exact tag
/// names only): several words tried as one tag, and the last word anywhere
/// in a tag; most used first. Wallhaven searches loosely: nothing to suggest.
pub(crate) async fn tag_suggestions(
    state: &AppState,
    provider: Provider,
    text: &str,
) -> CmdResult<Vec<moebooru::TagSuggestion>> {
    let site = match provider {
        Provider::Wallhaven => return Ok(Vec::new()),
        Provider::Yandere => Site::Yandere,
        Provider::Konachan => konachan_site(state).await?,
    };
    let mut out: Vec<moebooru::TagSuggestion> = Vec::new();
    for (pattern, replaces_all) in moebooru::suggestion_patterns(text) {
        let req = state.http.get(site.tags_json_url()).query(&[
            ("name", pattern.as_str()),
            ("order", "count"),
            ("limit", "8"),
        ]);
        let text = get_text(provider, req).await?;
        for t in moebooru::parse_tags(&text, replaces_all).map_err(err)? {
            if !out.iter().any(|o| o.name == t.name) {
                out.push(t);
            }
        }
    }
    out.truncate(10);
    Ok(out)
}

/// [`tag_suggestions`] for the search box: none once the last tag typed is
/// a real one (nothing left to fix).
pub(crate) async fn suggestions(
    state: &AppState,
    provider: Provider,
    text: &str,
) -> CmdResult<Vec<moebooru::TagSuggestion>> {
    let got = tag_suggestions(state, provider, text).await?;
    let last = moebooru::plain_words(text).pop();
    let exact = got.iter().any(|s| !s.replaces_all && Some(&s.name) == last.as_ref());
    Ok(if exact { Vec::new() } else { got })
}

/// The filters from last session (without search text), for every provider.
pub(crate) async fn filters(state: &AppState) -> CmdResult<OnlineFilters> {
    let store = state.store.clone();
    blocking(move || store.load_online_filters()).await
}

pub(crate) async fn save_filters(state: &AppState, filters: OnlineFilters) -> CmdResult<()> {
    let store = state.store.clone();
    blocking(move || store.save_online_filters(&filters)).await
}

/// Open a web link (a wallpaper's source or page) in the browser.
pub(crate) fn open_url(url: &str) -> CmdResult<()> {
    if !wallhaven::is_openable_url(url) {
        return Err(format!("Refusing to open {url:?}"));
    }
    let mut child = std::process::Command::new("xdg-open")
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not run xdg-open: {e}"))?;
    // Reap it in the background so it doesn't linger as a zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}

pub(crate) fn http_error(provider: Provider, status: reqwest::StatusCode) -> String {
    let name = provider.label();
    match (provider, status.as_u16()) {
        (Provider::Wallhaven, 401) => "Wallhaven rejected the request (check your API key in Settings).".into(),
        // Not the API itself: Cloudflare in front of it answers some requests
        // with a browser check. Any search containing "like" (so `like:<id>`)
        // always gets it; that search is broken on the website too.
        (Provider::Wallhaven, 403) => "Wallhaven's bot protection blocked this request. Searches \
            containing \"like\" (such as like:<id>) are always blocked, on the website too."
            .into(),
        (Provider::Wallhaven, 429) => "Wallhaven rate limit reached (45 requests per minute). Wait a moment.".into(),
        (_, 429) => format!("{name} is limiting requests. Wait a moment."),
        (_, code) => format!("{name} returned HTTP {code}."),
    }
}

async fn wallhaven_key(state: &AppState) -> CmdResult<Option<String>> {
    let store = state.store.clone();
    blocking(move || Ok(store.load_settings()?.wallhaven_api_key)).await
}

/// konachan.net or konachan.com, per Settings.
async fn konachan_site(state: &AppState) -> CmdResult<Site> {
    let store = state.store.clone();
    Ok(Site::from(blocking(move || Ok(store.load_settings()?.konachan_site)).await?))
}

async fn get_text(provider: Provider, req: reqwest::RequestBuilder) -> CmdResult<String> {
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(http_error(provider, resp.status()));
    }
    resp.text().await.map_err(err)
}
