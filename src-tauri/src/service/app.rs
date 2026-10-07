//! The app's own data: settings, saved state, the palette; the clipboard.

use wallpicker_core::settings::{Settings, State as SavedState};

use crate::{AppState, CmdResult, blocking, err, theme};

pub(crate) async fn settings(state: &AppState) -> CmdResult<Settings> {
    let store = state.store.clone();
    blocking(move || store.load_settings()).await
}

/// What each monitor shows, its history and adjustments.
pub(crate) async fn saved_state(state: &AppState) -> CmdResult<SavedState> {
    let store = state.store.clone();
    blocking(move || store.load_state()).await
}

/// The app's colors from the current wallpaper (None: its own).
pub(crate) async fn palette(state: &AppState) -> CmdResult<Option<theme::Palette>> {
    let store = state.store.clone();
    tauri::async_runtime::spawn_blocking(move || theme::palette(&store)).await.map_err(err)?
}

/// Put `text` on the Wayland clipboard (`wl-copy`, which stays to serve it:
/// it outlives the interface, which exits on close; WebKitGTK's clipboard
/// API wasn't reliable either).
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
