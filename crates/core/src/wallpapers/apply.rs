use super::*;
/// Apply and remember a native wallpaper, each monitor with its
/// fit ([`Fits`]).
pub fn apply_and_remember(runner: &dyn Runner, store: &Store, monitors: &[String], path: &Path) -> Result<ApplyReport> {
    if !crate::library::is_supported_image(path) {
        return Err(Error::Invalid(format!("unsupported image type: {}", path.display())));
    }
    if monitors.is_empty() {
        return Err(Error::Invalid("no monitor selected".into()));
    }

    // Read once, not per monitor.
    let backend = Backend::new(store);
    let fits = Fits::load(store);
    let apply_one = |m: &str| backend.apply(runner, m, path, fits.of(m, path));

    let mut report = ApplyReport::default();
    for m in monitors {
        match apply_one(m) {
            Ok(()) => match store.set_current(m, path) {
                Ok(()) => report.applied.push(m.clone()),
                // The wallpaper is on screen; only the bookkeeping failed.
                Err(e) => report
                    .failed
                    .push((m.clone(), format!("applied, but could not save state: {e}"))),
            },
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    Ok(report)
}
