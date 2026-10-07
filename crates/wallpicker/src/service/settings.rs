//! The settings: saving them (and showing what a change means for the
//! wallpapers on screen), and the other programs that draw wallpapers.

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use wallpicker_core::settings::{Settings, WallpaperBackend};

use crate::{AppState, CmdResult, blocking};

/// Save `settings` as the form had them, and return them as saved
/// (normalized). A new backend shows what each monitor had; a fit that
/// changed shows at once where it applies.
pub async fn save(state: &AppState, mut settings: Settings) -> CmdResult<Settings> {
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
        if saved.wallpaper_backend != WallpaperBackend::Noctalia {
            let switched = saved.wallpaper_backend != backend_before;
            let after = hyprpaper::Fits::load(&store);
            if let Err(e) = hyprpaper::reapply(&SystemRunner, &store, &|m, p| switched || before.of(m, p) != after.of(m, p)) {
                eprintln!("wallpicker: could not re-apply the wallpapers: {e}");
            }
        }
        Ok(saved)
    })
    .await
}

/// Other programs drawing wallpapers besides `backend` (they'd fight over
/// the screen).
pub async fn conflicts(backend: WallpaperBackend) -> CmdResult<Vec<String>> {
    blocking(move || Ok(hyprpaper::conflicts(&SystemRunner, backend))).await
}

/// Stop a program [`conflicts`] found (hyprpaper, awww-daemon...).
pub async fn stop_program(name: String) -> CmdResult<()> {
    blocking(move || hyprpaper::stop_program(&SystemRunner, &name)).await
}
