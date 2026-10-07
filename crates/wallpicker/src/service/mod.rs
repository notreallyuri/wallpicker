//! What the app does, without Tauri: plain functions over the shared
//! `AppState`, one file per area. The window's commands (`commands/`) and
//! `wallpicker serve` (`serve/`, for the Quickshell UI) both call these.
//! Moved here area by area as the Quickshell UI needs them.

pub mod app;
pub mod backup;
pub mod files;
pub mod library;
pub mod online;
pub mod settings;
pub mod slideshow;
pub mod wallpapers;
pub mod workspaces;
