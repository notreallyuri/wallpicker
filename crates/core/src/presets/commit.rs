//! Remember only successful outputs; an incomplete apply never claims a full span.

use super::{Preset, plan::Plan};
use crate::{
    Result,
    store::{Store, remember_current},
};

pub(super) fn remember(store: &Store, preset: &Preset, plan: &Plan, succeeded: &[String]) -> Result<()> {
    if succeeded.is_empty() {
        return Ok(());
    }
    let mut state = store.load_state()?;
    if state.span.as_ref().is_some_and(|s| succeeded.iter().any(|m| s.covers(m))) {
        state.span = None;
    }
    for entry in &plan.entries {
        if !succeeded.contains(&entry.monitor) {
            continue;
        }
        remember_current(&mut state, &entry.monitor, &entry.source);
        if entry.adjust.is_none() {
            state.adjust.remove(&entry.monitor);
        } else {
            state.adjust.insert(entry.monitor.clone(), entry.adjust);
        }
    }
    let complete_span = plan.span.as_ref().filter(|s| s.monitors.iter().all(|m| succeeded.contains(m)));
    if let Some(span) = complete_span {
        state.span = Some(span.clone());
    }
    store.save_state(&state)?;
    crate::transform::prune(&store.dirs().cache, &state);
    if complete_span.is_some()
        && let Some(span) = &preset.span
    {
        let mut settings = store.load_settings()?;
        settings.span_layout.extend(span.layout.clone());
        settings.span_monitors = span.monitors.clone();
        settings.span_bezel = span.bezel;
        store.save_settings(&settings)?;
    }
    Ok(())
}
