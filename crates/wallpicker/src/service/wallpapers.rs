//! What the monitors show (listing them, applying, Previous, adjusting),
//! and the wallpaper files themselves (trash, show in folder).

use std::path::PathBuf;

use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper::{self, ApplyReport};
use wallpicker_core::library;
use wallpicker_core::monitors::Monitor;
use wallpicker_core::settings::{self, Adjust, SpanLook, State};

use wallpicker_core::store::Store;

use crate::service::library::update_catalog;
use crate::{AppState, CmdResult, blocking, err, wallpaper_path};

pub async fn monitors() -> CmdResult<Vec<Monitor>> {
    blocking(|| hyprpaper::list_monitors(&SystemRunner)).await
}

/// `path` on `monitors`, remembered for `--restore` (and, with workspaces
/// remembering, for the workspace each monitor shows).
pub async fn apply(state: &AppState, monitors: Vec<String>, path: String) -> CmdResult<ApplyReport> {
    let store = state.store.clone();
    blocking(move || {
        let settings = store.load_settings()?;
        let report = hyprpaper::apply_and_remember_with(&SystemRunner, &store, settings.wallpaper_backend, &monitors, &PathBuf::from(path))?;
        wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
        Ok(report)
    })
    .await
}

/// `path` across every monitor, each showing its part (`span.rs`), as
/// `look` says (the same picture again: only its look changes).
pub async fn span(state: &AppState, path: String, look: SpanLook) -> CmdResult<ApplyReport> {
    let path = wallpaper_path(&path)?;
    let store = state.store.clone();
    let lock = state.adjust_lock.clone();
    blocking(move || {
        // Like an adjust: it makes copies and prunes the unused ones.
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        let backend = store.load_settings()?.wallpaper_backend;
        let report = hyprpaper::span_and_remember(&SystemRunner, &store, backend, &path, look)?;
        wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
        let _ = store.write_hyprpaper_conf();
        Ok(report)
    })
    .await
}

/// The monitors act on their own again, each keeping its part.
pub async fn separate(state: &AppState) -> CmdResult<State> {
    let store = state.store.clone();
    blocking(move || store.separate()).await
}

/// Back to what each of `monitors` showed before.
pub async fn previous(state: &AppState, monitors: Vec<String>) -> CmdResult<ApplyReport> {
    let store = state.store.clone();
    blocking(move || {
        let backend = store.load_settings()?.wallpaper_backend;
        let report = hyprpaper::apply_previous(&SystemRunner, &store, backend, &monitors)?;
        wallpicker_core::workspaces::remember_shown(&SystemRunner, &store, &report.applied);
        Ok(report)
    })
    .await
}

/// Adjust what `monitor` shows (mirrored...), and show it there now. Every
/// backend: the changed copy is a file of its own. Temporary: another
/// wallpaper on that monitor clears it.
pub async fn set_adjust(
    state: &AppState,
    monitor: String,
    adjust: Adjust,
) -> CmdResult<State> {
    let store = state.store.clone();
    let lock = state.adjust_lock.clone();
    blocking(move || {
        // One at a time: each ends by deleting the copies its state doesn't
        // use, which would be the other's.
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        let saved = store.set_adjust(&monitor, adjust)?;
        let report = hyprpaper::reapply(&SystemRunner, &store, &|m, _| m == monitor)?;
        if let Some((_, why)) = report.failed.first() {
            return Err(wallpicker_core::error::Error::Invalid(format!(
                "Could not show it on {monitor}: {why}"
            )));
        }
        let _ = store.write_hyprpaper_conf();
        wallpicker_core::transform::prune(&store.dirs().cache, &saved);
        Ok(saved)
    })
    .await
}

/// The picture the Adjust dialog frames on: sharp enough to zoom into
/// (fitted inside 1600 px, cached), not the whole original.
pub async fn framing_image(state: &AppState, path: String) -> CmdResult<PathBuf> {
    let path = wallpaper_path(&path)?;
    let cache = state.store.dirs().cache.join("framing");
    blocking(move || wallpicker_core::thumbs::fit_within(&cache, &path, 1600)).await
}

/// The fit modes the chosen backend does, with labels ("" is its default).
pub async fn fit_modes(state: &AppState) -> CmdResult<Vec<(&'static str, &'static str)>> {
    let store = state.store.clone();
    blocking(move || Ok(settings::fit_modes(store.load_settings()?.wallpaper_backend).to_vec())).await
}

pub async fn show_in_folder(state: &AppState, path: String) -> CmdResult<()> {
    let store = state.store.clone();
    crate::rt::spawn_blocking(move || show_in_folder_blocking(&store, &path)).await.map_err(err)?
}

/// Move a wallpaper to the trash (the picker and `wallpicker trash`).
pub async fn trash_path(state: &AppState, path: &str) -> CmdResult<()> {
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
    crate::rt::spawn_blocking(move || trash::delete(&file))
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
pub fn show_in_folder_blocking(store: &Store, path: &str) -> CmdResult<()> {
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
