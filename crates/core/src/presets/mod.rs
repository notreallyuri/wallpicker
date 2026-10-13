//! Named wallpaper setups. Capturing and editing presets never changes the desktop.

#[cfg(test)]
mod application_tests;
mod apply;
mod capture;
mod commit;
mod plan;
mod reassign;
#[cfg(test)]
mod reassign_tests;
#[cfg(test)]
mod span_tests;
mod storage;
#[cfg(test)]
mod tests;
pub(crate) mod validation;

use crate::settings::{FitMode, Flip, SpanLook, SpanPoint};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const FORMAT: u32 = 1;

pub use apply::{ApplyReport, apply};
pub use reassign::{MonitorMapping, reassign};

/// A stable identity and an explicitly saved setup; not linked to live state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub monitors: BTreeMap<String, MonitorWallpaper>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<PresetSpan>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorWallpaper {
    pub wallpaper: PathBuf,
    /// Resolved global/monitor/adjustment fit. None explicitly means backend default.
    pub fit: Option<FitMode>,
    pub flip: Flip,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<PresetFrame>,
    /// A standalone source-image cut retained after separating a span.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<SourceRegion>,
}

/// Output dimensions are deliberately omitted: rebuild them when applying.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PresetFrame {
    pub zoom: u16,
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetSpan {
    pub wallpaper: PathBuf,
    pub monitors: Vec<String>,
    pub look: SpanLook,
    pub layout: BTreeMap<String, SpanPoint>,
    pub bezel: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub version: u32,
    pub next_id: u64,
    pub items: Vec<Preset>,
}

impl Default for Collection {
    fn default() -> Self {
        Self {
            version: FORMAT,
            next_id: 1,
            items: Vec::new(),
        }
    }
}
