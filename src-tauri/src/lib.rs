//! Tauri backend. Deliberately thin: every decision that can be tested without
//! a desktop session lives in `wallpicker-core`; this file wires it to the UI
//! and does the HTTP calls to Wallhaven.

pub mod backup;
pub mod cli;
pub mod open;
pub mod theme;
mod usage;
mod tray;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::Manager;
use tokio::sync::Semaphore;
use wallpicker_core::catalog::{self, Catalog};
use wallpicker_core::duplicates::DuplicateGroup;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper::{self, ApplyReport};
use wallpicker_core::library::{self, LocalImage};
use wallpicker_core::monitors::Monitor;
use wallpicker_core::settings::{Settings, State as SavedState, WallpaperBackend};
use wallpicker_core::store::{Dirs, Store};
use wallpicker_core::thumbs;
use wallpicker_core::moebooru::{self, MoebooruQuery, Site};
use wallpicker_core::online::{
    self, Details, OnlineFilters, OnlineQuery, OnlineWallpaper, Page, Provider,
};
use wallpicker_core::wallhaven::{self, API_SEARCH_URL, API_WALLPAPER_URL};

/// Largest Wallhaven download we accept. Wallpapers are a few MB; this only
/// guards against a misbehaving server.
const MAX_DOWNLOAD_BYTES: usize = 64 * 1024 * 1024;

struct AppState {
    store: Store,
    http: reqwest::Client,
    /// Decoding a big PNG is memory-hungry; the grid asks for many thumbnails
    /// at once, so only a few are generated concurrently.
    thumb_gate: Arc<Semaphore>,
    /// `library.json` is loaded, changed and saved per command; this keeps two
    /// commands from interleaving and losing one's change.
    catalog_lock: Arc<Mutex<()>>,
}

impl AppState {
    /// Shared by the app and the command line.
    fn new(store: Store) -> AppState {
        let http = reqwest::Client::builder()
            .user_agent(concat!("wallpicker/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("failed to build HTTP client");
        AppState {
            store,
            http,
            thumb_gate: Arc::new(Semaphore::new(3)),
            catalog_lock: Arc::new(Mutex::new(())),
        }
    }
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Run blocking core code off the UI thread.
async fn blocking<T, F>(f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> wallpicker_core::Result<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(err)?
        .map_err(err)
}

// ------------------------------------------------------------------ local ----

#[tauri::command]
async fn list_monitors() -> CmdResult<Vec<Monitor>> {
    blocking(|| hyprpaper::list_monitors(&SystemRunner)).await
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, AppState>) -> CmdResult<Settings> {
    let store = state.store.clone();
    blocking(move || store.load_settings()).await
}

#[tauri::command]
async fn save_settings(
    state: tauri::State<'_, AppState>,
    mut settings: Settings,
) -> CmdResult<Settings> {
    let store = state.store.clone();
    blocking(move || {
        let before = hyprpaper::Fits::load(&store);
        let on_disk = store.load_settings()?;
        let backend_before = on_disk.wallpaper_backend;
        // The snapshot folder is set from Settings → Backup on its own (it
        // moves the snapshots); a form opened before that mustn't undo it.
        settings.snapshot_dir = on_disk.snapshot_dir;
        // Workspace wallpapers change as you apply (and from their own list).
        settings.workspaces.wallpapers = on_disk.workspaces.wallpapers;
        store.save_settings(&settings)?;
        let saved = store.load_settings()?;
        // A new backend shows what each monitor had; a fit that changed shows
        // right away where it applies.
        if saved.wallpaper_backend != WallpaperBackend::Noctalia {
            let switched = saved.wallpaper_backend != backend_before;
            let after = hyprpaper::Fits::load(&store);
            if let Err(e) =
                hyprpaper::reapply(&SystemRunner, &store, &|m, p| switched || before.of(m, p) != after.of(m, p))
            {
                eprintln!("wallpicker: could not re-apply the wallpapers: {e}");
            }
        }
        Ok(saved)
    })
    .await
}

#[tauri::command]
async fn get_state(state: tauri::State<'_, AppState>) -> CmdResult<SavedState> {
    let store = state.store.clone();
    blocking(move || store.load_state()).await
}

#[tauri::command]
async fn scan_local(state: tauri::State<'_, AppState>) -> CmdResult<Vec<LocalImage>> {
    let store = state.store.clone();
    blocking(move || {
        let s = store.load_settings()?;
        Ok(library::scan(&s.folders, s.recursive))
    })
    .await
}

/// Returns the path of a cached thumbnail for `path`.
#[tauri::command]
async fn thumbnail(state: tauri::State<'_, AppState>, path: String) -> CmdResult<String> {
    let _permit = state.thumb_gate.acquire().await.map_err(err)?;
    let store = state.store.clone();
    blocking(move || {
        let width = store.load_settings()?.thumb_width;
        let src = PathBuf::from(path);
        if !library::is_supported_image(&src) {
            return Err(wallpicker_core::Error::Invalid(
                "unsupported image type".into(),
            ));
        }
        let out = thumbs::ensure_thumb(&store.dirs().cache, &src, width)?;
        Ok(out.to_string_lossy().into_owned())
    })
    .await
}

/// Apply `path` to the given monitors and remember it for `--restore`.
#[tauri::command]
async fn apply_wallpaper(
    state: tauri::State<'_, AppState>,
    monitors: Vec<String>,
    path: String,
) -> CmdResult<ApplyReport> {
    let store = state.store.clone();
    blocking(move || {
        let settings = store.load_settings()?;
        let report = hyprpaper::apply_and_remember_with(
            &SystemRunner,
            &store,
            settings.wallpaper_backend,
            &monitors,
            &PathBuf::from(path),
        )?;
        wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
        Ok(report)
    })
    .await
}

/// Back to what each of `monitors` showed before.
#[tauri::command]
async fn previous_wallpaper(state: tauri::State<'_, AppState>, monitors: Vec<String>) -> CmdResult<ApplyReport> {
    let store = state.store.clone();
    blocking(move || {
        let backend = store.load_settings()?.wallpaper_backend;
        let report = hyprpaper::apply_previous(&SystemRunner, &store, backend, &monitors)?;
        wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
        Ok(report)
    })
    .await
}

/// Hyprland's workspaces and those with a wallpaper, for Settings.
#[tauri::command]
async fn workspace_rows(state: tauri::State<'_, AppState>) -> CmdResult<Vec<wallpicker_core::workspaces::WorkspaceRow>> {
    let store = state.store.clone();
    blocking(move || wallpicker_core::workspaces::rows(&SystemRunner, &store)).await
}

/// Set or clear (`path` None) a workspace's wallpaper.
#[tauri::command]
async fn set_workspace_wallpaper(state: tauri::State<'_, AppState>, workspace: String, path: Option<String>) -> CmdResult<()> {
    let store = state.store.clone();
    let path = path.map(|p| wallpaper_path(&p)).transpose()?;
    blocking(move || wallpicker_core::workspaces::assign(&store, &workspace, path)).await
}

/// Move a wallpaper to the trash (recoverable from the file manager), and
/// forget its favorite, tags and any monitor's "current" entry for it. Only
/// images inside the wallpaper or download folders are accepted.
#[tauri::command]
async fn trash_wallpaper(state: tauri::State<'_, AppState>, path: String) -> CmdResult<()> {
    trash_path(&state, &path).await
}

/// [`trash_wallpaper`] without Tauri (the app and `wallpicker trash`).
async fn trash_path(state: &AppState, path: &str) -> CmdResult<()> {
    let given = wallpaper_path(path)?;
    let store = state.store.clone();
    let real = blocking(move || {
        let settings = store.load_settings()?;
        let mut roots = settings.folders.clone();
        roots.push(settings.download_dir.clone());
        library::trash_target(&given, &roots)
    })
    .await?;

    let file = real.clone();
    tauri::async_runtime::spawn_blocking(move || trash::delete(&file))
        .await
        .map_err(err)?
        .map_err(|e| format!("Could not move it to the trash: {e}"))?;

    // The scan's path and the resolved one can differ (a symlinked folder).
    let (given, resolved) = (PathBuf::from(path), real);
    let forget = (given.clone(), resolved.clone());
    update_catalog(state, move |c| {
        c.forget(&forget.0);
        c.forget(&forget.1);
    })
    .await?;
    let store = state.store.clone();
    blocking(move || {
        store.forget_current(&given)?;
        store.forget_current(&resolved)
    })
    .await
}

/// Show a library image in the file manager: selected, through the standard
/// FileManager1 D-Bus call (Thunar, Nautilus, Dolphin, Nemo...), or else its
/// folder opened with `xdg-open`. Shared by the app and the command line.
fn show_in_folder_blocking(store: &Store, path: &str) -> CmdResult<()> {
    let given = wallpaper_path(path)?;
    let settings = store.load_settings().map_err(err)?;
    let mut roots = settings.folders.clone();
    roots.push(settings.download_dir.clone());
    let real = library::library_file(&given, &roots).map_err(err)?;

    let shown = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--print-reply",
            "--reply-timeout=3000",
            "--dest=org.freedesktop.FileManager1",
            "--type=method_call",
            "/org/freedesktop/FileManager1",
            "org.freedesktop.FileManager1.ShowItems",
        ])
        .arg(format!("array:string:{}", library::file_uri(&real)))
        .arg("string:")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if shown {
        return Ok(());
    }
    let dir = real.parent().ok_or("it has no folder")?;
    let mut child = std::process::Command::new("xdg-open")
        .arg(dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not open the folder: {e}"))?;
    // Reaped in the background, so it doesn't linger as a zombie.
    std::thread::spawn(move || child.wait());
    Ok(())
}

#[tauri::command]
async fn show_in_folder(state: tauri::State<'_, AppState>, path: String) -> CmdResult<()> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || show_in_folder_blocking(&store, &path))
        .await
        .map_err(err)?
}

/// Put `text` on the Wayland clipboard (`wl-copy`): the webview's own
/// clipboard API isn't reliable in WebKitGTK.
#[tauri::command]
async fn copy_text(text: String) -> CmdResult<()> {
    if text.len() > 64 * 1024 {
        return Err("That's too long to copy.".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        use std::io::Write as _;
        let mut child = std::process::Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("Could not run wl-copy (is wl-clipboard installed?): {e}"))?;
        child
            .stdin
            .take()
            .ok_or("wl-copy has no input")?
            .write_all(text.as_bytes())
            .map_err(err)?;
        // wl-copy forks to serve the clipboard; the first process exits soon.
        match child.wait() {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("wl-copy failed ({s})")),
            Err(e) => Err(err(e)),
        }
    })
    .await
    .map_err(err)?
}

// ---------------------------------------------------------------- catalog ----

/// Change the catalog under the lock and return the saved version.
async fn update_catalog(
    state: &AppState,
    change: impl FnOnce(&mut Catalog) + Send + 'static,
) -> CmdResult<Catalog> {
    let store = state.store.clone();
    let lock = state.catalog_lock.clone();
    blocking(move || {
        // A panic elsewhere while holding the lock doesn't make the file bad.
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        store.update_catalog(change)
    })
    .await
}

fn wallpaper_path(path: &str) -> CmdResult<PathBuf> {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        Ok(p)
    } else {
        Err(format!("Not an absolute path: {path:?}"))
    }
}

#[tauri::command]
async fn get_catalog(state: tauri::State<'_, AppState>) -> CmdResult<Catalog> {
    let store = state.store.clone();
    blocking(move || store.load_catalog()).await
}

#[tauri::command]
async fn set_favorite(
    state: tauri::State<'_, AppState>,
    path: String,
    favorite: bool,
) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    update_catalog(&state, move |c| c.set_favorite(&path, favorite)).await
}

/// The real tags a Konachan or yande.re search means (they match exact tag
/// names only): several words tried as one tag, and the last word anywhere
/// in a tag; most used first. Wallhaven searches loosely: nothing to suggest.
async fn tag_suggestions_for(state: &AppState, provider: Provider, text: &str) -> CmdResult<Vec<moebooru::TagSuggestion>> {
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
        let req = state
            .http
            .get(site.tags_json_url())
            .query(&[("name", pattern.as_str()), ("order", "count"), ("limit", "8")]);
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
async fn tag_suggestions(
    state: tauri::State<'_, AppState>,
    provider: Provider,
    text: String,
) -> CmdResult<Vec<moebooru::TagSuggestion>> {
    tag_suggestions_for(&state, provider, &text).await
}

/// Your Wallhaven collections: they need the API key (private ones too).
async fn wallhaven_collections_of(state: &AppState) -> CmdResult<Vec<wallhaven::Collection>> {
    let Some(key) = wallhaven_key(state).await? else {
        return Err("Your collections need a Wallhaven API key (Settings → Wallhaven).".into());
    };
    let req = state.http.get(wallhaven::API_COLLECTIONS_URL).header("X-API-Key", key);
    let text = get_text(Provider::Wallhaven, req).await?;
    wallhaven::parse_collections(&text).map_err(err)
}

/// A page of one of your collections. Opening one needs your username too:
/// the API doesn't tell whose key it is.
async fn wallhaven_collection_page(state: &AppState, id: u64, page: u32) -> CmdResult<Page> {
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

#[tauri::command]
async fn wallhaven_collections(state: tauri::State<'_, AppState>) -> CmdResult<Vec<wallhaven::Collection>> {
    wallhaven_collections_of(&state).await
}

#[tauri::command]
async fn wallhaven_collection(state: tauri::State<'_, AppState>, id: u64, page: u32) -> CmdResult<Page> {
    wallhaven_collection_page(&state, id, page).await
}

/// The library's duplicate groups, best first in each.
async fn duplicates(state: &AppState) -> CmdResult<Vec<DuplicateGroup>> {
    let store = state.store.clone();
    blocking(move || {
        let settings = store.load_settings()?;
        let images = library::scan(&settings.folders, settings.recursive);
        let catalog = store.load_catalog().unwrap_or_default();
        Ok(wallpicker_core::duplicates::find(&images, &catalog, &store.dirs().cache, settings.thumb_width))
    })
    .await
}

/// Every library image's colors (computed once, cached), by path.
pub fn color_profiles(
    store: &Store,
) -> wallpicker_core::error::Result<std::collections::BTreeMap<PathBuf, wallpicker_core::colors::ColorProfile>> {
    let settings = store.load_settings()?;
    let images = library::scan(&settings.folders, settings.recursive);
    Ok(wallpicker_core::colors::profiles(&images, &store.dirs().cache, settings.thumb_width))
}

#[tauri::command]
async fn image_colors(
    state: tauri::State<'_, AppState>,
) -> CmdResult<std::collections::BTreeMap<PathBuf, wallpicker_core::colors::ColorProfile>> {
    let store = state.store.clone();
    blocking(move || color_profiles(&store)).await
}

#[tauri::command]
async fn find_duplicates(state: tauri::State<'_, AppState>) -> CmdResult<Vec<DuplicateGroup>> {
    duplicates(&state).await
}

#[derive(Debug, Default, serde::Serialize)]
pub struct ResolveReport {
    pub trashed: Vec<String>,
    /// (path, why)
    pub failed: Vec<(String, String)>,
    /// Monitors that showed a trashed copy and now show the kept one.
    pub reapplied: Vec<String>,
}

/// Keep `keep`, move `others` to the trash. The kept one gets their
/// favorite, tags and fit (when it has none), and goes on any monitor that
/// showed one of them. Each copy is trashed with the usual checks; one that
/// fails doesn't stop the rest.
async fn resolve_duplicates_with(state: &AppState, keep: &str, others: &[String]) -> CmdResult<ResolveReport> {
    let keep_path = wallpaper_path(keep)?;
    let store = state.store.clone();
    let check = keep_path.clone();
    blocking(move || {
        let s = store.load_settings()?;
        let mut roots = s.folders.clone();
        roots.push(s.download_dir.clone());
        library::library_file(&check, &roots)
    })
    .await?;
    let other_paths: Vec<PathBuf> = others.iter().map(|o| wallpaper_path(o)).collect::<CmdResult<_>>()?;
    if other_paths.contains(&keep_path) {
        return Err("The kept wallpaper can't also be trashed.".into());
    }

    let (k, o) = (keep_path.clone(), other_paths.clone());
    update_catalog(state, move |c| {
        let favorite = o.iter().any(|p| c.is_favorite(p));
        let tags: Vec<String> = o.iter().flat_map(|p| c.tags(p).to_vec()).collect();
        if favorite {
            c.set_favorite(&k, true);
        }
        for t in &tags {
            c.add_tag(&k, t);
        }
    })
    .await?;

    let shown: Vec<String> = state
        .store
        .load_state()
        .map_err(err)?
        .current
        .iter()
        .filter(|(_, p)| other_paths.contains(p))
        .map(|(m, _)| m.clone())
        .collect();

    let mut report = ResolveReport::default();
    for o in others {
        match trash_path(state, o).await {
            Ok(()) => report.trashed.push(o.clone()),
            Err(e) => report.failed.push((o.clone(), e)),
        }
    }
    if !shown.is_empty() {
        let store = state.store.clone();
        let keep = keep_path.clone();
        let applied = blocking(move || {
            let backend = store.load_settings()?.wallpaper_backend;
            hyprpaper::apply_and_remember_with(&SystemRunner, &store, backend, &shown, &keep)
        })
        .await?;
        report.reapplied = applied.applied;
    }
    Ok(report)
}

#[tauri::command]
async fn resolve_duplicates(
    state: tauri::State<'_, AppState>,
    keep: String,
    others: Vec<String>,
) -> CmdResult<ResolveReport> {
    resolve_duplicates_with(&state, &keep, &others).await
}

/// A native file dialog over the window: what was picked, or None.
fn file_dialog(app: &tauri::AppHandle) -> tauri_plugin_dialog::FileDialogBuilder<tauri::Wry> {
    use tauri_plugin_dialog::DialogExt;
    let mut d = app.dialog().file();
    if let Some(w) = app.get_webview_window("main") {
        d = d.set_parent(&w);
    }
    d
}

fn picked(p: Option<tauri_plugin_dialog::FilePath>) -> Option<PathBuf> {
    p.and_then(|f| f.into_path().ok())
}

/// Export the library to a file picked in a save dialog (None: cancelled).
#[tauri::command]
async fn backup_export_pick(app: tauri::AppHandle, state: tauri::State<'_, AppState>) -> CmdResult<Option<backup::Exported>> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let suggested = backup::default_export_path();
        let mut d = file_dialog(&app).set_title("Export the wallpicker library").add_filter("wallpicker backup", &["json"]);
        if let (Some(dir), Some(name)) = (suggested.parent(), suggested.file_name()) {
            d = d.set_directory(dir).set_file_name(name.to_string_lossy());
        }
        let Some(path) = picked(d.blocking_save_file()) else { return Ok(None) };
        backup::export(&store, Some(&path.to_string_lossy())).map(Some)
    })
    .await
    .map_err(err)?
}

/// Import a backup picked in an open dialog (None: cancelled).
#[tauri::command]
async fn backup_import_pick(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    replace: bool,
) -> CmdResult<Option<wallpicker_core::backup::ImportReport>> {
    let store = state.store.clone();
    let lock = state.catalog_lock.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        let d = file_dialog(&app)
            .set_title("Import a wallpicker backup")
            .add_filter("wallpicker backup", &["json"])
            .set_directory(home);
        let Some(path) = picked(d.blocking_pick_file()) else { return Ok(None) };
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        backup::import(&store, &path.to_string_lossy(), replace).map(Some)
    })
    .await
    .map_err(err)?
}

/// Where the daily snapshots go now.
#[tauri::command]
async fn snapshot_dir(state: tauri::State<'_, AppState>) -> CmdResult<PathBuf> {
    let store = state.store.clone();
    blocking(move || Ok(store.snapshots_dir())).await
}

/// Pick another folder for the snapshots (copying them there); None:
/// cancelled. `reset`: back to the default folder, no dialog.
#[tauri::command]
async fn snapshot_dir_pick(app: tauri::AppHandle, state: tauri::State<'_, AppState>, reset: bool) -> CmdResult<Option<PathBuf>> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let dir = if reset {
            None
        } else {
            let d = file_dialog(&app).set_title("A folder for the daily snapshots").set_directory(store.snapshots_dir());
            match picked(d.blocking_pick_folder()) {
                Some(p) => Some(p),
                None => return Ok(None),
            }
        };
        store.set_snapshots_dir(dir).map(Some).map_err(|e| e.to_string())
    })
    .await
    .map_err(err)?
}

#[tauri::command]
async fn backup_snapshots(state: tauri::State<'_, AppState>) -> CmdResult<Vec<wallpicker_core::store::Snapshot>> {
    let store = state.store.clone();
    blocking(move || store.snapshots()).await
}

#[tauri::command]
async fn backup_restore(state: tauri::State<'_, AppState>, id: String) -> CmdResult<Catalog> {
    let store = state.store.clone();
    let lock = state.catalog_lock.clone();
    blocking(move || {
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        store.restore_snapshot(&id)
    })
    .await
}

/// The interface's background color now, for the next window to start in.
#[tauri::command]
fn window_background(app: tauri::AppHandle, r: u8, g: u8, b: u8) {
    tray::set_background(&app, (r, g, b));
}

/// The image for an online wallpaper's details: downloaded, cached and fitted
/// to the screen (see `cli::online::fetch_preview`); None when it can't be.
#[tauri::command]
async fn online_preview(state: tauri::State<'_, AppState>, wallpaper: OnlineWallpaper) -> CmdResult<Option<PathBuf>> {
    Ok(cli::online::fetch_preview(&state, &wallpaper).await)
}

/// Memory and CPU right now (Settings → Performance).
#[tauri::command]
async fn app_usage() -> CmdResult<usage::Usage> {
    tauri::async_runtime::spawn_blocking(usage::usage).await.map_err(err)
}

/// Where `wallpicker --open` asked the window to go, once.
#[tauri::command]
fn take_open_request() -> Option<open::Place> {
    open::take()
}

/// Other programs drawing wallpapers besides the chosen backend (they'd
/// fight over the screen), for Settings.
#[tauri::command]
async fn wallpaper_conflicts(backend: WallpaperBackend) -> CmdResult<Vec<String>> {
    blocking(move || Ok(hyprpaper::conflicts(&SystemRunner, backend))).await
}

/// Stop a program [`wallpaper_conflicts`] found (hyprpaper, awww-daemon...).
#[tauri::command]
async fn stop_wallpaper_program(name: String) -> CmdResult<()> {
    blocking(move || Ok(hyprpaper::stop_program(&SystemRunner, &name)?)).await
}

/// The app's colors, from the current wallpaper (None: its own).
#[tauri::command]
async fn wallpaper_palette(state: tauri::State<'_, AppState>) -> CmdResult<Option<theme::Palette>> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || theme::palette(&store)).await.map_err(err)?
}

#[tauri::command]
async fn noctalia_colors() -> CmdResult<theme::NoctaliaColors> {
    tauri::async_runtime::spawn_blocking(theme::noctalia_colors).await.map_err(err)
}

#[tauri::command]
async fn set_noctalia_colors(state: tauri::State<'_, AppState>, follow: bool) -> CmdResult<theme::NoctaliaColors> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || theme::set_noctalia_follow(&store, follow)).await.map_err(err)?
}

/// The slideshow's next change, for Settings.
#[tauri::command]
async fn slideshow_status(state: tauri::State<'_, AppState>) -> CmdResult<cli::slideshow::Status> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || cli::slideshow::status_of(&store))
        .await
        .map_err(err)?
}

/// "Change now": a slideshow change right away (the timer starts over).
#[tauri::command]
async fn slideshow_now(state: tauri::State<'_, AppState>) -> CmdResult<()> {
    let store = state.store.clone();
    match tauri::async_runtime::spawn_blocking(move || cli::slideshow::tick_with(&store, true))
        .await
        .map_err(err)?
    {
        Ok(true) => Ok(()),
        Ok(false) => Err("Some monitors could not be changed.".into()),
        Err(e) => Err(e),
    }
}

/// Adjust what `monitor` shows (mirrored...), and show it there now. Every
/// backend: the changed copy is a file of its own. Temporary: another
/// wallpaper on that monitor clears it.
pub(crate) async fn set_adjust(
    state: &AppState,
    monitor: String,
    adjust: wallpicker_core::settings::Adjust,
) -> CmdResult<SavedState> {
    let store = state.store.clone();
    blocking(move || {
        let saved = store.set_adjust(&monitor, adjust)?;
        let report = hyprpaper::reapply(&SystemRunner, &store, &|m, _| m == monitor)?;
        if let Some((_, why)) = report.failed.first() {
            return Err(wallpicker_core::error::Error::Invalid(format!("Could not show it on {monitor}: {why}")));
        }
        let _ = store.write_hyprpaper_conf();
        wallpicker_core::transform::prune(&store.dirs().cache, &saved);
        Ok(saved)
    })
    .await
}

/// The picture the Adjust dialog frames on: sharp enough to zoom into
/// (fitted inside 1600 px, cached), not the whole original.
#[tauri::command]
async fn framing_image(state: tauri::State<'_, AppState>, path: String) -> CmdResult<PathBuf> {
    let path = wallpaper_path(&path)?;
    let cache = state.store.dirs().cache.join("framing");
    blocking(move || wallpicker_core::thumbs::fit_within(&cache, &path, 1600)).await
}

#[tauri::command]
async fn set_monitor_adjust(
    state: tauri::State<'_, AppState>,
    monitor: String,
    adjust: wallpicker_core::settings::Adjust,
) -> CmdResult<SavedState> {
    set_adjust(&state, monitor, adjust).await
}

#[tauri::command]
async fn add_tag(state: tauri::State<'_, AppState>, path: String, tag: String) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    if catalog::normalize_tag(&tag).is_none() {
        return Err(format!(
            "Tags must be 1 to {} characters, without control characters.",
            catalog::MAX_TAG_LEN
        ));
    }
    update_catalog(&state, move |c| {
        c.add_tag(&path, &tag);
    })
    .await
}

#[tauri::command]
async fn remove_tag(
    state: tauri::State<'_, AppState>,
    path: String,
    tag: String,
) -> CmdResult<Catalog> {
    let path = wallpaper_path(&path)?;
    update_catalog(&state, move |c| c.remove_tag(&path, &tag)).await
}

// ----------------------------------------------------------------- online ----

fn http_error(provider: Provider, status: reqwest::StatusCode) -> String {
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

async fn wallhaven_key(state: &AppState) -> CmdResult<Option<String>> {
    let store = state.store.clone();
    blocking(move || Ok(store.load_settings()?.wallhaven_api_key)).await
}

async fn get_text(provider: Provider, req: reqwest::RequestBuilder) -> CmdResult<String> {
    let resp = req.send().await.map_err(err)?;
    if !resp.status().is_success() {
        return Err(http_error(provider, resp.status()));
    }
    resp.text().await.map_err(err)
}

#[tauri::command]
async fn online_search(state: tauri::State<'_, AppState>, query: OnlineQuery) -> CmdResult<Page> {
    search(&state, query).await
}

/// One page of results from any provider (the app and `wallpicker search`).
async fn search(state: &AppState, query: OnlineQuery) -> CmdResult<Page> {
    match query {
        OnlineQuery::Wallhaven(q) => {
            let key = wallhaven_key(&state).await?;
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
            let store = state.store.clone();
            let site = Site::from(blocking(move || Ok(store.load_settings()?.konachan_site)).await?);
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

/// The filters from last session (without search text), for every provider.
#[tauri::command]
async fn get_online_filters(state: tauri::State<'_, AppState>) -> CmdResult<OnlineFilters> {
    let store = state.store.clone();
    blocking(move || store.load_online_filters()).await
}

#[tauri::command]
async fn save_online_filters(
    state: tauri::State<'_, AppState>,
    filters: OnlineFilters,
) -> CmdResult<()> {
    let store = state.store.clone();
    blocking(move || store.save_online_filters(&filters)).await
}

/// Uploader, source and tags. Some providers include them in the search
/// result; Wallhaven needs a request.
#[tauri::command]
async fn online_details(
    state: tauri::State<'_, AppState>,
    wallpaper: OnlineWallpaper,
) -> CmdResult<Details> {
    details(&state, &wallpaper).await
}

async fn details(state: &AppState, wallpaper: &OnlineWallpaper) -> CmdResult<Details> {
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

/// Open a web link (a wallpaper's source or page) in the browser.
#[tauri::command]
fn open_url(url: String) -> CmdResult<()> {
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
async fn online_downloaded(state: tauri::State<'_, AppState>) -> CmdResult<Vec<String>> {
    let store = state.store.clone();
    blocking(move || downloaded_keys(&store)).await
}

fn downloaded_keys(store: &Store) -> wallpicker_core::Result<Vec<String>> {
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
async fn online_download(
    state: tauri::State<'_, AppState>,
    wallpaper: OnlineWallpaper,
) -> CmdResult<String> {
    download_and_tag(&state, &wallpaper).await
}

/// [`online_download`] without Tauri (the app and `wallpicker apply-online`).
async fn download_and_tag(state: &AppState, wallpaper: &OnlineWallpaper) -> CmdResult<String> {
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
async fn tag_artists(state: &AppState, wallpaper: &OnlineWallpaper, path: &Path) -> CmdResult<()> {
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
async fn download(state: &AppState, wallpaper: &OnlineWallpaper) -> CmdResult<(PathBuf, bool)> {
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

// -------------------------------------------------------------------- app ----

/// `wallpicker --restore`: re-apply the saved wallpapers and exit.
/// Returns the process exit code.
pub fn restore_cli() -> i32 {
    let store = Store::new(Dirs::from_env());
    let backend = store
        .load_settings()
        .map(|s| s.wallpaper_backend)
        .unwrap_or_default();
    if backend == WallpaperBackend::Noctalia {
        println!("wallpicker uses Noctalia, which restores its wallpapers itself; nothing to do");
        return 0;
    }
    // Retry for ~20s: at login hyprpaper may still be starting.
    match hyprpaper::restore(&SystemRunner, &store, 40, Duration::from_millis(500)) {
        Ok(report) => {
            for m in &report.applied {
                println!("restored {m}");
            }
            for (m, why) in &report.skipped {
                eprintln!("skipped {m}: {why}");
            }
            0
        }
        Err(e) => {
            eprintln!("wallpicker: {e}");
            1
        }
    }
}

/// A development build (plain `cargo build`) loads the UI from the dev
/// server instead of embedding it; without `cargo tauri dev` running, its
/// window would just say "connection refused". Say what's going on instead.
#[cfg(dev)]
fn require_dev_server(context: &tauri::Context) {
    use std::net::{TcpStream, ToSocketAddrs};
    let Some(url) = context.config().build.dev_url.as_ref() else {
        return;
    };
    let host = url.host_str().unwrap_or("localhost");
    let port = url.port_or_known_default().unwrap_or(80);
    let reachable = (host, port).to_socket_addrs().is_ok_and(|mut addrs| {
        addrs.any(|a| TcpStream::connect_timeout(&a, std::time::Duration::from_millis(300)).is_ok())
    });
    if !reachable {
        eprintln!(
            "wallpicker: this is a development build; its window loads the UI from {url},\n\
             which isn't running. Use `cargo tauri dev` while developing, or build the app with\n\
             `cargo tauri build --no-bundle` (that embeds the UI). `wallpicker list` and\n\
             `wallpicker apply` work in either build."
        );
        std::process::exit(1);
    }
}

/// glibc's malloc keeps freed memory for reuse: big blocks once its dynamic
/// mmap threshold has grown (to 32 MB), and one pool per thread (the app has
/// dozens). After a few full-size images that's hundreds of MB held while the
/// app sits in the tray. A fixed 1 MB threshold sends big blocks straight
/// back to the system when freed, and two pools are plenty for a UI.
fn tune_malloc() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    // SAFETY: mallopt only adjusts allocator parameters; called once at startup.
    unsafe {
        libc::mallopt(libc::M_MMAP_THRESHOLD, 1024 * 1024);
        libc::mallopt(libc::M_ARENA_MAX, 2);
    }
}

/// The app. `start_hidden` (`wallpicker --tray`): only the tray icon at
/// first, for an autostart.
pub fn run(start_hidden: bool) {
    tune_malloc();
    let context = tauri::generate_context!();
    #[cfg(dev)]
    require_dev_server(&context);
    let dirs = Dirs::from_env();
    let cache = dirs.cache.clone();

    let store = Store::new(dirs);
    // hyprpaper.conf sources this file; make sure it exists (and matches the
    // saved state) before anything else, even if nothing gets applied.
    if let Err(e) = store.write_hyprpaper_conf() {
        eprintln!("wallpicker: could not write the hyprpaper config: {e}");
    }

    tauri::Builder::default()
        // First: a second `wallpicker` hands over to this one and exits.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            tray::second_launch(app, &argv, std::path::Path::new(&cwd))
        }))
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(store.clone()))
        .setup(move |app| {
            // Without a tray (nothing to show it in), the app just runs as a
            // window; closing it quits.
            if let Err(e) = tray::setup(app.handle()) {
                eprintln!("wallpicker: no tray icon: {e}");
            }
            // The window isn't made at startup (tauri.conf: `create: false`);
            // `--tray` leaves it unmade until it's wanted.
            if !start_hidden {
                tray::show(app.handle());
            }
            // Thumbnails are shown through Tauri's asset protocol, which only
            // serves allow-listed folders. The cache location depends on
            // XDG_CACHE_HOME, so it is allowed here instead of in tauri.conf.
            std::fs::create_dir_all(&cache)?;
            app.asset_protocol_scope().allow_directory(&cache, true)?;
            // A wallpaper per workspace: follows Hyprland while the app runs
            // (waits while it's off, or while `wallpicker workspaces run` has it).
            let handle = app.handle().clone();
            let listener_store = store.clone();
            std::thread::spawn(move || {
                use tauri::Emitter;
                wallpicker_core::workspaces::listen(&SystemRunner, &listener_store, &|| true, &|| {
                    let _ = handle.emit("state-changed", ());
                });
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_monitors,
            get_settings,
            save_settings,
            get_state,
            scan_local,
            thumbnail,
            apply_wallpaper,
            online_search,
            get_online_filters,
            save_online_filters,
            online_details,
            online_download,
            online_downloaded,
            open_url,
            get_catalog,
            set_favorite,
            add_tag,
            remove_tag,
            trash_wallpaper,
            show_in_folder,
            copy_text,
            slideshow_status,
            slideshow_now,
            find_duplicates,
            image_colors,
            previous_wallpaper,
            wallpaper_palette,
            take_open_request,
            set_monitor_adjust,
            workspace_rows,
            set_workspace_wallpaper,
            framing_image,
            app_usage,
            online_preview,
            window_background,
            backup_export_pick,
            backup_import_pick,
            snapshot_dir,
            snapshot_dir_pick,
            backup_snapshots,
            backup_restore,
            wallpaper_conflicts,
            stop_wallpaper_program,
            noctalia_colors,
            set_noctalia_colors,
            wallhaven_collections,
            tag_suggestions,
            wallhaven_collection,
            resolve_duplicates,
        ])
        .build(context)
        .expect("error while building wallpicker")
        .run(|app, event| {
            // Closing the last window would end the app; with the tray it
            // goes on (the window is made again when wanted). Quitting from
            // the tray menu asks with an exit code, and isn't stopped.
            if let tauri::RunEvent::ExitRequested { code: None, api, .. } = event
                && tray::hides_on_close(app)
            {
                api.prevent_exit();
            }
        });
}
