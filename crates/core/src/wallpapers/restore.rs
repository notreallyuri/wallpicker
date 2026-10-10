use super::*;
/// Put back what each of `monitors` showed before ([`Store::previous`]),
/// with its fit; monitors with nothing earlier are reported as failed.
pub fn apply_previous(runner: &dyn Runner, store: &Store, monitors: &[String]) -> Result<ApplyReport> {
    let fits = Fits::load(store);
    let backend = Backend::new(store);
    let mut report = ApplyReport::default();
    for m in monitors {
        let Some(path) = store.previous(m)? else {
            report.failed.push((m.clone(), "nothing earlier to go back to".into()));
            continue;
        };
        let applied = backend.apply(runner, m, &path, fits.of(m, &path));
        match applied.and_then(|()| store.went_back(m, &path)) {
            Ok(()) => report.applied.push(m.clone()),
            Err(e) => report.failed.push((m.clone(), e.to_string())),
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
    let backend = Backend::from_settings(store);
    let monitors = list_monitors(runner)?;

    let mut report = RestoreReport::default();
    for (name, path) in &state.current {
        if !monitors.iter().any(|m| &m.name == name) {
            report.skipped.push((name.clone(), "monitor not connected".into()));
            continue;
        }
        if !path.is_file() {
            report
                .skipped
                .push((name.clone(), format!("file is gone: {}", path.display())));
            continue;
        }

        let mut last_err = None;
        let mut done = false;
        for attempt in 0..attempts.max(1) {
            match backend.apply(runner, name, path, fits.of(name, path)) {
                Ok(()) => {
                    done = true;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    if attempt + 1 < attempts.max(1) {
                        sleep(delay);
                    }
                }
            }
        }
        if done {
            report.applied.push(name.clone());
        } else if let Some(e) = last_err {
            report.skipped.push((name.clone(), e.to_string()));
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
    for (m, path) in &state.current {
        if !which(m, path) || !connected.iter().any(|c| &c.name == m) || !path.is_file() {
            continue;
        }
        match backend.apply(runner, m, path, fits.of(m, path)) {
            Ok(()) => report.applied.push(m.clone()),
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    Ok(report)
}

/// What `monitor` shows in the saved state, as the configured backend gets
/// it (the file and its fit): what wallpicker's renderer draws on a monitor
/// as it appears. None when it shows nothing saved, or its file is gone.
pub fn saved_for(store: &Store, monitor: &str) -> Option<(PathBuf, Option<FitMode>)> {
    let state = store.load_state().ok()?;
    let path = state.current.get(monitor).filter(|p| p.is_file())?;
    let fit = Fits::load(store).of(monitor, path);
    match Backend::from_settings(store).shown(monitor, path, fit) {
        Ok(shown) => Some(shown),
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
