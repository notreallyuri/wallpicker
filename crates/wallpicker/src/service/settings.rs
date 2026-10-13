//! The settings: saving them (and showing what a change means for the
//! wallpapers on screen), and the other programs that draw wallpapers.

use std::path::Path;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::wallpapers;

use wallpicker_core::settings::{self, Settings};
use wallpicker_core::wallhaven;

use crate::{AppState, CmdResult, blocking};

/// Save `settings` as the form had them, and return them as saved
/// (normalized). Changed fits are applied immediately.
pub async fn save(state: &AppState, mut settings: Settings) -> CmdResult<Settings> {
    check(&settings)?;
    let store = state.store.clone();
    blocking(move || {
        let before = wallpapers::Fits::load(&store);
        let on_disk = store.load_settings()?;
        // The snapshot folder is set from Settings → Backup on its own (it
        // moves the snapshots); a form opened before that mustn't undo it.
        settings.snapshot_dir = on_disk.snapshot_dir;
        // Workspace wallpapers change as you apply (and from their own list).
        settings.workspaces.wallpapers = on_disk.workspaces.wallpapers;
        // Pausing acts at once (pause_animation); a form opened before mustn't undo it.
        settings.animation.paused = on_disk.animation.paused;
        store.save_settings(&settings)?;
        let saved = store.load_settings()?;
        let after = wallpapers::Fits::load(&store);
        if let Err(e) = wallpapers::reapply(&SystemRunner, &store, &|m, p| before.of(m, p) != after.of(m, p)) {
            eprintln!("wallpicker: could not re-apply the wallpapers: {e}");
        }
        Ok(saved)
    })
    .await
}

/// Pause or resume every animated wallpaper; the daemon follows at once.
pub async fn pause_animation(state: &AppState, paused: bool) -> CmdResult<Settings> {
    let store = state.store.clone();
    blocking(move || {
        let mut settings = store.load_settings()?;
        settings.animation.paused = paused;
        store.save_settings(&settings)?;
        store.load_settings()
    })
    .await
}

/// What can't be saved as typed: a folder that isn't an absolute path (or
/// under ~), a Wallhaven username with characters it can't have, a night
/// time that isn't HH:MM.
fn check(s: &Settings) -> CmdResult<()> {
    let placeable = |p: &Path| p.is_absolute() || p.starts_with("~");
    if let Some(bad) = s.folders.iter().find(|p| !placeable(p)) {
        return Err(format!(
            "Folders must be absolute paths (or start with ~): {}",
            bad.display()
        ));
    }
    if !placeable(&s.download_dir) {
        return Err("The download folder must be an absolute path (or start with ~).".into());
    }
    let set_folders = [&s.slideshow.folder, &s.slideshow.night.folder];
    if set_folders.iter().any(|f| f.as_deref().is_some_and(|p| !placeable(p))) {
        return Err("A slideshow folder must be an absolute path (or start with ~).".into());
    }
    if s.animation.max_fps > wallpicker_core::settings::Animation::MAX_FPS {
        return Err("The animation frame rate goes up to 240 (0: the display's).".into());
    }
    if s.wallhaven_username
        .as_deref()
        .is_some_and(|n| !n.trim().is_empty() && !wallhaven::is_valid_username(n.trim()))
    {
        return Err("That isn't a Wallhaven username (letters, digits, _ - . only).".into());
    }
    if [&s.slideshow.night.from, &s.slideshow.night.to]
        .iter()
        .any(|t| settings::parse_hhmm(t).is_none())
    {
        return Err("The night's times must look like 20:00.".into());
    }
    Ok(())
}

/// Other programs drawing wallpapers besides wallpicker (they'd fight over
/// the screen).
pub async fn conflicts() -> CmdResult<Vec<String>> {
    blocking(move || Ok(wallpapers::conflicts(&SystemRunner))).await
}

/// Stop a program [`conflicts`] found (hyprpaper, awww-daemon...).
pub async fn stop_program(name: String) -> CmdResult<()> {
    blocking(move || wallpapers::stop_program(&SystemRunner, &name)).await
}
