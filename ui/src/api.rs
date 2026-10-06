//! Typed wrappers around the Tauri commands defined in `src-tauri/src/lib.rs`.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use wallpicker_core::catalog::Catalog;
use wallpicker_core::library::LocalImage;
use wallpicker_core::monitors::Monitor;
use wallpicker_core::settings::{Settings, State};
use wallpicker_core::online::{Details, OnlineFilters, OnlineQuery, OnlineWallpaper, Page, Provider};
use wasm_bindgen::prelude::*;

// `withGlobalTauri` exposes the API as `window.__TAURI__`.
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = convertFileSrc)]
    fn tauri_convert_file_src(path: &str) -> String;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    fn tauri_listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> Result<js_sys::Promise, JsValue>;
}

/// Run `f` each time the backend emits `event` (for as long as the window lives).
pub fn listen(event: &str, mut f: impl FnMut() + 'static) {
    let handler = Closure::<dyn FnMut(JsValue)>::new(move |_| f());
    // Fails only outside Tauri (a browser preview): then nothing comes.
    if tauri_listen(event, &handler).is_ok() {
        handler.forget();
    }
}

/// Other programs drawing wallpapers besides `backend` ("hyprpaper",
/// "Noctalia's wallpaper"...): they'd fight over the screen.
pub async fn wallpaper_conflicts(backend: &str) -> Result<Vec<String>, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        backend: &'a str,
    }
    call("wallpaper_conflicts", &Args { backend }).await
}

/// Stop one of the programs [`wallpaper_conflicts`] found.
pub async fn stop_wallpaper_program(name: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        name: &'a str,
    }
    call("stop_wallpaper_program", &Args { name }).await
}

/// A library export: where, and how many wallpapers.
#[derive(Debug, Clone, Deserialize)]
pub struct Exported {
    pub path: String,
    pub items: usize,
}

pub async fn backup_export(path: Option<String>) -> Result<Exported, String> {
    #[derive(Serialize)]
    struct Args {
        path: Option<String>,
    }
    call("backup_export", &Args { path }).await
}

pub async fn backup_default_path() -> Result<String, String> {
    call("backup_default_path", &NoArgs {}).await
}

pub async fn backup_import(path: &str, replace: bool) -> Result<wallpicker_core::backup::ImportReport, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
        replace: bool,
    }
    call("backup_import", &Args { path, replace }).await
}

/// One of the library's automatic daily snapshots.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub label: String,
    pub items: usize,
}

pub async fn backup_snapshots() -> Result<Vec<Snapshot>, String> {
    call("backup_snapshots", &NoArgs {}).await
}

pub async fn backup_restore(id: &str) -> Result<Catalog, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        id: &'a str,
    }
    call("backup_restore", &Args { id }).await
}

/// Every library image's colors, by path (computed once, then cached).
pub async fn image_colors() -> Result<std::collections::BTreeMap<std::path::PathBuf, wallpicker_core::colors::ColorProfile>, String> {
    call("image_colors", &NoArgs {}).await
}

/// Where `wallpicker --open` asked the window to go: a tab, "settings",
/// "duplicates", or "tags" with the image.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct OpenPlace {
    pub view: String,
    pub path: Option<String>,
}

pub async fn take_open_request() -> Result<Option<OpenPlace>, String> {
    call("take_open_request", &NoArgs {}).await
}

/// URL the webview can load a local file from (via Tauri's asset protocol).
pub fn asset_url(path: &str) -> String {
    tauri_convert_file_src(path)
}

fn js_error(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

async fn call<A, R>(cmd: &str, args: &A) -> Result<R, String>
where
    A: Serialize + ?Sized,
    R: DeserializeOwned,
{
    // json_compatible: maps become plain objects and None becomes null,
    // which is what Tauri's JSON deserializer expects.
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    let js_args = args.serialize(&ser).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args).await.map_err(js_error)?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct NoArgs {}

/// Mirrors `wallpicker_core::hyprpaper::ApplyReport` (native-only module).
#[derive(Debug, Clone, Deserialize)]
pub struct ApplyReport {
    pub applied: Vec<String>,
    pub failed: Vec<(String, String)>,
}

pub async fn list_monitors() -> Result<Vec<Monitor>, String> {
    call("list_monitors", &NoArgs {}).await
}

pub async fn get_settings() -> Result<Settings, String> {
    call("get_settings", &NoArgs {}).await
}

pub async fn save_settings(settings: &Settings) -> Result<Settings, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        settings: &'a Settings,
    }
    call("save_settings", &Args { settings }).await
}

pub async fn get_state() -> Result<State, String> {
    call("get_state", &NoArgs {}).await
}

pub async fn scan_local() -> Result<Vec<LocalImage>, String> {
    call("scan_local", &NoArgs {}).await
}

pub async fn thumbnail(path: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
    }
    call("thumbnail", &Args { path }).await
}

pub async fn apply_wallpaper(monitors: &[String], path: &str) -> Result<ApplyReport, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        monitors: &'a [String],
        path: &'a str,
    }
    call("apply_wallpaper", &Args { monitors, path }).await
}

pub async fn open_url(url: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        url: &'a str,
    }
    call("open_url", &Args { url }).await
}

pub async fn get_catalog() -> Result<Catalog, String> {
    call("get_catalog", &NoArgs {}).await
}

pub async fn set_favorite(path: &str, favorite: bool) -> Result<Catalog, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
        favorite: bool,
    }
    call("set_favorite", &Args { path, favorite }).await
}

pub async fn add_tag(path: &str, tag: &str) -> Result<Catalog, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
        tag: &'a str,
    }
    call("add_tag", &Args { path, tag }).await
}

pub async fn remove_tag(path: &str, tag: &str) -> Result<Catalog, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
        tag: &'a str,
    }
    call("remove_tag", &Args { path, tag }).await
}

pub async fn online_search(query: &OnlineQuery) -> Result<Page, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        query: &'a OnlineQuery,
    }
    call("online_search", &Args { query }).await
}

pub async fn online_details(wallpaper: &OnlineWallpaper) -> Result<Details, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        wallpaper: &'a OnlineWallpaper,
    }
    call("online_details", &Args { wallpaper }).await
}

pub async fn online_download(wallpaper: &OnlineWallpaper) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        wallpaper: &'a OnlineWallpaper,
    }
    call("online_download", &Args { wallpaper }).await
}

/// Keys ("provider:id") of the wallpapers already in the download folder.
pub async fn online_downloaded() -> Result<Vec<String>, String> {
    call("online_downloaded", &NoArgs {}).await
}

pub async fn get_online_filters() -> Result<OnlineFilters, String> {
    call("get_online_filters", &NoArgs {}).await
}

pub async fn save_online_filters(filters: &OnlineFilters) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        filters: &'a OnlineFilters,
    }
    call("save_online_filters", &Args { filters }).await
}

/// Move a local wallpaper to the trash.
/// Give a wallpaper its own fit (None: the monitor's); re-applied where it
/// shows.
pub async fn set_wallpaper_fit(path: &str, fit: Option<wallpicker_core::settings::FitMode>) -> Result<Catalog, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
        fit: Option<wallpicker_core::settings::FitMode>,
    }
    call("set_wallpaper_fit", &Args { path, fit }).await
}

/// `{enabled, minutes, next_in}`: when the slideshow changes next.
#[derive(Debug, Clone, Deserialize)]
pub struct SlideshowStatus {
    pub enabled: bool,
    pub next_in: Option<u64>,
    /// "day" or "night", when the night set is on.
    #[serde(default)]
    pub period: Option<String>,
}

pub async fn slideshow_status() -> Result<SlideshowStatus, String> {
    call("slideshow_status", &NoArgs {}).await
}

/// A slideshow change now (the timer starts over).
pub async fn slideshow_now() -> Result<(), String> {
    call("slideshow_now", &NoArgs {}).await
}

/// The real tags a Konachan or yande.re search means (those sites match
/// exact tag names only).
pub async fn tag_suggestions(provider: Provider, text: &str) -> Result<Vec<wallpicker_core::moebooru::TagSuggestion>, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        provider: Provider,
        text: &'a str,
    }
    call("tag_suggestions", &Args { provider, text }).await
}

pub async fn wallhaven_collections() -> Result<Vec<wallpicker_core::wallhaven::Collection>, String> {
    call("wallhaven_collections", &NoArgs {}).await
}

pub async fn wallhaven_collection(id: u64, page: u32) -> Result<Page, String> {
    #[derive(Serialize)]
    struct Args {
        id: u64,
        page: u32,
    }
    call("wallhaven_collection", &Args { id, page }).await
}

pub async fn find_duplicates() -> Result<Vec<wallpicker_core::duplicates::DuplicateGroup>, String> {
    call("find_duplicates", &NoArgs {}).await
}

/// What clearing a duplicate group did.
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveReport {
    pub trashed: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Keep `keep`, move `others` to the trash (their favorite and tags move
/// to it).
pub async fn resolve_duplicates(keep: &str, others: &[String]) -> Result<ResolveReport, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        keep: &'a str,
        others: &'a [String],
    }
    call("resolve_duplicates", &Args { keep, others }).await
}

/// Material 3 colors from the current wallpaper (role → "#rrggbb").
#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    pub dark: std::collections::BTreeMap<String, String>,
    pub light: std::collections::BTreeMap<String, String>,
}

pub async fn wallpaper_palette() -> Result<Option<Palette>, String> {
    call("wallpaper_palette", &NoArgs {}).await
}

/// Whether Noctalia's colors follow the wallpaper.
#[derive(Debug, Clone, Deserialize)]
pub struct NoctaliaColors {
    pub available: bool,
    pub following: bool,
    pub scheme: Option<String>,
}

pub async fn noctalia_colors() -> Result<NoctaliaColors, String> {
    call("noctalia_colors", &NoArgs {}).await
}

pub async fn set_noctalia_colors(follow: bool) -> Result<NoctaliaColors, String> {
    #[derive(Serialize)]
    struct Args {
        follow: bool,
    }
    call("set_noctalia_colors", &Args { follow }).await
}

/// Back to what each of `monitors` showed before.
pub async fn previous_wallpaper(monitors: &[String]) -> Result<ApplyReport, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        monitors: &'a [String],
    }
    call("previous_wallpaper", &Args { monitors }).await
}

/// Open the file manager on a library image, selected when it can do that.
pub async fn show_in_folder(path: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
    }
    call("show_in_folder", &Args { path }).await
}

/// Put text on the clipboard (through the backend: `wl-copy`).
pub async fn copy_text(text: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        text: &'a str,
    }
    call("copy_text", &Args { text }).await
}

pub async fn trash_wallpaper(path: &str) -> Result<(), String> {
    #[derive(Serialize)]
    struct Args<'a> {
        path: &'a str,
    }
    call("trash_wallpaper", &Args { path }).await
}
