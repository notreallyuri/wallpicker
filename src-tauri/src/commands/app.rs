//! The app's own settings and looks: settings and saved state, the theme
//! following the wallpaper, the slideshow, the window and its usage.

use wallpicker_core::settings::{Settings, State as SavedState};

use crate::service;
use crate::{AppState, CmdResult, err};
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
    settings: Settings,
) -> CmdResult<Settings> {
    service::settings::save(&state, settings).await
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
    service::app::noctalia_colors().await
}

#[tauri::command]
pub(crate) async fn set_noctalia_colors(
    state: tauri::State<'_, AppState>,
    follow: bool,
) -> CmdResult<theme::NoctaliaColors> {
    service::app::set_noctalia_colors(&state, follow).await
}

/// The slideshow's next change, for Settings.
#[tauri::command]
pub(crate) async fn slideshow_status(
    state: tauri::State<'_, AppState>,
) -> CmdResult<cli::slideshow::Status> {
    service::slideshow::status(&state).await
}

/// "Change now": a slideshow change right away (the timer starts over).
#[tauri::command]
pub(crate) async fn slideshow_now(state: tauri::State<'_, AppState>) -> CmdResult<()> {
    service::slideshow::now(&state).await
}
