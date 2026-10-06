//! The commands the window calls (`invoke`), by area. Shared bits (the app
//! state, `blocking`, `wallpaper_path`) are in lib.rs.

pub(crate) mod app;
pub(crate) mod backup;
pub(crate) mod library;
pub(crate) mod online;
pub(crate) mod wallpapers;
