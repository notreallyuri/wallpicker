//! Native wallpaper application, adjustments, restore and competing daemon detection.
use crate::monitors::list as list_monitors;
use crate::{
    cmd::Runner,
    error::{Error, Result},
    settings::{FitMode, Settings, SpanLook, SpanPoint},
    store::Store,
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::Duration,
};
mod apply;
mod conflicts;
mod drawing;
mod restore;
mod span;
pub use apply::apply_and_remember;
pub use conflicts::{conflicts, stop_program};
pub use drawing::{Backend, Fits};
pub use restore::{RestoreReport, apply_previous, reapply, restore, restore_with, saved_for, saved_for_application};
pub use span::{SpanAsk, SpanLayout, SpanMonitor, span_and_remember, span_layout};
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct ApplyReport {
    /// Monitors that now show the image (and were saved to the state file).
    pub applied: Vec<String>,
    /// (monitor, error message)
    pub failed: Vec<(String, String)>,
}

#[cfg(test)]
mod tests;
