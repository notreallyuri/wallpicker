//! Preflight and resolution-dependent geometry. No backend calls or state writes.

use super::Preset;
use crate::{
    Error, Result,
    monitors::Monitor,
    settings::{Adjust, Cut, Frame, Spanned},
    span,
    store::Store,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub(super) struct Entry {
    pub monitor: String,
    pub source: PathBuf,
    pub shown: PathBuf,
    pub adjust: Adjust,
}

pub(super) struct Plan {
    pub entries: Vec<Entry>,
    pub skipped: Vec<(String, String)>,
    pub span: Option<Spanned>,
}

pub(super) fn prepare(store: &Store, preset: &Preset, outputs: &[Monitor]) -> Result<Plan> {
    super::validation::preset(preset)?;
    let connected: BTreeMap<_, _> = outputs.iter().filter(|m| !m.disabled).map(|m| (m.name.as_str(), m)).collect();
    if connected.len() != outputs.iter().filter(|m| !m.disabled).count() {
        return Err(Error::Invalid("duplicate connected monitor names".into()));
    }
    let missing_span: Vec<_> = preset
        .span
        .as_ref()
        .map_or(Vec::new(), |s| s.monitors.iter().filter(|m| !connected.contains_key(m.as_str())).cloned().collect());
    let mut skipped = Vec::new();
    let mut selected = Vec::new();
    for (name, wallpaper) in &preset.monitors {
        if !connected.contains_key(name.as_str()) {
            skipped.push((name.clone(), "monitor is disconnected".into()));
        } else if !missing_span.is_empty() && preset.span.as_ref().is_some_and(|s| s.monitors.contains(name)) {
            skipped.push((name.clone(), format!("span is incomplete; missing {}", missing_span.join(", "))));
        } else {
            selected.push((name, wallpaper, connected[name.as_str()]));
        }
    }
    // Decode each selected source completely, once, before any application.
    let sources: BTreeSet<_> = selected.iter().map(|(_, w, _)| &w.wallpaper).collect();
    let mut dimensions = BTreeMap::new();
    for path in sources {
        if !crate::library::is_supported_image(path) {
            return Err(Error::Invalid(format!("unsupported preset image: {}", path.display())));
        }
        let image = crate::compose::open(path)?;
        dimensions.insert(path.clone(), image.dimensions());
    }
    let span = preset.span.as_ref().filter(|_| missing_span.is_empty()).map(|saved| {
        let monitors: Vec<_> = saved.monitors.iter().map(|m| (*connected[m.as_str()]).clone()).collect();
        let placed = span::arrangement(&monitors, &saved.layout, saved.bezel);
        Spanned {
            path: saved.wallpaper.clone(),
            look: saved.look,
            monitors: saved.monitors.clone(),
            places: placed.iter().map(|p| (p.name.clone(), p.rect)).collect(),
        }
    });
    let cuts: BTreeMap<_, _> = match (&span, &preset.span) {
        (Some(group), Some(saved)) => {
            let monitors: Vec<_> = group.monitors.iter().map(|m| (*connected[m.as_str()]).clone()).collect();
            let placed = span::arrangement(&monitors, &saved.layout, saved.bezel);
            let (w, h) = dimensions[&group.path];
            span::cuts(&placed, w, h, &group.look).into_iter().collect()
        }
        _ => BTreeMap::new(),
    };
    let mut entries = Vec::new();
    for (name, wallpaper, output) in selected {
        let (width, height) =
            if output.transform % 2 == 1 { (output.height, output.width) } else { (output.width, output.height) };
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(Error::Invalid(format!("unsupported output dimensions for {name}: {width}×{height}")));
        }
        let region = wallpaper.region.map(|r| Cut {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
            out_width: width,
            out_height: height,
        });
        if let Some(c) = region {
            let (w, h) = dimensions[&wallpaper.wallpaper];
            if c.x + c.width > w || c.y + c.height > h {
                return Err(Error::Invalid(format!(
                    "saved source region no longer fits {}",
                    wallpaper.wallpaper.display()
                )));
            }
        }
        let adjust = Adjust {
            fit: wallpaper.fit,
            backend_default_fit: wallpaper.fit.is_none(),
            flip: wallpaper.flip,
            frame: wallpaper.frame.map(|f| Frame { zoom: f.zoom, x: f.x, y: f.y, width, height }),
            span: cuts.get(name).copied().or(region),
        };
        // Generate all changed images now. A later failure still leaves live
        // state and the desktop untouched; only disposable cache files exist.
        let shown = crate::transform::shown_file(&store.dirs().cache, &wallpaper.wallpaper, &adjust)?;
        entries.push(Entry { monitor: name.clone(), source: wallpaper.wallpaper.clone(), shown, adjust });
    }
    Ok(Plan { entries, skipped, span })
}
