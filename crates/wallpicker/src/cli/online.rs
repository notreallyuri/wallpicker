//! `wallpicker search`, `details` and `apply-online`: the online providers,
//! with each provider's filters as saved in the app.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::moebooru::MoebooruSort;
use wallpicker_core::online::{OnlineQuery, OnlineWallpaper, Page, Provider};
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::wallhaven::Sorting;

use super::{Choice, fail, print_json, usage_for};
use crate::AppState;

/// Largest thumbnail accepted; real ones are a few hundred KB.
const MAX_THUMB_BYTES: usize = 8 * 1024 * 1024;
/// Thumbnails fetched at once.
const THUMB_PARALLEL: usize = 6;

#[derive(Serialize)]
struct SearchOutput {
    provider: &'static str,
    providers: Vec<Choice>,
    /// This provider's sort, and the ones it offers.
    sort: &'static str,
    sorts: Vec<Choice>,
    /// How many of this provider's saved filters differ from the defaults.
    filters_active: usize,
    query: String,
    page: u64,
    last_page: u64,
    /// 0 when the provider doesn't say (Konachan/yande.re without a count).
    total: u64,
    items: Vec<ResultOut>,
}

#[derive(Serialize)]
struct ResultOut {
    key: String,
    id: String,
    resolution: String,
    /// "sfw", "sketchy" or "nsfw".
    purity: String,
    /// "♥ 12" (Wallhaven favorites) or "Score 19".
    likes: String,
    page_url: String,
    /// Already in the download folder: applying it won't download again.
    downloaded: bool,
    /// Local copy of the thumbnail, with `--thumbs`.
    thumb: Option<PathBuf>,
    /// Pass to `wallpicker apply-online` as is.
    handle: String,
}

fn sorts_for(provider: Provider) -> Vec<Choice> {
    match provider {
        Provider::Wallhaven => Sorting::ALL
            .iter()
            .map(|s| Choice { value: s.as_str(), label: s.label() })
            .collect(),
        Provider::Konachan | Provider::Yandere => MoebooruSort::ALL
            .iter()
            .map(|s| Choice { value: s.as_str(), label: s.label() })
            .collect(),
    }
}

fn likes_label(w: &OnlineWallpaper) -> String {
    match w.provider {
        Provider::Wallhaven => format!("♥ {}", w.likes),
        Provider::Konachan | Provider::Yandere => format!("Score {}", w.likes),
    }
}

/// `wallpicker search <provider> [query...] [--page N] [--sort S] [--json] [--thumbs]`.
/// Uses the provider's saved filters (the app's Filters dialog); `--sort`
/// changes that provider's saved sort, for the app too.
pub fn search(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker search <wallhaven|konachan|yandere> [query...] [--page N] [--sort S] [--json] [--thumbs]";
    let mut provider = None;
    let mut words = Vec::new();
    let (mut page, mut sort, mut json, mut want_thumbs) = (1u32, None, false, false);
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--json" => json = true,
            "--thumbs" => want_thumbs = true,
            "--page" => match rest.next().and_then(|p| p.parse::<u32>().ok()).filter(|p| *p >= 1) {
                Some(p) => page = p,
                None => return usage_for("search", "--page needs a number from 1", usage_text),
            },
            "--sort" => match rest.next() {
                Some(s) => sort = Some(s.clone()),
                None => return usage_for("search", "--sort needs a value", usage_text),
            },
            flag if flag.starts_with('-') => return usage_for("search", &format!("unknown option {flag}"), usage_text),
            p if provider.is_none() => match Provider::from_str(p) {
                Some(p) => provider = Some(p),
                None => return usage_for("search", &format!("unknown provider {p}"), usage_text),
            },
            word => words.push(word.to_string()),
        }
    }
    let Some(provider) = provider else {
        return usage_for("search", "which provider?", usage_text);
    };
    let query = words.join(" ");

    let store = Store::new(Dirs::from_env());
    let mut filters = match store.load_online_filters() {
        Ok(f) => f,
        Err(e) => return fail(&e),
    };
    if let Some(sort) = &sort {
        let ok = match provider {
            Provider::Wallhaven => Sorting::from_str(sort).map(|s| filters.wallhaven.sorting = s).is_some(),
            Provider::Konachan => MoebooruSort::from_str(sort).map(|s| filters.konachan.sorting = s).is_some(),
            Provider::Yandere => MoebooruSort::from_str(sort).map(|s| filters.yandere.sorting = s).is_some(),
        };
        if !ok {
            let valid: Vec<&str> = sorts_for(provider).iter().map(|c| c.value).collect();
            return usage_for("search", &format!("{sort} isn't a {} sort ({})", provider.as_str(), valid.join(", ")), usage_text);
        }
        if let Err(e) = store.save_online_filters(&filters) {
            return fail(&e);
        }
    }
    let filters_active = match provider {
        Provider::Wallhaven => filters.wallhaven.active_filters(),
        Provider::Konachan => filters.konachan.active_filters(),
        Provider::Yandere => filters.yandere.active_filters(),
    };
    let (online_query, current_sort) = match provider {
        Provider::Wallhaven => {
            let mut q = filters.wallhaven.clone();
            (q.q, q.page) = (query.clone(), page);
            let sort = q.sorting.as_str();
            (OnlineQuery::Wallhaven(q), sort)
        }
        Provider::Konachan => {
            let mut q = filters.konachan.clone();
            (q.tags, q.page) = (query.clone(), page);
            let sort = q.sorting.as_str();
            (OnlineQuery::Konachan(q), sort)
        }
        Provider::Yandere => {
            let mut q = filters.yandere.clone();
            (q.tags, q.page) = (query.clone(), page);
            let sort = q.sorting.as_str();
            (OnlineQuery::Yandere(q), sort)
        }
    };

    let state = AppState::new(store.clone());
    let result = crate::rt::block_on(async {
        let page = crate::service::online::search(&state, online_query).await?;
        let thumbs = if want_thumbs {
            fetch_thumbs(&state, &page.items).await
        } else {
            Vec::new()
        };
        Ok::<_, String>((page, thumbs))
    });
    let (page, thumbs) = match result {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let downloaded = crate::service::online::downloaded_keys(&store).unwrap_or_default();

    let out = SearchOutput {
        provider: provider.as_str(),
        providers: Provider::ALL
            .iter()
            .map(|p| Choice { value: p.as_str(), label: p.label() })
            .collect(),
        sort: current_sort,
        sorts: sorts_for(provider),
        filters_active,
        query,
        page: page.page,
        last_page: page.last_page,
        total: page.total,
        items: result_items(&page, &thumbs, &downloaded),
    };

    let mut stdout = std::io::stdout().lock();
    let written = if json {
        match serde_json::to_string(&out) {
            Ok(s) => writeln!(stdout, "{s}"),
            Err(e) => return fail(&e),
        }
    } else {
        writeln!(stdout, "{} page {} of {}", out.provider, out.page, out.last_page).and_then(|()| {
            out.items.iter().try_for_each(|i| {
                let mark = if i.downloaded { "d" } else { " " };
                writeln!(stdout, "{mark} {}  {}  {}", i.key, i.resolution, i.page_url)
            })
        })
    };
    match written {
        Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => fail(&e),
        _ => 0,
    }
}

/// Hosts the providers serve thumbnails and previews from.
const IMAGE_HOSTS: [&str; 6] = [
    "th.wallhaven.cc",
    "w.wallhaven.cc",
    "konachan.net",
    "konachan.com",
    "assets.yande.re",
    "files.yande.re",
];

/// Largest preview accepted: above the size of a Wallhaven original used as
/// one (see `OnlineWallpaper::details_image`).
const MAX_PREVIEW_BYTES: usize = 32 * 1024 * 1024;
/// Cached thumbnails and previews older than this are removed on a search.
const CACHE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(14 * 24 * 60 * 60);

/// `dir/<provider>-<id>.<ext>` for an image of `w` at `url`, or None when the
/// id or the address can't be trusted.
fn cache_path(dir: &Path, w: &OnlineWallpaper, url: &str) -> Option<PathBuf> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let trusted = parsed.scheme() == "https" && parsed.host_str().is_some_and(|h| IMAGE_HOSTS.contains(&h));
    if !trusted || !wallpicker_core::online::is_valid_id(&w.id) {
        return None;
    }
    let ext = match parsed.path().rsplit('.').next().map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "png",
        Some("webp") => "webp",
        _ => "jpg",
    };
    Some(dir.join(format!("{}-{}.{ext}", w.provider.as_str(), w.id)))
}

/// Download `url` to `dest` unless it's already there.
async fn cache_image(http: reqwest::Client, url: String, dest: PathBuf, max: usize) -> Option<PathBuf> {
    if dest.metadata().is_ok_and(|m| m.len() > 0) {
        return Some(dest);
    }
    let resp = http.get(&url).send().await.ok()?;
    if !resp.status().is_success() || resp.content_length().unwrap_or(0) as usize > max {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() > max {
        return None;
    }
    // Beside the destination, then renamed: never a half file.
    let tmp = dest.with_extension("part");
    std::fs::write(&tmp, &bytes).ok()?;
    std::fs::rename(&tmp, &dest).ok()?;
    Some(dest)
}

/// Drop cached images nobody has looked at in a while.
fn prune_cache(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age > CACHE_MAX_AGE);
        if old && entry.file_type().is_ok_and(|t| t.is_file()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// A page's results as the plugin gets them.
fn result_items(page: &Page, thumbs: &[Option<PathBuf>], downloaded: &[String]) -> Vec<ResultOut> {
    page.items
        .iter()
        .enumerate()
        .map(|(i, w)| ResultOut {
            key: w.key(),
            id: w.id.clone(),
            resolution: w.resolution(),
            purity: w.purity.clone(),
            likes: likes_label(w),
            page_url: w.page_url.clone(),
            downloaded: downloaded.contains(&w.key()),
            thumb: thumbs.get(i).cloned().flatten(),
            handle: serde_json::to_string(w).unwrap_or_default(),
        })
        .collect()
}

#[derive(Serialize)]
struct TagsOutput {
    exact: bool,
    suggestions: Vec<TagOutput>,
}

#[derive(Serialize)]
struct TagOutput {
    name: String,
    count: u64,
    kind: String,
    /// The search with this tag in it.
    applied: String,
}

/// `wallpicker tags <konachan|yandere> <text...> [--json]`: the real tags a
/// search means (these sites match exact tag names only).
pub fn tags(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker tags <konachan|yandere> <text...> [--json]";
    let json = args.iter().any(|a| a == "--json");
    let rest: Vec<&String> = args.iter().filter(|a| *a != "--json").collect();
    let Some((provider, words)) = rest.split_first() else {
        return usage_for("tags", "which site?", usage_text);
    };
    let Some(provider) = Provider::from_str(provider) else {
        return usage_for("tags", &format!("unknown provider {provider}"), usage_text);
    };
    let text = words.iter().map(|w| w.as_str()).collect::<Vec<_>>().join(" ");
    let state = AppState::new(Store::new(Dirs::from_env()));
    match crate::rt::block_on(crate::service::online::tag_suggestions(&state, provider, &text)) {
        Ok(list) if json => {
            // Each with the search it makes, and whether what's typed already
            // ends in a real tag (then there's nothing to suggest).
            let last = wallpicker_core::moebooru::plain_words(&text).last().cloned();
            let exact = list.iter().any(|t| !t.replaces_all && Some(&t.name) == last.as_ref());
            let suggestions: Vec<TagOutput> = list
                .iter()
                .map(|t| TagOutput {
                    name: t.name.clone(),
                    count: t.count,
                    kind: t.kind.clone(),
                    applied: wallpicker_core::moebooru::apply_suggestion(&text, t),
                })
                .collect();
            print_json(&TagsOutput { exact, suggestions })
        }
        Ok(list) => {
            for t in &list {
                println!("{:>7}  {}  ({})", t.count, t.name, t.kind);
            }
            0
        }
        Err(e) => fail(&e),
    }
}

/// `wallpicker collections [--json]`: your Wallhaven collections (needs the
/// API key).
pub fn collections(args: &[String]) -> i32 {
    let json = args.iter().any(|a| a == "--json");
    let state = AppState::new(Store::new(Dirs::from_env()));
    match crate::rt::block_on(crate::service::online::wallhaven_collections(&state)) {
        Ok(list) if json => print_json(&list),
        Ok(list) => {
            for c in &list {
                println!("{}  {} ({}{})", c.id, c.label, c.count, if c.public { ", public" } else { "" });
            }
            0
        }
        Err(e) => fail(&e),
    }
}

#[derive(Serialize)]
struct CollectionOutput {
    id: u64,
    page: u64,
    last_page: u64,
    total: u64,
    items: Vec<ResultOut>,
}

/// `wallpicker collection <id> [--page N] [--json] [--thumbs]`: a page of one
/// of your Wallhaven collections, like `search`'s results.
pub fn collection(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker collection <id> [--page N] [--json] [--thumbs]";
    let (mut id, mut page, mut json, mut want_thumbs) = (None, 1u32, false, false);
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--json" => json = true,
            "--thumbs" => want_thumbs = true,
            "--page" => match rest.next().and_then(|p| p.parse::<u32>().ok()).filter(|p| *p >= 1) {
                Some(p) => page = p,
                None => return usage_for("collection", "--page needs a number from 1", usage_text),
            },
            n if id.is_none() => match n.parse::<u64>() {
                Ok(n) => id = Some(n),
                Err(_) => return usage_for("collection", &format!("{n} isn't a collection id"), usage_text),
            },
            other => return usage_for("collection", &format!("unknown option {other}"), usage_text),
        }
    }
    let Some(id) = id else {
        return usage_for("collection", "which collection? (`wallpicker collections` lists them)", usage_text);
    };
    let store = Store::new(Dirs::from_env());
    let state = AppState::new(store.clone());
    let result = crate::rt::block_on(async {
        let page = crate::service::online::wallhaven_collection_page(&state, id, page).await?;
        let thumbs = if want_thumbs { fetch_thumbs(&state, &page.items).await } else { Vec::new() };
        Ok::<_, String>((page, thumbs))
    });
    let (page, thumbs) = match result {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let downloaded = crate::service::online::downloaded_keys(&store).unwrap_or_default();
    let out = CollectionOutput {
        id,
        page: page.page,
        last_page: page.last_page,
        total: page.total,
        items: result_items(&page, &thumbs, &downloaded),
    };
    if json {
        return print_json(&out);
    }
    println!("collection {id}: page {} of {}", out.page, out.last_page);
    for i in &out.items {
        println!("{} {}  {}  {}", if i.downloaded { "d" } else { " " }, i.key, i.resolution, i.page_url);
    }
    0
}

/// Local copies of the results' thumbnails (None where one couldn't be had),
/// in result order. Cached by provider and id.
async fn fetch_thumbs(state: &AppState, items: &[OnlineWallpaper]) -> Vec<Option<PathBuf>> {
    let dir = state.store.dirs().cache.join("online");
    if std::fs::create_dir_all(&dir).is_err() {
        return vec![None; items.len()];
    }
    prune_cache(&dir);
    let gate = std::sync::Arc::new(tokio::sync::Semaphore::new(THUMB_PARALLEL));
    let tasks: Vec<_> = items
        .iter()
        .map(|w| {
            let (http, gate, url) = (state.http.clone(), gate.clone(), w.thumb.clone());
            let dest = cache_path(&dir, w, &url);
            crate::rt::spawn(async move {
                let dest = dest?;
                let _permit = gate.acquire().await.ok()?;
                cache_image(http, url, dest, MAX_THUMB_BYTES).await
            })
        })
        .collect();
    let mut out = Vec::with_capacity(tasks.len());
    for t in tasks {
        out.push(t.await.ok().flatten());
    }
    out
}

#[derive(Serialize)]
struct DetailsOutput {
    provider: &'static str,
    provider_label: &'static str,
    /// "2560x1440 · anime".
    title: String,
    /// "♥ 12", "3400 views", "2.1 MB".
    stats: Vec<String>,
    page_url: String,
    /// Where the image came from; only http(s) links, else "".
    source: String,
    uploader: Option<SearchLink>,
    artists: Vec<SearchLink>,
    tags: Vec<TagOut>,
    downloaded: bool,
    /// Local copy of the bigger preview, with `--preview`.
    preview: Option<PathBuf>,
}

/// A name, and the search (in the same provider) that finds more of it.
#[derive(Serialize)]
struct SearchLink {
    name: String,
    query: Option<String>,
}

#[derive(Serialize)]
struct TagOut {
    name: String,
    query: String,
    category: String,
    is_artist: bool,
}

/// A local copy of the image `w`'s details show (cached), if it can be had.
/// Previews (the app's details, the panel's) are fitted inside this many
/// pixels: an 8K original decoded where it's shown is ~130 MB, a fitted one
/// a few MB, and still sharp in a details view. Settings → Performance can
/// ask for originals.
const FITTED_PREVIEW: u32 = 1600;

/// The sharpest image for `w`'s details, downloaded (cached), then fitted to
/// [`FITTED_PREVIEW`] unless originals are wanted or it's already small.
pub async fn fetch_preview(state: &AppState, w: &OnlineWallpaper) -> Option<PathBuf> {
    let dir = state.store.dirs().cache.join("online").join("preview");
    std::fs::create_dir_all(&dir).ok()?;
    prune_cache(&dir);
    let url = w.details_image().to_string();
    let dest = cache_path(&dir, w, &url)?;
    let original = cache_image(state.http.clone(), url, dest, MAX_PREVIEW_BYTES).await?;
    if state.store.load_settings().is_ok_and(|s| s.full_previews) {
        return Some(original);
    }
    let fit_dir = state.store.dirs().cache.join("online").join("preview-fit");
    prune_cache(&fit_dir.join("thumbs"));
    crate::rt::spawn_blocking(move || {
        wallpicker_core::thumbs::fit_within(&fit_dir, &original, FITTED_PREVIEW).ok()
    })
    .await
    .ok()
    .flatten()
}

#[derive(Serialize)]
struct PreviewOutput {
    preview: Option<PathBuf>,
}

/// `wallpicker preview <handle>`: just the details image, as
/// `{"preview": path}`, so it can load beside a quick `details`.
pub fn preview(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker preview <handle>";
    let [handle] = args else {
        return usage_for("preview", "one wallpaper handle, please", usage_text);
    };
    let w: OnlineWallpaper = match serde_json::from_str(handle) {
        Ok(w) => w,
        Err(e) => return fail(&format!("not a wallpaper handle: {e}")),
    };
    let state = AppState::new(Store::new(Dirs::from_env()));
    let preview = crate::rt::block_on(fetch_preview(&state, &w));
    print_json(&PreviewOutput { preview })
}

/// `wallpicker details <handle> [--preview]`: uploader, source and tags of a
/// result from `search --json`, as JSON.
pub fn details(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker details <handle> [--preview]";
    let mut handle = None;
    let mut want_preview = false;
    for a in args {
        match a.as_str() {
            "--preview" => want_preview = true,
            "--json" => {} // always JSON; accepted for symmetry
            flag if flag.starts_with('-') => return usage_for("details", &format!("unknown option {flag}"), usage_text),
            h if handle.is_none() => handle = Some(h),
            _ => return usage_for("details", "one wallpaper at a time", usage_text),
        }
    }
    let Some(handle) = handle else {
        return usage_for("details", "which wallpaper?", usage_text);
    };
    let w: OnlineWallpaper = match serde_json::from_str(handle) {
        Ok(w) => w,
        Err(e) => return fail(&format!("not a wallpaper handle: {e}")),
    };
    let store = Store::new(Dirs::from_env());
    let state = AppState::new(store.clone());
    let result = crate::rt::block_on(async {
        let preview = async {
            if want_preview { fetch_preview(&state, &w).await } else { None }
        };
        let (d, preview) = tokio::join!(crate::service::online::details(&state, &w), preview);
        d.map(|d| (d, preview))
    });
    let (d, preview) = match result {
        Ok(r) => r,
        Err(e) => return fail(&e),
    };
    let downloaded = crate::service::online::downloaded_keys(&store).unwrap_or_default().contains(&w.key());

    let mut stats = vec![likes_label(&w)];
    stats.extend(w.views.map(|v| format!("{v} views")));
    if w.file_size > 0 {
        stats.push(format!("{:.1} MB", w.file_size as f64 / 1_000_000.0));
    }
    let source = d.source.trim();
    let out = DetailsOutput {
        provider: w.provider.as_str(),
        provider_label: w.provider.label(),
        title: if w.category.is_empty() {
            w.resolution()
        } else {
            format!("{} · {}", w.resolution(), w.category)
        },
        stats,
        page_url: w.page_url.clone(),
        source: if source.starts_with("https://") || source.starts_with("http://") {
            source.to_string()
        } else {
            String::new()
        },
        uploader: d.uploader.map(|u| SearchLink { name: u.name, query: u.query }),
        artists: d
            .tags
            .iter()
            .filter(|t| t.is_artist)
            .map(|t| SearchLink { name: t.name.clone(), query: Some(t.query.clone()) })
            .collect(),
        tags: d
            .tags
            .into_iter()
            .map(|t| TagOut { name: t.name, query: t.query, category: t.category, is_artist: t.is_artist })
            .collect(),
        downloaded,
        preview,
    };
    print_json(&out)
}

/// `wallpicker apply-online <handle> [--monitor NAME]... [--download-only]`.
/// The handle is a result's `handle` from `wallpicker search --json`.
pub fn apply_online(args: &[String]) -> i32 {
    let usage_text = "usage: wallpicker apply-online <handle> [--monitor NAME]... [--download-only]";
    let mut handle = None;
    let mut monitors = Vec::new();
    let mut download_only = false;
    let mut rest = args.iter();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--monitor" | "-m" => match rest.next() {
                Some(m) => monitors.push(m.clone()),
                None => return usage_for("apply-online", "--monitor needs a monitor name", usage_text),
            },
            "--download-only" => download_only = true,
            flag if flag.starts_with('-') && !flag.starts_with("-{") => {
                return usage_for("apply-online", &format!("unknown option {flag}"), usage_text);
            }
            h if handle.is_none() => handle = Some(h.to_string()),
            _ => return usage_for("apply-online", "only one wallpaper at a time", usage_text),
        }
    }
    let Some(handle) = handle else {
        return usage_for("apply-online", "which wallpaper?", usage_text);
    };
    // The download re-checks the id (it becomes a file name) and that the
    // image comes from the provider's own host.
    let wallpaper: OnlineWallpaper = match serde_json::from_str(&handle) {
        Ok(w) => w,
        Err(e) => return fail(&format!("not a wallpaper handle: {e}")),
    };

    let store = Store::new(Dirs::from_env());
    let settings = match store.load_settings() {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let state = AppState::new(store.clone());
    let path = match crate::rt::block_on(crate::service::online::download_and_tag(&state, &wallpaper, |_, _| {})) {
        Ok(p) => PathBuf::from(p),
        Err(e) => return fail(&e),
    };
    if download_only {
        println!("saved {}", path.display());
        return 0;
    }
    if monitors.is_empty() {
        match hyprpaper::list_monitors(&SystemRunner) {
            Ok(ms) => monitors = ms.into_iter().map(|m| m.name).collect(),
            Err(e) => return fail(&e),
        }
    }
    match hyprpaper::apply_and_remember_with(
        &SystemRunner,
        &store,
        settings.wallpaper_backend,
        &monitors,
        &path,
    ) {
        Ok(report) => {
            wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
            for m in &report.applied {
                println!("applied to {m}");
            }
            for (m, why) in &report.failed {
                eprintln!("{m}: {why}");
            }
            i32::from(!report.failed.is_empty())
        }
        Err(e) => fail(&e),
    }
}

