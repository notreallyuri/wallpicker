use super::*;
/// What a span asks beyond the picture and its look: the monitors (None:
/// the ones spanned last, else all) and, from the Span dialog, a new
/// arrangement to keep (each monitor's top left in mm, and the bezel).
#[derive(Debug, Clone, Default)]
pub struct SpanAsk {
    pub monitors: Option<Vec<String>>,
    pub layout: Option<(BTreeMap<String, SpanPoint>, u32)>,
}

/// Span `path` across some connected monitors (`span.rs`, on wallpicker's
/// canvas): each records it (its history keeps what it showed) with its
/// part as a cut, then shows that part, zoomed, moved and mirrored by
/// `look`. The same picture again only cuts it anew (the history stays).
/// The monitors chosen and the arrangement are kept for next time.
/// Temporary like any adjustment: another wallpaper on one of them ends the
/// span.
pub fn span_and_remember(
    runner: &dyn Runner,
    store: &Store,
    path: &Path,
    look: SpanLook,
    ask: SpanAsk,
) -> Result<ApplyReport> {
    if !crate::library::is_supported_image(path) {
        return Err(Error::Invalid(format!("unsupported image type: {}", path.display())));
    }
    let adjusting_span = store.load_state()?.span.is_some_and(|span| span.path == path);
    let connected = list_monitors(runner)?;
    let mut settings = store.load_settings()?;
    let wanted = ask.monitors.unwrap_or_else(|| settings.span_monitors.clone());
    let mut chosen: Vec<String> = connected
        .iter()
        .map(|m| m.name.clone())
        .filter(|n| wanted.is_empty() || wanted.contains(n))
        .collect();
    // The last ones are gone (unplugged): every monitor.
    if chosen.len() < 2 && ask_was_default(&wanted, &settings) {
        chosen = connected.iter().map(|m| m.name.clone()).collect();
    }
    if chosen.len() < 2 {
        return Err(Error::Invalid("spanning needs two monitors or more".into()));
    }
    let mut preparation = crate::render::Preparation::default();
    let (width, height) = preparation.inspect(path)?;
    if let Some((layout, bezel)) = ask.layout {
        settings.span_layout.extend(layout);
        settings.span_bezel = bezel;
    }
    settings.span_monitors = chosen.clone();
    let placed: Vec<crate::span::Placed> =
        crate::span::arrangement(&connected, &settings.span_layout, settings.span_bezel)
            .into_iter()
            .filter(|p| chosen.contains(&p.name))
            .collect();
    let look = look.sanitized();
    let cuts = crate::span::cuts(&placed, width, height, &look);
    let backend = Backend::new(store);
    let backend = if adjusting_span {
        backend.without_transition()
    } else {
        backend
    };
    let mut outputs = Vec::new();
    for (monitor, cut) in &cuts {
        let adjust = crate::settings::Adjust {
            span: Some(*cut),
            flip: look.flip,
            ..Default::default()
        };
        outputs.push(crate::render::Output {
            monitor: monitor.clone(),
            picture: preparation.picture(&store.dirs().cache, path, &adjust, None)?,
        });
    }
    let rendered = backend.apply_batch(runner, outputs)?;
    let mut report = ApplyReport {
        applied: rendered.applied,
        failed: rendered.failed,
    };
    if report.applied.is_empty() {
        return Ok(report);
    }
    // Commit only confirmed outputs. A partial span becomes standalone cuts;
    // failed outputs retain their paths, adjustments and history.
    let remember = || -> Result<()> {
        let mut state = store.load_state()?;
        if state
            .span
            .as_ref()
            .is_some_and(|s| report.applied.iter().any(|m| s.covers(m)))
        {
            state.span = None;
        }
        for (monitor, cut) in &cuts {
            if !report.applied.contains(monitor) {
                continue;
            }
            crate::store::remember_current(&mut state, monitor, path);
            state.adjust.insert(
                monitor.clone(),
                crate::settings::Adjust {
                    span: Some(*cut),
                    flip: look.flip,
                    ..Default::default()
                },
            );
        }
        let complete = report.applied.len() == cuts.len();
        if complete {
            state.span = Some(crate::settings::Spanned {
                path: path.into(),
                look,
                monitors: cuts.iter().map(|(m, _)| m.clone()).collect(),
                places: placed.iter().map(|p| (p.name.clone(), p.rect)).collect(),
            });
        }
        store.save_state(&state)?;
        crate::transform::prune(&store.dirs().cache, &state);
        if complete {
            store.save_settings(&settings)?;
        }
        Ok(())
    };
    if let Err(e) = remember() {
        for monitor in std::mem::take(&mut report.applied) {
            report
                .failed
                .push((monitor, format!("shown, but could not save span state: {e}")));
        }
    }
    Ok(report)
}

/// Whether the monitors asked were the remembered ones (not a choice made
/// now), so a choice gone stale can fall back to every monitor.
fn ask_was_default(wanted: &[String], settings: &Settings) -> bool {
    wanted == settings.span_monitors.as_slice()
}

/// What the Span dialog shows: every connected monitor on the canvas (as
/// arranged, or `fresh` from Hyprland's layout), which ones are spanned by
/// default, and the bezel.
#[derive(Debug, Clone, Serialize)]
pub struct SpanLayout {
    pub monitors: Vec<SpanMonitor>,
    pub chosen: Vec<String>,
    pub bezel: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpanMonitor {
    pub name: String,
    #[serde(flatten)]
    pub rect: crate::settings::SpanRect,
    pub out_width: u32,
    pub out_height: u32,
}

pub fn span_layout(runner: &dyn Runner, store: &Store, fresh: bool, bezel: Option<u32>) -> Result<SpanLayout> {
    let connected = list_monitors(runner)?;
    let settings = store.load_settings()?;
    let bezel = bezel.unwrap_or(settings.span_bezel).min(100);
    let saved = if fresh {
        BTreeMap::new()
    } else {
        settings.span_layout.clone()
    };
    let monitors: Vec<SpanMonitor> = crate::span::arrangement(&connected, &saved, bezel)
        .into_iter()
        .map(|p| SpanMonitor {
            name: p.name,
            rect: p.rect,
            out_width: p.out_width,
            out_height: p.out_height,
        })
        .collect();
    let mut chosen: Vec<String> = monitors
        .iter()
        .map(|m| m.name.clone())
        .filter(|n| settings.span_monitors.contains(n))
        .collect();
    if chosen.len() < 2 {
        chosen = monitors.iter().map(|m| m.name.clone()).collect();
    }
    Ok(SpanLayout {
        monitors,
        chosen,
        bezel,
    })
}
