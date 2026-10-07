//! The settings: saving them (and showing what a change means for the
//! wallpapers on screen), and the other programs that draw wallpapers.

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper;
use std::path::Path;

use wallpicker_core::settings::{self, Settings, WallpaperBackend};
use wallpicker_core::wallhaven;

use crate::{AppState, CmdResult, blocking};

/// Save `settings` as the form had them, and return them as saved
/// (normalized). A new backend shows what each monitor had; a fit that
/// changed shows at once where it applies.
pub async fn save(state: &AppState, mut settings: Settings) -> CmdResult<Settings> {
    check(&settings)?;
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

/// What can't be saved as typed: a folder that isn't an absolute path (or
/// under ~), a Wallhaven username with characters it can't have, a night
/// time that isn't HH:MM.
fn check(s: &Settings) -> CmdResult<()> {
    let placeable = |p: &Path| p.is_absolute() || p.starts_with("~");
    if let Some(bad) = s.folders.iter().find(|p| !placeable(p)) {
        return Err(format!("Folders must be absolute paths (or start with ~): {}", bad.display()));
    }
    if !placeable(&s.download_dir) {
        return Err("The download folder must be an absolute path (or start with ~).".into());
    }
    let set_folders = [&s.slideshow.folder, &s.slideshow.night.folder];
    if set_folders.iter().any(|f| f.as_deref().is_some_and(|p| !placeable(p))) {
        return Err("A slideshow folder must be an absolute path (or start with ~).".into());
    }
    if s.wallhaven_username.as_deref().is_some_and(|n| !n.trim().is_empty() && !wallhaven::is_valid_username(n.trim())) {
        return Err("That isn't a Wallhaven username (letters, digits, _ - . only).".into());
    }
    if [&s.slideshow.night.from, &s.slideshow.night.to].iter().any(|t| settings::parse_hhmm(t).is_none()) {
        return Err("The night's times must look like 20:00.".into());
    }
    Ok(())
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
