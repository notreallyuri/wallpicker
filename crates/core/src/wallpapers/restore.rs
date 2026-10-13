use super::*;
/// Put back what each of `monitors` showed before ([`Store::previous`]),
/// with its fit; monitors with nothing earlier are reported as failed.
pub fn apply_previous(runner: &dyn Runner, store: &Store, monitors: &[String]) -> Result<ApplyReport> {
    let fits = Fits::load(store);
    let backend = Backend::new(store);
    let mut report = ApplyReport::default();
    let mut preparation = crate::render::Preparation::default();
    let mut paths = BTreeMap::new();
    let mut outputs = Vec::new();
    for m in monitors {
        let Some(path) = store.previous(m)? else {
            report.failed.push((m.clone(), "nothing earlier to go back to".into()));
            continue;
        };
        match backend.prepare(&mut preparation, m, &path, fits.of(m, &path)) {
            Ok(output) => {
                outputs.push(output);
                paths.insert(m.clone(), path);
            }
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    let rendered = backend.apply_batch(runner, outputs)?;
    report.failed.extend(rendered.failed);
    for monitor in rendered.applied {
        match store.went_back(&monitor, &paths[&monitor]) {
            Ok(()) => report.applied.push(monitor),
            Err(e) => report
                .failed
                .push((monitor, format!("shown, but could not save history: {e}"))),
        }
    }
    Ok(report)
}

/// Outcome of [`restore`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct RestoreReport {
    pub applied: Vec<String>,
    /// (monitor, reason)
    pub skipped: Vec<(String, String)>,
}

/// Re-apply the saved wallpapers. Meant to run at login, where the daemon may
/// still be starting, so each monitor is retried `attempts` times, waiting
/// `delay` between tries (via `sleep`, so tests don't actually wait).
pub fn restore_with(
    runner: &dyn Runner,
    store: &Store,
    attempts: u32,
    delay: Duration,
    sleep: &dyn Fn(Duration),
) -> Result<RestoreReport> {
    let state = store.load_state()?;
    let fits = Fits::load(store);
    let backend = Backend::from_settings(store).restoring();
    let monitors = list_monitors(runner)?;

    let mut report = RestoreReport::default();
    let mut preparation = crate::render::Preparation::default();
    let mut pending = Vec::new();
    for (name, path) in &state.current {
        if !monitors.iter().any(|m| &m.name == name && !m.disabled) {
            report.skipped.push((name.clone(), "monitor not connected".into()));
            continue;
        }
        match backend.prepare(&mut preparation, name, path, fits.of(name, path)) {
            Ok(output) => pending.push(output),
            Err(e) => report.skipped.push((name.clone(), e.to_string())),
        }
    }
    for attempt in 0..attempts.max(1) {
        if pending.is_empty() {
            break;
        }
        let rendered = backend
            .apply_batch(runner, pending.clone())
            .unwrap_or_else(|e| crate::render::Report {
                applied: Vec::new(),
                failed: pending.iter().map(|o| (o.monitor.clone(), e.to_string())).collect(),
            });
        report.applied.extend(rendered.applied);
        pending.retain(|o| rendered.failed.iter().any(|(m, _)| *m == o.monitor));
        if attempt + 1 == attempts.max(1) {
            report.skipped.extend(rendered.failed);
        } else if !pending.is_empty() {
            sleep(delay);
        }
    }
    Ok(report)
}

/// Apply again what's on the monitors that `which(monitor, path)` picks,
/// with their fit as it is now: after a fit changes, so it shows right away.
/// Monitors that aren't
/// connected, or whose file is gone, are left alone.
pub fn reapply(runner: &dyn Runner, store: &Store, which: &dyn Fn(&str, &Path) -> bool) -> Result<ApplyReport> {
    let state = store.load_state()?;
    let fits = Fits::load(store);
    let backend = Backend::from_settings(store).without_transition();
    let connected = list_monitors(runner)?;
    let mut report = ApplyReport::default();
    let mut preparation = crate::render::Preparation::default();
    let mut outputs = Vec::new();
    for (m, path) in &state.current {
        if !which(m, path) || !connected.iter().any(|c| &c.name == m && !c.disabled) || !path.is_file() {
            continue;
        }
        match backend.prepare(&mut preparation, m, path, fits.of(m, path)) {
            Ok(output) => outputs.push(output),
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    let rendered = backend.apply_batch(runner, outputs)?;
    report.applied = rendered.applied;
    report.failed.extend(rendered.failed);
    Ok(report)
}

/// Restore a source-aware selection for an output. None when nothing is saved,
/// its source is gone or preparation fails. Renderer startup uses the shared
/// application variant below so restored outputs can share playback later.
pub fn saved_for(store: &Store, monitor: &str) -> Option<crate::render::Selection> {
    saved_for_application(store, monitor, &crate::render::application_id())
}

/// One restoration identity is shared by every output in a renderer lifetime.
pub fn saved_for_application(store: &Store, monitor: &str, application: &str) -> Option<crate::render::Selection> {
    let state = store.load_state().ok()?;
    let path = state.current.get(monitor).filter(|p| p.is_file())?;
    let fit = Fits::load(store).of(monitor, path);
    let backend = Backend::from_settings(store).restoring();
    match backend.prepare(&mut crate::render::Preparation::default(), monitor, path, fit) {
        Ok(output) => Some(crate::render::Selection {
            application: application.into(),
            intent: crate::render::Intent::Restore,
            picture: output.picture,
        }),
        Err(e) => {
            eprintln!("wallpicker: {monitor}: {e}");
            None
        }
    }
}

/// [`restore_with`] using real sleeping.
pub fn restore(runner: &dyn Runner, store: &Store, attempts: u32, delay: Duration) -> Result<RestoreReport> {
    restore_with(runner, store, attempts, delay, &std::thread::sleep)
}
