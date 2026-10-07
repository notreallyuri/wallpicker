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
pub mod moebooru;
pub mod monitors;
pub mod online;
pub mod settings;
pub mod span;
pub mod wallhaven;

pub mod awww;
pub mod cmd;
pub mod hyprpaper;
pub mod noctalia;
pub mod store;
pub mod thumbs;
pub mod transform;
pub mod unsplash;
pub mod workspaces;

pub use error::{Error, Result};
