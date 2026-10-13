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
    application: String,
    intent: crate::render::Intent,
}

impl Backend {
    pub fn new(store: &Store) -> Backend {
        Backend {
            application: crate::render::application_id(),
            intent: crate::render::Intent::Apply,
            state: store.load_state().unwrap_or_default(),
            cache: store.dirs().cache.clone(),
            transition: store.load_settings().map(|s| s.transition).unwrap_or_default(),
        }
    }

    /// Interactive adjustments replace the current frame immediately.
    /// This affects only this operation; saved transition preferences stay intact.
    pub fn without_transition(mut self) -> Self {
        self.intent = crate::render::Intent::Adjust;
        self.transition.kind = "none".into();
        self.transition.duration_ms = 0;
        self
    }

    /// The one in the settings.
    pub fn from_settings(store: &Store) -> Backend {
        Backend::new(store)
    }

    pub fn restoring(mut self) -> Self {
        self.intent = crate::render::Intent::Restore;
        self
    }

    pub fn selection(&self, picture: crate::render::Picture) -> crate::render::Selection {
        crate::render::Selection {
            application: self.application.clone(),
            intent: self.intent,
            picture,
        }
    }

    pub fn prepare(
        &self,
        preparation: &mut crate::render::Preparation,
        monitor: &str,
        path: &Path,
        fit: Option<FitMode>,
    ) -> Result<crate::render::Output> {
        let adjust = self.state.adjust_for(monitor, path);
        Ok(crate::render::Output {
            monitor: monitor.into(),
            picture: preparation.picture(&self.cache, path, &adjust, fit)?,
        })
    }

    /// A batch keeps one application identity through retries and all outputs.
    pub fn apply_batch(
        &self,
        runner: &dyn Runner,
        outputs: Vec<crate::render::Output>,
    ) -> Result<crate::render::Report> {
        if outputs.is_empty() {
            return Ok(crate::render::Report::default());
        }
        let batch = crate::render::Batch {
            version: crate::render::VERSION,
            application: self.application.clone(),
            intent: self.intent,
            transition: self.transition.clone(),
            outputs,
        };
        batch.validate()?;
        let report = runner.render(&batch)?;
        report.validate_for(&batch)?;
        Ok(report)
    }

    pub fn apply(&self, runner: &dyn Runner, monitor: &str, path: &Path, fit: Option<FitMode>) -> Result<()> {
        let output = self.prepare(&mut crate::render::Preparation::default(), monitor, path, fit)?;
        let report = self.apply_batch(runner, vec![output])?;
        match report.failed.into_iter().next() {
            Some((_, error)) => Err(Error::Invalid(error)),
            None => Ok(()),
        }
    }
}
