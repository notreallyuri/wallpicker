//! The online providers (Wallhaven, Konachan, yande.re): search, details,
//! collections, tag suggestions, previews and downloads.

use std::path::{Path, PathBuf};
use wallpicker_core::moebooru::{self, MoebooruQuery, Site};
use wallpicker_core::online::{
    self, Details, OnlineFilters, OnlineQuery, OnlineWallpaper, Page, Provider,
};
use wallpicker_core::store::Store;
use wallpicker_core::wallhaven::{self, API_SEARCH_URL, API_WALLPAPER_URL};

use crate::cli;
use crate::commands::library::update_catalog;
use crate::{AppState, CmdResult, blocking, err};

/// Largest Wallhaven download we accept. Wallpapers are a few MB; this only
/// guards against a misbehaving server.
pub(crate) const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

/// The real tags a Konachan or yande.re search means (they match exact tag
/// names only): several words tried as one tag, and the last word anywhere
/// in a tag; most used first. Wallhaven searches loosely: nothing to suggest.
pub(crate) async fn tag_suggestions_for(
    state: &AppState,
    provider: Provider,
    text: &str,
) -> CmdResult<Vec<moebooru::TagSuggestion>> {
    let site = match provider {
        Provider::Wallhaven => return Ok(Vec::new()),
        Provider::Yandere => Site::Yandere,
        Provider::Konachan => {
            let store = state.store.clone();
            Site::from(blocking(move || Ok(store.load_settings()?.konachan_site)).await?)
        }
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

#[tauri::command]
pub(crate) async fn tag_suggestions(
    state: tauri::State<'_, AppState>,
    provider: Provider,
    text: String,
) -> CmdResult<Vec<moebooru::TagSuggestion>> {
    tag_suggestions_for(&state, provider, &text).await
}

/// Your Wallhaven collections: they need the API key (private ones too).
pub(crate) async fn wallhaven_collections_of(
    state: &AppState,
) -> CmdResult<Vec<wallhaven::Collection>> {
    let Some(key) = wallhaven_key(state).await? else {
        return Err("Your collections need a Wallhaven API key (Settings → Wallhaven).".into());
    };
    let req = state
        .http
        .get(wallhaven::API_COLLECTIONS_URL)
        .header("X-API-Key", key);
    let text = get_text(Provider::Wallhaven, req).await?;
    wallhaven::parse_collections(&text).map_err(err)
}

/// A page of one of your collections. Opening one needs your username too:
/// the API doesn't tell whose key it is.
pub(crate) async fn wallhaven_collection_page(
    state: &AppState,
    id: u64,
    page: u32,
) -> CmdResult<Page> {
    let store = state.store.clone();
    let username = blocking(move || Ok(store.load_settings()?.wallhaven_username)).await?;
    let Some(username) = username else {
        return Err(
            "Add your Wallhaven username in Settings → Wallhaven to open your collections.".into(),
        );
    };
    let url = wallhaven::collection_url(&username, id, page)
        .ok_or("That Wallhaven username isn't valid.")?;
    let mut req = state.http.get(url);
    if let Some(key) = wallhaven_key(state).await? {
        req = req.header("X-API-Key", key);
    }
    let text = get_text(Provider::Wallhaven, req).await?;
    Ok(Page::from(
        wallhaven::parse_search_response(&text).map_err(err)?,
    ))
}

#[tauri::command]
pub(crate) async fn wallhaven_collections(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<wallhaven::Collection>> {
    wallhaven_collections_of(&state).await
}

#[tauri::command]
pub(crate) async fn wallhaven_collection(
    state: tauri::State<'_, AppState>,
    id: u64,
    page: u32,
) -> CmdResult<Page> {
    wallhaven_collection_page(&state, id, page).await
}

/// The image for an online wallpaper's details: downloaded, cached and fitted
/// to the screen (see `cli::online::fetch_preview`); None when it can't be.
#[tauri::command]
pub(crate) async fn online_preview(
    state: tauri::State<'_, AppState>,
    wallpaper: OnlineWallpaper,
) -> CmdResult<Option<PathBuf>> {
    Ok(cli::online::fetch_preview(&state, &wallpaper).await)
}

pub(crate) fn http_error(provider: Provider, status: reqwest::StatusCode) -> String {
    let name = provider.label();
    match (provider, status.as_u16()) {
        (Provider::Wallhaven, 401) => {
            "Wallhaven rejected the request (check your API key in Settings).".into()
        }
        // Not the API itself: Cloudflare in front of it answers some requests
        // with a browser check. Any search containing "like" (so `like:<id>`)
        // always gets it; that search is broken on the website too.
        (Provider::Wallhaven, 403) => "Wallhaven's bot protection blocked this request. Searches \
            containing \"like\" (such as like:<id>) are always blocked, on the website too."
            .into(),
        (Provider::Wallhaven, 429) => {
            "Wallhaven rate limit reached (45 requests per minute). Wait a moment.".into()
        }
        (_, 429) => format!("{name} is limiting requests. Wait a moment."),
        (_, code) => format!("{name} returned HTTP {code}."),
    }
}

pub(crate) async fn wallhaven_key(state: &AppState) -> CmdResult<Option<String>> {
    let store = state.store.clone();
    blocking(move || Ok(store.load_settings()?.wallhaven_api_key)).await
}

pub(crate) async fn get_text(
    provider: Provider,
    req: reqwest::RequestBuilder,
) -> CmdResult<String> {
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(http_error(provider, resp.status()));
    }
    resp.text().await.map_err(err)
}

#[tauri::command]
pub(crate) async fn online_search(
    state: tauri::State<'_, AppState>,
    query: OnlineQuery,
) -> CmdResult<Page> {
    search(&state, query).await
}

/// One page of results from any provider (the app and `wallpicker search`).
pub(crate) async fn search(state: &AppState, query: OnlineQuery) -> CmdResult<Page> {
    match query {
        OnlineQuery::Wallhaven(q) => {
            let key = wallhaven_key(&state).await?;
            let mut req = state
                .http
                .get(API_SEARCH_URL)
                .query(&q.to_params(key.is_some()));
            if let Some(key) = &key {
                // A header, not a URL parameter, so the key never lands in logs.
                req = req.header("X-API-Key", key);
            }
            let text = get_text(Provider::Wallhaven, req).await?;
            Ok(Page::from(
                wallhaven::parse_search_response(&text).map_err(err)?,
            ))
        }
        OnlineQuery::Konachan(q) => {
            // konachan.net (safe-for-work) or konachan.com, per Settings.
            let store = state.store.clone();
            let site =
                Site::from(blocking(move || Ok(store.load_settings()?.konachan_site)).await?);
            // konachan.net only has Safe posts: ratings picked while on
            // konachan.com would otherwise find nothing there.
            let q = match site {
                Site::KonachanNet => MoebooruQuery {
                    ratings: moebooru::Ratings::default(),
                    ..q
                },
                _ => q,
            };
            moebooru_search(&state, site, &q).await
        }
        OnlineQuery::Yandere(q) => moebooru_search(&state, Site::Yandere, &q).await,
    }
}

pub(crate) async fn moebooru_search(
    state: &AppState,
    site: Site,
    q: &MoebooruQuery,
) -> CmdResult<Page> {
    let provider = site.provider();
    let posts = state.http.get(site.posts_json_url()).query(&q.to_params());
    let count = state
        .http
        .get(site.posts_xml_url())
        .query(&q.count_params());
    let (posts, count) = tokio::join!(get_text(provider, posts), get_text(provider, count));
    let (posts, types) = moebooru::parse_posts(&posts?).map_err(err)?;
    // Only the pager needs the count; without it, it still knows whether
    // there's a next page.
    let total = count.ok().and_then(|xml| moebooru::parse_count(&xml));
    Ok(moebooru::page(site, posts, &types, q.page, total))
}

/// The filters from last session (without search text), for every provider.
#[tauri::command]
pub(crate) async fn get_online_filters(
    state: tauri::State<'_, AppState>,
) -> CmdResult<OnlineFilters> {
    let store = state.store.clone();
    blocking(move || store.load_online_filters()).await
}

#[tauri::command]
pub(crate) async fn save_online_filters(
    state: tauri::State<'_, AppState>,
    filters: OnlineFilters,
) -> CmdResult<()> {
    let store = state.store.clone();
    blocking(move || store.save_online_filters(&filters)).await
}

/// Uploader, source and tags. Some providers include them in the search
/// result; Wallhaven needs a request.
#[tauri::command]
pub(crate) async fn online_details(
    state: tauri::State<'_, AppState>,
    wallpaper: OnlineWallpaper,
) -> CmdResult<Details> {
    details(&state, &wallpaper).await
}

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
            Ok(Details::from(
                wallhaven::parse_info_response(&text).map_err(err)?,
            ))
        }
        // Moebooru results carry their details, so this is only reached for
        // a result that somehow lost them.
        Provider::Konachan | Provider::Yandere => Ok(Details::default()),
    }
}

/// Open a web link (a wallpaper's source or page) in the browser.
#[tauri::command]
pub(crate) fn open_url(url: String) -> CmdResult<()> {
    if !wallhaven::is_openable_url(&url) {
        return Err(format!("Refusing to open {url:?}"));
    }
    let mut child = std::process::Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map_err(|e| format!("Could not run xdg-open: {e}"))?;
    // Reap it in the background so it doesn't linger as a zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}

/// Keys ("provider:id") of the wallpapers already in the download folder: the
/// files `online_download` would reuse instead of downloading again.
#[tauri::command]
pub(crate) async fn online_downloaded(state: tauri::State<'_, AppState>) -> CmdResult<Vec<String>> {
    let store = state.store.clone();
    blocking(move || downloaded_keys(&store)).await
}

pub(crate) fn downloaded_keys(store: &Store) -> wallpicker_core::Result<Vec<String>> {
    let dir = store.load_settings()?.download_dir;
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    Ok(entries
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.len() > 0))
        .filter_map(|e| online::key_from_file_name(e.file_name().to_str()?))
        .collect())
}

/// Download a wallpaper into the configured download folder and return where
/// it was saved. Already-downloaded files are reused. New downloads (and
/// untagged old ones) get their artist tags, where the provider marks them.
#[tauri::command]
pub(crate) async fn online_download(
    state: tauri::State<'_, AppState>,
    wallpaper: OnlineWallpaper,
) -> CmdResult<String> {
    download_and_tag(&state, &wallpaper).await
}

/// [`online_download`] without Tauri (the app and `wallpicker apply-online`).
pub(crate) async fn download_and_tag(
    state: &AppState,
    wallpaper: &OnlineWallpaper,
) -> CmdResult<String> {
    let (dest, fresh) = download(state, wallpaper).await?;

    let store = state.store.clone();
    let check = dest.clone();
    let untagged = blocking(move || Ok(store.load_catalog()?.tags(&check).is_empty()))
        .await
        .unwrap_or(false);
    if fresh || untagged {
        // Best effort: the file is saved either way.
        if let Err(e) = tag_artists(state, wallpaper, &dest).await {
            eprintln!("wallpicker: could not tag {}: {e}", dest.display());
        }
    }
    Ok(dest.to_string_lossy().into_owned())
}

/// The wallpaper's artist tags, added to `path` as local tags.
pub(crate) async fn tag_artists(
    state: &AppState,
    wallpaper: &OnlineWallpaper,
    path: &Path,
) -> CmdResult<()> {
    // Wallhaven's need a request; Moebooru results carry theirs (marked from
    // the search's tag kinds).
    let artists: Vec<String> = details(state, wallpaper)
        .await?
        .tags
        .into_iter()
        .filter(|t| t.is_artist)
        .map(|t| t.name)
        .collect();
    if artists.is_empty() {
        return Ok(());
    }
    let path = path.to_path_buf();
    update_catalog(state, move |c| {
        for name in &artists {
            c.add_tag(&path, name);
        }
    })
    .await
    .map(|_| ())
}

/// Returns where the file is, and whether it was downloaded just now.
pub(crate) async fn download(
    state: &AppState,
    wallpaper: &OnlineWallpaper,
) -> CmdResult<(PathBuf, bool)> {
    // The wallpaper comes back from the UI, so re-check what ends up in the
    // file name and which host we contact.
    let file_name = wallpaper.file_name().map_err(err)?;
    if !wallpaper.is_allowed_download_url() {
        return Err("Refusing to download from an unexpected address.".into());
    }
    let provider = wallpaper.provider;

    let store = state.store.clone();
    let dir = blocking(move || Ok(store.load_settings()?.download_dir)).await?;
    let dest = dir.join(&file_name);

    if dest.metadata().map(|m| m.len() > 0).unwrap_or(false) {
        return Ok((dest, false));
    }

    let resp = state.http.get(&wallpaper.full).send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(http_error(provider, resp.status()));
    }
    let is_image = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("image/"))
        .unwrap_or(false);
    if !is_image {
        return Err(format!("{} did not return an image.", provider.label()));
    }
    if resp
        .content_length()
        .map(|n| n as usize > MAX_DOWNLOAD_BYTES)
        .unwrap_or(false)
    {
        return Err("The image is larger than the 64 MiB download limit.".into());
    }
    let bytes = resp.bytes().await.map_err(err)?;
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err("The image is larger than the 64 MiB download limit.".into());
    }

    blocking(move || {
        std::fs::create_dir_all(&dir)?;
        // Write beside the destination, then rename, so a half-finished
        // download is never mistaken for a complete file.
        let tmp = dest.with_extension("part");
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, &dest)?;
        Ok((dest, true))
    })
    .await
}
