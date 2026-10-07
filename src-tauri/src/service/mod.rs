//! What the app does, without Tauri: plain functions over the shared
//! `AppState`, one file per area. The window's commands (`commands/`) and
//! `wallpicker serve` (`serve/`, for the Quickshell UI) both call these.
//! Moved here area by area as the Quickshell UI needs them.

pub(crate) mod app;
pub(crate) mod library;
pub(crate) mod online;
pub(crate) mod wallpapers;
