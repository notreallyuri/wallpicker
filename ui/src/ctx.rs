//! App-wide state shared through Leptos context, and the actions on it.

use std::path::Path;

use leptos::prelude::*;
use wallpicker_core::catalog::Catalog;
use wallpicker_core::monitors::Monitor;
use wallpicker_core::settings::{Settings, State};

use crate::api;
use crate::components::nc::prelude::*;

/// ToggleGroup value for "every monitor". Hyprland output names never look
/// like this.
pub(crate) const ALL_MONITORS: &str = "__all__";

/// App-wide state, shared through Leptos context. Every field is a signal (or
/// another `Copy` handle), so the struct is `Copy` and moves into closures
/// freely.
#[derive(Clone, Copy)]
pub(crate) struct Ctx {
    pub(crate) monitors: RwSignal<Vec<Monitor>>,
    /// The monitor bar's choice: one monitor name, or [`ALL_MONITORS`].
    /// Bound to a single-select ToggleGroup.
    pub(crate) selection: RwSignal<Vec<String>>,
    /// Monitors the next click applies to, derived from `selection`.
    pub(crate) targets: Signal<Vec<String>>,
    /// What each monitor currently shows (from state.json).
    pub(crate) state: RwSignal<State>,
    pub(crate) settings: RwSignal<Option<Settings>>,
    /// Bumped to make the local view rescan its folders.
    pub(crate) library_rev: RwSignal<u32>,
    /// Path currently being applied, to show a busy tile.
    pub(crate) applying: RwSignal<Option<String>>,
    /// Favorites and tags (library.json).
    pub(crate) catalog: RwSignal<Catalog>,
    /// A Wallhaven search asked for from elsewhere (a tag in a collection's
    /// details): the Online tab runs it and clears this.
    pub(crate) online_search: RwSignal<Option<String>>,
    /// The current wallpaper, blurred, behind the app (Appearance; a
    /// per-machine view preference, kept in localStorage).
    pub(crate) backdrop: RwSignal<bool>,
    /// The monitor drawer under the top bar is open (remembered).
    pub(crate) monitors_open: RwSignal<bool>,
    /// Where `wallpicker --open` asked to go, until the part that shows it
    /// (the shell for tabs and Settings, Local for Duplicates and tags)
    /// takes it.
    pub(crate) open_place: RwSignal<Option<crate::api::OpenPlace>>,
    pub(crate) toasts: ToastContext,
}

impl Ctx {
    pub(crate) fn new(toasts: ToastContext) -> Ctx {
        let monitors = RwSignal::new(Vec::<Monitor>::new());
        let selection = RwSignal::new(Vec::<String>::new());
        let targets = Signal::derive(move || match selection.with(|s| s.first().cloned()) {
            Some(name) if name == ALL_MONITORS => {
                monitors.with(|ms| ms.iter().map(|m| m.name.clone()).collect())
            }
            Some(name) => vec![name],
            None => Vec::new(),
        });
        Ctx {
            monitors,
            selection,
            targets,
            state: RwSignal::new(State::default()),
            settings: RwSignal::new(None),
            library_rev: RwSignal::new(0),
            applying: RwSignal::new(None),
            catalog: RwSignal::new(Catalog::default()),
            online_search: RwSignal::new(None),
            backdrop: RwSignal::new(crate::prefs::get("ui.backdrop").as_deref() != Some("0")),
            monitors_open: RwSignal::new(crate::prefs::get("ui.monitors_open").as_deref() == Some("1")),
            open_place: RwSignal::new(None),
            toasts,
        }
    }

    pub(crate) fn info(self, text: impl Into<String>) {
        self.toasts.toast(text).show();
    }

    // Errors are also logged to the web inspector console, since toasts
    // disappear and their text can't be selected.
    pub(crate) fn error(self, text: impl Into<String>) {
        let text = text.into();
        leptos::logging::error!("{text}");
        self.toasts
            .toast(text)
            .variant(ToastVariant::Destructive)
            .duration(8000.0)
            .show();
    }

    pub(crate) fn error_with(self, title: &str, detail: &str) {
        leptos::logging::error!("{title}: {detail}");
        self.toasts
            .toast(title)
            .description(detail)
            .variant(ToastVariant::Destructive)
            .duration(8000.0)
            .show();
    }

    pub(crate) fn has_api_key(self) -> bool {
        self.settings.with(|s| {
            s.as_ref()
                .and_then(|s| s.wallhaven_api_key.as_ref())
                .is_some()
        })
    }

    pub(crate) async fn refresh_state(self) {
        match api::get_state().await {
            Ok(s) => self.state.set(s),
            Err(e) => self.error_with("Could not read saved state", &e),
        }
    }

    pub(crate) async fn refresh_catalog(self) {
        match api::get_catalog().await {
            Ok(c) => self.catalog.set(c),
            Err(e) => self.error_with("Could not read favorites and tags", &e),
        }
    }

    /// Change the catalog: shown at once, then replaced by what the backend
    /// saved. If saving fails, reload the saved version and say why.
    async fn change_catalog(
        self,
        local: impl FnOnce(&mut Catalog),
        save: impl std::future::Future<Output = Result<Catalog, String>>,
    ) {
        self.catalog.update(local);
        match save.await {
            Ok(c) => self.catalog.set(c),
            Err(e) => {
                self.error_with("Could not save favorites and tags", &e);
                self.refresh_catalog().await;
            }
        }
    }

    pub(crate) async fn set_favorite(self, path: String, favorite: bool) {
        let p = path.clone();
        self.change_catalog(
            move |c| c.set_favorite(Path::new(&p), favorite),
            api::set_favorite(&path, favorite),
        )
        .await;
    }

    pub(crate) async fn set_fit(self, path: String, fit: Option<wallpicker_core::settings::FitMode>) {
        let p = path.clone();
        self.change_catalog(move |c| c.set_fit(Path::new(&p), fit), api::set_wallpaper_fit(&path, fit))
            .await;
        // It was applied again where it shows.
        self.refresh_state().await;
    }

    pub(crate) async fn add_tag(self, path: String, tag: String) {
        let (p, t) = (path.clone(), tag.clone());
        self.change_catalog(
            move |c| {
                c.add_tag(Path::new(&p), &t);
            },
            api::add_tag(&path, &tag),
        )
        .await;
    }

    pub(crate) async fn remove_tag(self, path: String, tag: String) {
        let (p, t) = (path.clone(), tag.clone());
        self.change_catalog(
            move |c| c.remove_tag(Path::new(&p), &t),
            api::remove_tag(&path, &tag),
        )
        .await;
    }

    /// Apply a local file to the monitors selected in the bar.
    pub(crate) async fn apply(self, path: String) {
        self.apply_to(path, self.targets.get_untracked()).await;
    }

    /// Apply a local file to `targets`.
    pub(crate) async fn apply_to(self, path: String, targets: Vec<String>) {
        if targets.is_empty() {
            self.error("Select at least one monitor at the top first.");
            return;
        }
        self.applying.set(Some(path.clone()));
        let result = api::apply_wallpaper(&targets, &path).await;
        self.applying.set(None);

        match result {
            Ok(r) if r.failed.is_empty() => {
                self.info(format!("Applied to {}", r.applied.join(", ")));
            }
            Ok(r) => {
                let failed: Vec<&str> = r.failed.iter().map(|(m, _)| m.as_str()).collect();
                let title = if r.applied.is_empty() {
                    "Could not apply the wallpaper".to_string()
                } else {
                    format!(
                        "Applied to {}, but not to {}",
                        r.applied.join(", "),
                        failed.join(", ")
                    )
                };
                // Usually every monitor fails for the same reason (e.g.
                // hyprpaper isn't running); say it once.
                let first = &r.failed[0].1;
                let detail = if r.failed.iter().all(|(_, why)| why == first) {
                    first.clone()
                } else {
                    r.failed
                        .iter()
                        .map(|(m, why)| format!("{m}: {why}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                self.error_with(&title, &detail);
            }
            Err(e) => self.error_with("Could not apply the wallpaper", &e),
        }
        self.refresh_state().await;
    }
}
