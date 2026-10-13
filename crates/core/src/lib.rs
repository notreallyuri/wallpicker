//! Shared logic for wallpicker.
//!
//! Everything in here is plain Rust so it can be unit-tested without a
//! desktop session. `wallpicker` (crates/wallpicker: the picker's backend,
//! the daemon, the command line) is a thin layer over this crate.

pub mod backup;
pub mod catalog;
pub mod colors;
pub mod duplicates;
pub mod error;
pub mod library;
pub mod media;
pub mod moebooru;
pub mod monitors;
pub mod online;
pub mod presets;
pub mod settings;
pub mod span;
pub mod wallhaven;

pub mod cmd;
pub mod compose;
pub mod daemon;
pub mod noctalia;
pub mod store;
pub mod thumbs;
pub mod transform;
pub mod wallpapers;
pub mod workspaces;

pub use error::{Error, Result};

#[cfg(test)]
mod adjustment_tests;

#[cfg(test)]
mod migration_tests;
