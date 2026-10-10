//! Capture wallpaper intent, without histories, automation or cache images.

use super::{MonitorWallpaper, Preset, PresetFrame, PresetSpan, SourceRegion};
use crate::{
    Error, Result,
    settings::{Settings, SpanPoint, State},
};

pub(super) fn capture(id: String, name: String, state: &State, settings: &Settings) -> Result<Preset> {
    if state.current.is_empty() {
        return Err(Error::Invalid("there is no wallpaper setup to save".into()));
    }
    let span = state.span.as_ref().map(|saved| {
        let monitors = if saved.monitors.is_empty() {
            state
                .current
                .iter()
                .filter(|(m, p)| {
                    **p == saved.path
                        && (saved.places.contains_key(*m) || state.adjust.get(*m).is_some_and(|a| a.span.is_some()))
                })
                .map(|(m, _)| m.clone())
                .collect()
        } else {
            saved.monitors.clone()
        };
        let layout = saved.places.iter().map(|(m, r)| (m.clone(), SpanPoint { x: r.x, y: r.y })).collect();
        PresetSpan { wallpaper: saved.path.clone(), monitors, look: saved.look, layout, bezel: settings.span_bezel }
    });
    let monitors = state
        .current
        .iter()
        .map(|(monitor, path)| {
            let adjust = state.adjust_for(monitor, path);
            let grouped = span.as_ref().is_some_and(|s| s.monitors.contains(monitor));
            let region = if grouped {
                None
            } else {
                adjust.span.map(|cut| SourceRegion { x: cut.x, y: cut.y, width: cut.width, height: cut.height })
            };
            let frame = if grouped || region.is_some() {
                None
            } else {
                adjust.frame.map(|f| PresetFrame { zoom: f.zoom, x: f.x, y: f.y })
            };
            (
                monitor.clone(),
                MonitorWallpaper {
                    wallpaper: path.clone(),
                    fit: adjust.effective_fit(settings, monitor),
                    flip: adjust.flip,
                    frame,
                    region,
                },
            )
        })
        .collect();
    let preset = Preset { id, name, monitors, span };
    super::validation::preset(&preset)?;
    Ok(preset)
}
