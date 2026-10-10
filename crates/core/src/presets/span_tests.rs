use super::*;
use crate::{
    Result,
    cmd::Runner,
    monitors::{Monitor, parse_monitors},
    settings::{Adjust, Cut, Settings, SpanRect, Spanned, State},
    store::{Dirs, Store},
};
use std::{cell::RefCell, path::PathBuf};
use tempfile::TempDir;

struct Outputs {
    monitors: Vec<Monitor>,
    fail: bool,
    calls: RefCell<Vec<String>>,
}

impl Runner for Outputs {
    fn monitors(&self) -> Result<Vec<Monitor>> {
        Ok(self.monitors.clone())
    }
    fn run(&self, _: &str, args: &[&str]) -> Result<String> {
        self.calls.borrow_mut().push(args.join(" "));
        if self.fail && args.iter().any(|a| a.starts_with("DP-2,")) {
            Err(crate::Error::Invalid("deliberate failure".into()))
        } else {
            Ok(String::new())
        }
    }
}

fn setup() -> (TempDir, Store, Outputs, Preset) {
    let dir = TempDir::new().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let path = dir.path().join("span.png");
    image::RgbImage::from_pixel(64, 32, image::Rgb([0, 255, 0])).save(&path).unwrap();
    let mut state = State::default();
    let mut places = std::collections::BTreeMap::new();
    for (name, x) in [("DP-1", 0.0), ("DP-2", 600.0)] {
        state.current.insert(name.into(), path.clone());
        state.adjust.insert(
            name.into(),
            Adjust {
                span: Some(Cut { x: 0, y: 0, width: 32, height: 32, out_width: 16, out_height: 16 }),
                ..Default::default()
            },
        );
        places.insert(name.into(), SpanRect { x, y: 0.0, w: 600.0, h: 340.0 });
    }
    let extra = dir.path().join("other.png");
    image::RgbImage::from_pixel(32, 32, image::Rgb([255, 0, 0])).save(&extra).unwrap();
    state.current.insert("HDMI-A-1".into(), extra);
    state.span =
        Some(Spanned { path, monitors: vec!["DP-1".into(), "DP-2".into()], places, look: SpanLook::default() });
    store.save_state(&state).unwrap();
    let preset = store.save_current_preset("Span").unwrap();
    let mut current = store.load_state().unwrap();
    current.span = None;
    current.adjust.clear();
    for path in current.current.values_mut() {
        *path = PathBuf::from("old.png");
    }
    store.save_state(&current).unwrap();
    let monitors = parse_monitors(
        r#"[
        {"name":"DP-2","width":24,"height":16,"x":0,"scale":1.5,"physicalWidth":600,"physicalHeight":340},
        {"name":"DP-1","width":24,"height":16,"x":24,"transform":1,"physicalWidth":600,"physicalHeight":340},
        {"name":"HDMI-A-1","width":24,"height":16,"x":48}
    ]"#,
    )
    .unwrap();
    (dir, store, Outputs { monitors, fail: false, calls: RefCell::default() }, preset)
}

#[test]
fn span_restores_saved_canvas_and_rebuilds_cuts_for_current_rotations() {
    let (_dir, store, outputs, preset) = setup();
    let mut settings = store.load_settings().unwrap();
    settings.span_layout.insert("DP-1".into(), SpanPoint { x: 999.0, y: 999.0 });
    settings.span_bezel = 20;
    store.save_settings(&settings).unwrap();
    let report = apply(&outputs, &store, &preset.id).unwrap();
    assert_eq!(report.applied.len(), 3);
    let state = store.load_state().unwrap();
    let span = state.span.unwrap();
    assert_eq!(span.monitors, ["DP-1", "DP-2"]);
    assert_eq!(span.places["DP-1"].x, 0.0);
    assert_eq!(span.places["DP-2"].x, 600.0);
    let cut = state.adjust["DP-1"].span.unwrap();
    assert_eq!((cut.out_width, cut.out_height), (16, 24));
    let settings = store.load_settings().unwrap();
    assert_eq!(settings.span_layout["DP-1"], SpanPoint { x: 0.0, y: 0.0 });
    assert_eq!(settings.span_bezel, 0);
    assert_eq!(settings.span_monitors, ["DP-1", "DP-2"]);
}

#[test]
fn incomplete_span_is_skipped_as_a_group_without_falling_back_to_other_outputs() {
    let (_dir, store, mut outputs, preset) = setup();
    outputs.monitors.retain(|m| m.name != "DP-2");
    // A missing image for the skipped group must not block independent assignments.
    std::fs::remove_file(&preset.span.as_ref().unwrap().wallpaper).unwrap();
    let before = store.load_state().unwrap();
    let settings = store.load_settings().unwrap();
    let report = apply(&outputs, &store, &preset.id).unwrap();
    assert_eq!(report.applied, ["HDMI-A-1"]);
    assert_eq!(report.skipped.len(), 2);
    let state = store.load_state().unwrap();
    assert_eq!(state.current["DP-1"], before.current["DP-1"]);
    assert_eq!(state.current["DP-2"], before.current["DP-2"]);
    assert!(state.span.is_none());
    assert_eq!(store.load_settings().unwrap(), settings);
}

#[test]
fn partial_span_failure_leaves_successful_parts_separate_and_does_not_claim_failed_output() {
    let (_dir, store, mut outputs, preset) = setup();
    outputs.fail = true;
    let before = store.load_state().unwrap();
    let settings = Settings { span_bezel: 12, ..Default::default() };
    store.save_settings(&settings).unwrap();
    let report = apply(&outputs, &store, &preset.id).unwrap();
    assert_eq!(report.applied, ["DP-1", "HDMI-A-1"]);
    assert_eq!(report.failed[0].0, "DP-2");
    let state = store.load_state().unwrap();
    assert!(state.span.is_none());
    assert!(state.adjust["DP-1"].span.is_some());
    assert_eq!(state.current["DP-2"], before.current["DP-2"]);
    assert!(!state.adjust.contains_key("DP-2"));
    assert_eq!(store.load_settings().unwrap().span_bezel, 12);
}
