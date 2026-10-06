//! The app's own settings and looks: settings and saved state, the theme
//! following the wallpaper, the slideshow, the window and its usage.

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper::{self};
use wallpicker_core::settings::{Settings, State as SavedState, WallpaperBackend};

use crate::service;
use crate::{AppState, CmdResult, blocking, err};
use crate::{cli, open, theme, tray, usage};

/// The automation build's open-timing script reports here: printed with
/// when the window was asked for and built. Nothing in normal builds.
#[tauri::command]
pub(crate) fn open_timing(report: String) {
    #[cfg(feature = "automation")]
    {
        use std::sync::atomic::Ordering;
        println!(
            "open-timing {{\"asked\":{},\"built\":{},\"page\":{report}}}",
            crate::tray::OPEN_ASKED.load(Ordering::Relaxed),
            crate::tray::OPEN_BUILT.load(Ordering::Relaxed)
        );
    }
    #[cfg(not(feature = "automation"))]
    let _ = report;
}

#[tauri::command]
pub(crate) async fn get_settings(state: tauri::State<'_, AppState>) -> CmdResult<Settings> {
    service::app::settings(&state).await
}

#[tauri::command]
pub(crate) async fn save_settings(
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
            if let Err(e) = hyprpaper::reapply(&SystemRunner, &store, &|m, p| {
                switched || before.of(m, p) != after.of(m, p)
            }) {
                eprintln!("wallpicker: could not re-apply the wallpapers: {e}");
            }
        }
        Ok(saved)
    })
    .await
}

#[tauri::command]
pub(crate) async fn get_state(state: tauri::State<'_, AppState>) -> CmdResult<SavedState> {
    service::app::saved_state(&state).await
}

/// The interface's background color now, for the next window to start in.
#[tauri::command]
pub(crate) fn window_background(app: tauri::AppHandle, r: u8, g: u8, b: u8) {
    tray::set_background(&app, (r, g, b));
}

/// Memory and CPU right now (Settings → Performance).
#[tauri::command]
pub(crate) async fn app_usage() -> CmdResult<usage::Usage> {
    tauri::async_runtime::spawn_blocking(usage::usage)
        .await
        .map_err(err)
}

/// Where `wallpicker --open` asked the window to go, once.
#[tauri::command]
pub(crate) fn take_open_request() -> Option<open::Place> {
    open::take()
}

/// The app's colors, from the current wallpaper (None: its own).
#[tauri::command]
pub(crate) async fn wallpaper_palette(state: tauri::State<'_, AppState>) -> CmdResult<Option<theme::Palette>> {
    service::app::palette(&state).await
}

#[tauri::command]
pub(crate) async fn noctalia_colors() -> CmdResult<theme::NoctaliaColors> {
    tauri::async_runtime::spawn_blocking(theme::noctalia_colors)
        .await
        .map_err(err)
}

#[tauri::command]
pub(crate) async fn set_noctalia_colors(
    state: tauri::State<'_, AppState>,
    follow: bool,
) -> CmdResult<theme::NoctaliaColors> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || theme::set_noctalia_follow(&store, follow))
        .await
        .map_err(err)?
}

/// The slideshow's next change, for Settings.
#[tauri::command]
pub(crate) async fn slideshow_status(
    state: tauri::State<'_, AppState>,
) -> CmdResult<cli::slideshow::Status> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || cli::slideshow::status_of(&store))
        .await
        .map_err(err)?
}

/// "Change now": a slideshow change right away (the timer starts over).
#[tauri::command]
pub(crate) async fn slideshow_now(state: tauri::State<'_, AppState>) -> CmdResult<()> {
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
