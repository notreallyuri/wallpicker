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
    let mut preparation = crate::render::Preparation::default();
    let mut report = ApplyReport::default();
    let mut outputs = Vec::new();
    for m in monitors {
        match backend.prepare(&mut preparation, m, path, fits.of(m, path)) {
            Ok(output) => outputs.push(output),
            Err(e) => report.failed.push((m.clone(), e.to_string())),
        }
    }
    let rendered = backend.apply_batch(runner, outputs)?;
    report.failed.extend(rendered.failed);
    for m in rendered.applied {
        match store.set_current(&m, path) {
            Ok(()) => report.applied.push(m),
            Err(e) => report
                .failed
                .push((m, format!("applied, but could not save state: {e}"))),
        }
    }
    Ok(report)
}
