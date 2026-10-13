use super::*;
use crate::{
    Error, Result,
    cmd::Runner,
    monitors::{Monitor, parse_monitors},
    settings::{Adjust, Frame, Settings, State},
    store::{Dirs, Store},
};
use std::cell::RefCell;
use tempfile::TempDir;

struct Desktop {
    outputs: Vec<Monitor>,
    calls: RefCell<Vec<(String, Vec<String>)>>,
    fail_on: Option<String>,
}

impl Runner for Desktop {
    fn monitors(&self) -> Result<Vec<Monitor>> {
        Ok(self.outputs.clone())
    }
    fn show_wallpaper(
        &self,
        monitor: &str,
        path: &std::path::Path,
        _fit: Option<FitMode>,
        _transition: &crate::settings::Transition,
    ) -> Result<()> {
        self.run("wallpicker", &[&format!("{monitor},{}", path.display())])
            .map(drop)
    }
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        self.calls
            .borrow_mut()
            .push((program.into(), args.iter().map(|s| (*s).into()).collect()));
        if self
            .fail_on
            .as_ref()
            .is_some_and(|m| args.iter().any(|a| a.starts_with(&format!("{m},"))))
        {
            Err(Error::Command {
                cmd: program.into(),
                msg: "deliberate failure".into(),
            })
        } else {
            Ok(String::new())
        }
    }
}

fn setup() -> (TempDir, Store, Desktop, Preset) {
    let dir = TempDir::new().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let source = dir.path().join("source.png");
    image::RgbImage::from_fn(32, 24, |x, y| image::Rgb([x as u8 * 7, y as u8 * 9, 100]))
        .save(&source)
        .unwrap();
    let mut state = State::default();
    for name in ["DP-1", "DP-2"] {
        state.current.insert(name.into(), source.clone());
    }
    state.adjust.insert(
        "DP-1".into(),
        Adjust {
            frame: Some(Frame {
                zoom: 175,
                x: 230,
                y: 710,
                width: 1920,
                height: 1080,
            }),
            flip: Flip { h: true, v: false },
            ..Default::default()
        },
    );
    store.save_state(&state).unwrap();
    let preset = store.save_current_preset("Saved").unwrap();
    let mut state = store.load_state().unwrap();
    state.adjust.clear();
    for name in ["DP-1", "DP-2", "other"] {
        state.current.insert(name.into(), dir.path().join("old.png"));
    }
    store.save_state(&state).unwrap();
    let outputs = parse_monitors(
        r#"[
        {"name":"DP-2","width":16,"height":12,"x":16,"scale":1.5},
        {"name":"DP-1","width":16,"height":12,"transform":1,"scale":1.5},
        {"name":"other","width":16,"height":12,"x":32}
    ]"#,
    )
    .unwrap();
    (
        dir,
        store,
        Desktop {
            outputs,
            calls: RefCell::default(),
            fail_on: None,
        },
        preset,
    )
}

#[test]
fn application_restores_geometry_and_history_without_touching_unselected_outputs() {
    let (_dir, store, desktop, preset) = setup();
    let before = store.load_state().unwrap();
    let report = apply(&desktop, &store, &preset.id).unwrap();
    assert_eq!(report.applied, ["DP-1", "DP-2"]);
    assert!(report.failed.is_empty() && report.skipped.is_empty());
    let state = store.load_state().unwrap();
    let frame = state.adjust["DP-1"].frame.unwrap();
    assert_eq!(
        (frame.width, frame.height, frame.zoom, frame.x, frame.y),
        (12, 16, 175, 230, 710)
    );
    assert!(state.adjust["DP-1"].flip.h);
    assert_eq!(state.history["DP-1"], [before.current["DP-1"].clone()]);
    assert_eq!(state.current["other"], before.current["other"]);
    assert_eq!(state.current["DP-1"], preset.monitors["DP-1"].wallpaper);
    let calls = desktop.calls.borrow();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].1.last().unwrap().contains("transformed"));
    drop(calls);
    apply(&desktop, &store, &preset.id).unwrap();
    assert_eq!(store.load_state().unwrap().history, state.history);
}

#[test]
fn missing_and_corrupt_selected_sources_fail_before_backend_or_state_changes() {
    for corrupt in [false, true] {
        let (_dir, store, desktop, preset) = setup();
        let path = &preset.monitors["DP-1"].wallpaper;
        if corrupt {
            std::fs::write(path, "broken png").unwrap();
        } else {
            std::fs::remove_file(path).unwrap();
        }
        let before = store.load_state().unwrap();
        assert!(apply(&desktop, &store, &preset.id).is_err());
        assert!(desktop.calls.borrow().is_empty());
        assert_eq!(store.load_state().unwrap(), before);
    }
}

#[test]
fn disconnected_assignments_are_retained_and_skipped() {
    let (_dir, store, mut desktop, preset) = setup();
    desktop.outputs.retain(|m| m.name != "DP-1");
    let before = store.load_state().unwrap();
    let report = apply(&desktop, &store, &preset.id).unwrap();
    assert_eq!(report.applied, ["DP-2"]);
    assert_eq!(report.skipped[0].0, "DP-1");
    assert_eq!(store.load_state().unwrap().current["DP-1"], before.current["DP-1"]);
    assert_eq!(store.load_presets().unwrap(), vec![preset]);
}

#[test]
fn failed_output_keeps_its_state_and_successful_output_is_remembered() {
    let (_dir, store, mut desktop, preset) = setup();
    desktop.fail_on = Some("DP-2".into());
    let before = store.load_state().unwrap();
    let report = apply(&desktop, &store, &preset.id).unwrap();
    assert_eq!(report.applied, ["DP-1"]);
    assert_eq!(report.failed[0].0, "DP-2");
    let state = store.load_state().unwrap();
    assert_eq!(state.current["DP-2"], before.current["DP-2"]);
    assert!(!state.adjust.contains_key("DP-2"));
    assert_eq!(state.current["DP-1"], preset.monitors["DP-1"].wallpaper);
}

#[test]
fn explicit_backend_default_survives_future_global_fits_and_recapture() {
    let (_dir, store, desktop, preset) = setup();
    let settings = Settings {
        fit_mode: Some(FitMode::Tile),
        ..Default::default()
    };
    store.save_settings(&settings).unwrap();
    apply(&desktop, &store, &preset.id).unwrap();
    let source = &preset.monitors["DP-2"].wallpaper;
    assert_eq!(crate::wallpapers::Fits::load(&store).of("DP-2", source), None);
    assert_eq!(store.save_current_preset("Again").unwrap().monitors["DP-2"].fit, None);
    assert_eq!(store.load_settings().unwrap().fit_mode, Some(FitMode::Tile));
    assert!(store.load_state().unwrap().adjust["DP-2"].backend_default_fit);
}

#[test]
fn standalone_source_regions_rebuild_output_dimensions() {
    let (_dir, store, desktop, mut preset) = setup();
    preset.monitors.get_mut("DP-1").unwrap().frame = None;
    preset.monitors.get_mut("DP-1").unwrap().region = Some(SourceRegion {
        x: 4,
        y: 2,
        width: 20,
        height: 10,
    });
    let plan = plan::prepare(&store, &preset, &desktop.outputs).unwrap();
    let entry = plan.entries.iter().find(|e| e.monitor == "DP-1").unwrap();
    let cut = entry.adjust.span.unwrap();
    assert_eq!((cut.out_width, cut.out_height, cut.x, cut.y), (12, 16, 4, 2));
    assert_eq!(image::image_dimensions(entry.picture.static_file().unwrap().0).unwrap(), (12, 16));
    preset.monitors.get_mut("DP-1").unwrap().region.as_mut().unwrap().width = 100;
    assert!(plan::prepare(&store, &preset, &desktop.outputs).is_err());
}

#[test]
fn no_matching_displays_does_not_change_anything() {
    let (_dir, store, mut desktop, preset) = setup();
    desktop.outputs.clear();
    let before = store.load_state().unwrap();
    let report = apply(&desktop, &store, &preset.id).unwrap();
    assert_eq!(report.skipped.len(), 2);
    assert!(report.applied.is_empty() && report.failed.is_empty());
    assert!(desktop.calls.borrow().is_empty());
    assert_eq!(store.load_state().unwrap(), before);
    assert!(apply(&desktop, &store, "missing").is_err());
}

#[test]
fn a_bad_second_image_prevents_applying_the_good_first_image() {
    let (_dir, store, desktop, mut preset) = setup();
    let bad = store.dirs().data.join("bad.png");
    std::fs::create_dir_all(&store.dirs().data).unwrap();
    std::fs::write(&bad, "not an image").unwrap();
    preset.monitors.get_mut("DP-2").unwrap().wallpaper = bad;
    let before = store.load_state().unwrap();
    assert!(plan::prepare(&store, &preset, &desktop.outputs).is_err());
    assert!(desktop.calls.borrow().is_empty());
    assert_eq!(store.load_state().unwrap(), before);
}
