//! Explicit simultaneous connector renaming. Never changes live wallpaper state.
use super::{Preset, validation};
use crate::{Error, Result, store::Store};
use std::collections::{BTreeMap, BTreeSet};

pub type MonitorMapping = BTreeMap<String, String>;

pub fn reassign(preset: &Preset, mapping: &MonitorMapping) -> Result<Preset> {
    for (source, target) in mapping {
        if !preset.monitors.contains_key(source) {
            return Err(Error::Invalid(format!("preset has no monitor {source:?}")));
        }
        if target.trim().is_empty() || target.trim() != target || target.chars().any(char::is_control) {
            return Err(Error::Invalid(
                "monitor names must be nonempty without whitespace padding or control characters".into(),
            ));
        }
    }
    let renamed = |name: &String| mapping.get(name).unwrap_or(name).clone();
    let targets: BTreeSet<_> = preset.monitors.keys().map(renamed).collect();
    if targets.len() != preset.monitors.len() {
        return Err(Error::Invalid(
            "each saved monitor must have a different destination, including unchanged monitors".into(),
        ));
    }
    let mut result = preset.clone();
    result.monitors = preset
        .monitors
        .iter()
        .map(|(name, wallpaper)| (renamed(name), wallpaper.clone()))
        .collect();
    if let Some(span) = &mut result.span {
        span.monitors = span.monitors.iter().map(renamed).collect();
        span.layout = span
            .layout
            .iter()
            .map(|(name, point)| (renamed(name), *point))
            .collect();
    }
    validation::preset(&result)?;
    Ok(result)
}

impl Store {
    /// Preview without writing or applying. Unspecified connectors stay unchanged.
    pub fn preview_preset_reassignment(&self, id: &str, mapping: &MonitorMapping) -> Result<Preset> {
        let collection = self.preset_collection()?;
        let preset = collection
            .items
            .iter()
            .find(|p| p.id == id)
            .ok_or_else(|| Error::Invalid(format!("no preset with ID {id:?}")))?;
        reassign(preset, mapping)
    }

    /// `expected` protects a preview from concurrent preset edits while the dialog was open.
    pub fn reassign_preset(&self, id: &str, mapping: &MonitorMapping, expected: Option<&Preset>) -> Result<Preset> {
        self.edit_presets(|collection| {
            let preset = collection
                .items
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or_else(|| Error::Invalid(format!("no preset with ID {id:?}")))?;
            if expected.is_some_and(|expected| expected != preset) {
                return Err(Error::Invalid(
                    "this preset changed while reassignment was open; reopen it to review the new setup".into(),
                ));
            }
            *preset = reassign(preset, mapping)?;
            Ok(preset.clone())
        })
    }
}
