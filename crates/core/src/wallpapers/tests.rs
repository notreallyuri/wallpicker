use super::*;
use crate::{
    monitors::{Monitor, parse_monitors},
    settings::Transition,
    store::Dirs,
};
use std::cell::RefCell;

struct Desktop {
    calls: RefCell<Vec<(String, PathBuf, Option<FitMode>, String)>>,
    failures: RefCell<usize>,
}
impl Runner for Desktop {
    fn monitors(&self) -> Result<Vec<Monitor>> {
        parse_monitors(r#"[{"name":"DP-1","width":16,"height":8},{"name":"DP-2","width":16,"height":8}]"#)
    }
    fn run(&self, _: &str, _: &[&str]) -> Result<String> {
        panic!("native wallpaper application must not run an external wallpaper command")
    }
    fn show_wallpaper(&self, monitor: &str, path: &Path, fit: Option<FitMode>, transition: &Transition) -> Result<()> {
        self.calls
            .borrow_mut()
            .push((monitor.into(), path.into(), fit, transition.kind.clone()));
        let mut failures = self.failures.borrow_mut();
        if *failures > 0 {
            *failures -= 1;
            Err(Error::Invalid("renderer temporarily unavailable".into()))
        } else {
            Ok(())
        }
    }
}
fn setup() -> (tempfile::TempDir, Store, Desktop, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let path = dir.path().join("image,$ special.png");
    image::RgbImage::new(16, 8).save(&path).unwrap();
    let desktop = Desktop {
        calls: RefCell::default(),
        failures: RefCell::new(0),
    };
    (dir, store, desktop, path)
}
#[test]
fn application_records_successes_and_preserves_native_paths() {
    let (_dir, store, desktop, path) = setup();
    *desktop.failures.borrow_mut() = 1;
    let report = apply_and_remember(&desktop, &store, &["DP-1".into(), "DP-2".into()], &path).unwrap();
    assert_eq!(report.applied, ["DP-2"]);
    assert_eq!(report.failed[0].0, "DP-1");
    let state = store.load_state().unwrap();
    assert_eq!(state.current.len(), 1);
    assert_eq!(state.current["DP-2"], path);
    assert_eq!(desktop.calls.borrow()[1].1, path);
}
#[test]
fn previous_restores_history_and_keeps_selection_transition() {
    let (dir, store, desktop, path) = setup();
    let other = dir.path().join("other.png");
    image::RgbImage::new(16, 8).save(&other).unwrap();
    apply_and_remember(&desktop, &store, &["DP-1".into()], &path).unwrap();
    apply_and_remember(&desktop, &store, &["DP-1".into()], &other).unwrap();
    let report = apply_previous(&desktop, &store, &["DP-1".into()]).unwrap();
    assert_eq!(report.applied, ["DP-1"]);
    assert_eq!(store.load_state().unwrap().current["DP-1"], path);
    assert_eq!(desktop.calls.borrow().last().unwrap().3, "fade");
}
#[test]
fn restore_retries_and_skips_disconnected_or_missing_images() {
    let (dir, store, desktop, path) = setup();
    store.set_current("DP-1", &path).unwrap();
    store.set_current("DP-2", &dir.path().join("gone.png")).unwrap();
    store.set_current("gone-monitor", &path).unwrap();
    *desktop.failures.borrow_mut() = 1;
    let waits = RefCell::new(0);
    let report = restore_with(&desktop, &store, 3, Duration::ZERO, &|_| *waits.borrow_mut() += 1).unwrap();
    assert_eq!(report.applied, ["DP-1"]);
    assert_eq!(report.skipped.len(), 2);
    assert_eq!(*waits.borrow(), 1);
    assert_eq!(desktop.calls.borrow().len(), 2);
}
#[test]
fn native_fit_and_saved_adjustment_are_prepared_before_application() {
    let (_dir, store, desktop, path) = setup();
    let mut settings = Settings::default();
    settings.fit_mode = Some(FitMode::Tile);
    store.save_settings(&settings).unwrap();
    apply_and_remember(&desktop, &store, &["DP-1".into()], &path).unwrap();
    assert_eq!(desktop.calls.borrow()[0].2, Some(FitMode::Tile));
    store
        .set_adjust(
            "DP-1",
            crate::settings::Adjust {
                frame: Some(crate::settings::Frame {
                    zoom: 200,
                    x: 500,
                    y: 500,
                    width: 8,
                    height: 8,
                }),
                ..Default::default()
            },
        )
        .unwrap();
    let saved = saved_for(&store, "DP-1").unwrap();
    let (shown, fit) = saved.picture.static_file().unwrap();
    assert_ne!(shown, path);
    assert_eq!(image::image_dimensions(shown).unwrap(), (8, 8));
    assert_eq!(fit, None);
}
