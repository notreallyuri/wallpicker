//! What the monitors show: applying, Previous, adjusting, workspaces, and the
//! wallpaper files themselves (trash, show in folder).

use std::path::PathBuf;
use wallpicker_core::cmd::SystemRunner;
use wallpicker_core::hyprpaper::{self, ApplyReport};
use wallpicker_core::library;
use wallpicker_core::monitors::Monitor;
use wallpicker_core::settings::{State as SavedState, WallpaperBackend};
use wallpicker_core::store::Store;

use crate::commands::library::update_catalog;
use crate::{AppState, CmdResult, blocking, err, wallpaper_path};

#[tauri::command]
pub(crate) async fn list_monitors() -> CmdResult<Vec<Monitor>> {
    blocking(|| hyprpaper::list_monitors(&SystemRunner)).await
}

/// Apply `path` to the given monitors and remember it for `--restore`.
#[tauri::command]
pub(crate) async fn apply_wallpaper(
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
pub(crate) async fn previous_wallpaper(
    state: tauri::State<'_, AppState>,
    monitors: Vec<String>,
) -> CmdResult<ApplyReport> {
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
pub(crate) async fn workspace_rows(
    state: tauri::State<'_, AppState>,
) -> CmdResult<Vec<wallpicker_core::workspaces::WorkspaceRow>> {
    let store = state.store.clone();
    blocking(move || wallpicker_core::workspaces::rows(&SystemRunner, &store)).await
}

/// Set or clear (`path` None) a workspace's wallpaper.
#[tauri::command]
pub(crate) async fn set_workspace_wallpaper(
    state: tauri::State<'_, AppState>,
    workspace: String,
    path: Option<String>,
) -> CmdResult<()> {
    let store = state.store.clone();
    let path = path.map(|p| wallpaper_path(&p)).transpose()?;
    blocking(move || wallpicker_core::workspaces::assign(&store, &workspace, path)).await
}

/// Move a wallpaper to the trash (recoverable from the file manager), and
/// forget its favorite, tags and any monitor's "current" entry for it. Only
/// images inside the wallpaper or download folders are accepted.
#[tauri::command]
pub(crate) async fn trash_wallpaper(
    state: tauri::State<'_, AppState>,
    path: String,
) -> CmdResult<()> {
    trash_path(&state, &path).await
}

/// [`trash_wallpaper`] without Tauri (the app and `wallpicker trash`).
pub(crate) async fn trash_path(state: &AppState, path: &str) -> CmdResult<()> {
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
pub(crate) fn show_in_folder_blocking(store: &Store, path: &str) -> CmdResult<()> {
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
pub(crate) async fn show_in_folder(
    state: tauri::State<'_, AppState>,
    path: String,
) -> CmdResult<()> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || show_in_folder_blocking(&store, &path))
        .await
        .map_err(err)?
}

/// Put `text` on the Wayland clipboard (`wl-copy`): the webview's own
/// clipboard API isn't reliable in WebKitGTK.
#[tauri::command]
pub(crate) async fn copy_text(text: String) -> CmdResult<()> {
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

/// Other programs drawing wallpapers besides the chosen backend (they'd
/// fight over the screen), for Settings.
#[tauri::command]
pub(crate) async fn wallpaper_conflicts(backend: WallpaperBackend) -> CmdResult<Vec<String>> {
    blocking(move || Ok(hyprpaper::conflicts(&SystemRunner, backend))).await
}

/// Stop a program [`wallpaper_conflicts`] found (hyprpaper, awww-daemon...).
#[tauri::command]
pub(crate) async fn stop_wallpaper_program(name: String) -> CmdResult<()> {
    blocking(move || Ok(hyprpaper::stop_program(&SystemRunner, &name)?)).await
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
#[tauri::command]
pub(crate) async fn framing_image(
    state: tauri::State<'_, AppState>,
    path: String,
) -> CmdResult<PathBuf> {
    let path = wallpaper_path(&path)?;
    let cache = state.store.dirs().cache.join("framing");
    blocking(move || wallpicker_core::thumbs::fit_within(&cache, &path, 1600)).await
}

#[tauri::command]
pub(crate) async fn set_monitor_adjust(
    state: tauri::State<'_, AppState>,
    monitor: String,
    adjust: wallpicker_core::settings::Adjust,
) -> CmdResult<SavedState> {
    set_adjust(&state, monitor, adjust).await
}
