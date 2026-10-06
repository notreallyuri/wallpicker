//! Shared logic for wallpicker.
//!
//! Everything in here is plain Rust so it can be unit-tested without a
//! desktop session. The Tauri backend is a thin layer over this crate, and the
//! Leptos UI reuses the data types (built with `default-features = false`).

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
pub mod wallhaven;

#[cfg(feature = "native")]
pub mod awww;
#[cfg(feature = "native")]
pub mod cmd;
#[cfg(feature = "native")]
pub mod hyprpaper;
#[cfg(feature = "native")]
pub mod noctalia;
#[cfg(feature = "native")]
pub mod store;
#[cfg(feature = "native")]
pub mod thumbs;
#[cfg(feature = "native")]
pub mod transform;

pub use error::{Error, Result};
