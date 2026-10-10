//! Applying presets through the native daemon after complete image preflight.

use super::{commit, plan};
use crate::{Error, Result, cmd::Runner, store::Store, wallpapers::Backend};
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ApplyReport {
    pub applied: Vec<String>,
    pub skipped: Vec<(String, String)>,
    pub failed: Vec<(String, String)>,
}

/// Uses current transition preferences, never those of the saved setup.
pub fn apply(runner: &dyn Runner, store: &Store, id: &str) -> Result<ApplyReport> {
    let preset = store
        .load_presets()?
        .into_iter()
        .find(|p| p.id == id)
        .ok_or_else(|| Error::Invalid(format!("no preset with ID {id:?}")))?;
    let outputs = crate::monitors::list(runner)?;
    store.load_settings()?;
    // Refuse broken bookkeeping before changing what any output shows.
    store.load_state()?;
    let plan = plan::prepare(store, &preset, &outputs)?;
    let backend = Backend::new(store);
    let mut report = ApplyReport {
        skipped: plan.skipped.clone(),
        ..Default::default()
    };
    for entry in &plan.entries {
        let fit = if entry.adjust.frame.is_some() || entry.adjust.span.is_some() {
            None
        } else {
            entry.adjust.fit
        };
        match backend.apply_prepared(runner, &entry.monitor, &entry.shown, fit) {
            Ok(()) => report.applied.push(entry.monitor.clone()),
            Err(e) => report.failed.push((entry.monitor.clone(), e.to_string())),
        }
    }
    if let Err(e) = commit::remember(store, &preset, &plan, &report.applied) {
        for monitor in std::mem::take(&mut report.applied) {
            report
                .failed
                .push((monitor, format!("shown, but could not save preset state: {e}")));
        }
    }
    Ok(report)
}
