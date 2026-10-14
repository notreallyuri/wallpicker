//! Source-aware application contract shared by the service, IPC and renderer.
//! Runtime application IDs are never written to wallpaper state or presets.
mod geometry;
mod prepare;
#[cfg(test)]
mod tests;
pub mod transport;
#[cfg(test)]
mod transport_tests;
mod validate;

use crate::{
    Error, Result,
    media::FileIdentity,
    settings::{FitMode, Transition},
};
pub use geometry::{Geometry, View};
pub use prepare::Preparation;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const VERSION: u32 = 1;
pub const PLAYBACK_UNAVAILABLE: &str = "an animated wallpaper needs a source-aware render request";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    Apply,
    Adjust,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Content {
    Static {
        path: PathBuf,
        fit: Option<FitMode>,
        prepared_size: Option<(u32, u32)>,
    },
    Animated {
        identity: FileIdentity,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Picture {
    pub source: PathBuf,
    pub geometry: Geometry,
    pub content: Content,
    /// An animated wallpaper held on its frame (its monitor's `paused`
    /// adjustment): the renderer holds its playback while any monitor
    /// showing it says so.
    #[serde(default, skip_serializing_if = "is_false")]
    pub paused: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

impl Picture {
    pub fn static_file(&self) -> Result<(&std::path::Path, Option<FitMode>)> {
        match &self.content {
            Content::Static { path, fit, .. } => Ok((path, *fit)),
            Content::Animated { .. } => Err(Error::Invalid(PLAYBACK_UNAVAILABLE.into())),
        }
    }

    /// Legacy requests already contain a prepared file, with no source geometry.
    pub fn legacy(path: PathBuf, fit: Option<FitMode>) -> Self {
        Self {
            source: path.clone(),
            geometry: Geometry {
                fit,
                ..Default::default()
            },
            content: Content::Static {
                path,
                fit,
                prepared_size: None,
            },
            paused: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    pub monitor: String,
    pub picture: Picture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Batch {
    pub version: u32,
    pub application: String,
    pub intent: Intent,
    pub transition: Transition,
    pub outputs: Vec<Output>,
}

impl Batch {
    pub fn new(intent: Intent, transition: Transition, outputs: Vec<Output>) -> Self {
        Self {
            version: VERSION,
            application: application_id(),
            intent,
            transition,
            outputs,
        }
    }

    /// Same application plus same source identity identifies one future playback
    /// clock. Reapplying creates a new application; Adjust preserves live clocks.
    pub fn selection(&self, picture: Picture) -> Selection {
        Selection {
            application: self.application.clone(),
            intent: self.intent,
            picture,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub application: String,
    pub intent: Intent,
    pub picture: Picture,
}

impl From<(PathBuf, Option<FitMode>)> for Selection {
    fn from((path, fit): (PathBuf, Option<FitMode>)) -> Self {
        Self {
            application: application_id(),
            intent: Intent::Apply,
            picture: Picture::legacy(path, fit),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Report {
    pub applied: Vec<String>,
    pub failed: Vec<(String, String)>,
}

/// Process/time/counter identity: distinct across clients and rapid applications.
pub fn application_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}-{now}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}
