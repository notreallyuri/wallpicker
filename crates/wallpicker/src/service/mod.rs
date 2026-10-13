//! What wallpicker does: plain functions over the shared `AppState`, one
//! file per area. The command line (`cli/`) and
//! `wallpicker serve` (`serve/`, for the Quickshell UI) both call these.

pub mod app;
pub mod backup;
pub mod files;
pub mod library;
pub mod online;
pub mod settings;
pub mod slideshow;
pub mod wallpapers;
pub mod workspaces;

pub mod presets;
pub mod renderer;
