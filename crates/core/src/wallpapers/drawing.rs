use super::*;
/// What decides each monitor's fit, read once: the settings (general and
/// per-monitor fit) and the monitor's adjustment of what it shows.
pub struct Fits {
    settings: Settings,
    state: crate::settings::State,
}

impl Fits {
    /// Files that can't be read count as defaults: a fit is never worth
    /// failing over.
    pub fn load(store: &Store) -> Fits {
        Fits {
            settings: store.load_settings().unwrap_or_default(),
            state: store.load_state().unwrap_or_default(),
        }
    }

    pub fn of(&self, monitor: &str, path: &Path) -> Option<FitMode> {
        let adjust = self.state.adjust_for(monitor, path);
        // Framed: cut to the monitor's exact size, nothing to fit.
        if adjust.frame.is_some() {
            return None;
        }
        adjust.effective_fit(&self.settings, monitor)
    }
}

/// Prepared wallpaper state and transition preferences, read
/// once for a batch of monitors.
pub struct Backend {
    /// Each monitor's adjustments, and where changed copies are cached.
    state: crate::settings::State,
    cache: PathBuf,
    transition: crate::settings::Transition,
}

impl Backend {
    pub fn new(store: &Store) -> Backend {
        Backend {
            state: store.load_state().unwrap_or_default(),
            cache: store.dirs().cache.clone(),
            transition: store.load_settings().map(|s| s.transition).unwrap_or_default(),
        }
    }

    /// Interactive adjustments replace the current frame immediately.
    /// This affects only this operation; saved transition preferences stay intact.
    pub fn without_transition(mut self) -> Self {
        self.transition.kind = "none".into();
        self.transition.duration_ms = 0;
        self
    }

    /// The one in the settings.
    pub fn from_settings(store: &Store) -> Backend {
        Backend::new(store)
    }

    /// Show an adjusted wallpaper on a monitor.
    pub fn apply(&self, runner: &dyn Runner, monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<()> {
        let (file, fit) = self.shown(monitor, path, fit)?;
        self.apply_prepared(runner, monitor, &file, fit)
    }

    /// Show an already prepared image without consulting the current adjustments.
    /// Batch operations use this after preparing every image before application.
    pub fn apply_prepared(&self, runner: &dyn Runner, monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<()> {
        runner.show_wallpaper(monitor, path, fit, &self.transition)
    }

    /// The file the backend gets for `path` on `monitor`, and its fit: an
    /// adjusted wallpaper is shown from its changed copy (only while the
    /// monitor still shows it: a new one goes on as it is).
    pub fn shown(&self, monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<(PathBuf, Option<FitMode>)> {
        let adjust = self.state.adjust_for(monitor, path);
        let file = crate::transform::shown_file(&self.cache, path, &adjust)?;
        // Framed or spanned: the copy is the monitor's exact size, nothing
        // to fit.
        let fit = if adjust.frame.is_some() || adjust.span.is_some() {
            None
        } else {
            fit
        };
        Ok((file, fit))
    }
}
