use super::*;
use crate::settings::{Adjust, Cut, Frame, Settings, SpanRect, Spanned, State};
use crate::store::{Dirs, Store};
use std::fs;
use tempfile::TempDir;

fn fixture() -> (TempDir, Store) {
    let dir = TempDir::new().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let mut state = State::default();
    state.current.insert("DP-1".into(), dir.path().join("first.png"));
    state.current.insert("DP-2".into(), dir.path().join("second.png"));
    state.history.insert("DP-1".into(), vec![dir.path().join("old.png")]);
    state.slideshow_changed = 123;
    store.save_state(&state).unwrap();
    (dir, store)
}

#[test]
fn snapshot_keeps_sources_and_adjustments_without_mutating_live_state() {
    let (_dir, store) = fixture();
    let mut settings = Settings::default();
    settings.fit_mode = Some(FitMode::Cover);
    settings.monitor_fits.insert("DP-2".into(), FitMode::Contain);
    store.save_settings(&settings).unwrap();
    let mut state = store.load_state().unwrap();
    state.adjust.insert(
        "DP-1".into(),
        Adjust {
            fit: Some(FitMode::Tile),
            flip: Flip { h: true, v: true },
            frame: Some(Frame { zoom: 175, x: 230, y: 710, width: 1920, height: 1080 }),
            ..Default::default()
        },
    );
    store.save_state(&state).unwrap();
    let state_bytes = fs::read(store.dirs().state.join("state.json")).unwrap();
    let settings_bytes = fs::read(store.dirs().config.join("config.json")).unwrap();
    let preset = store.save_current_preset("  Night  ").unwrap();
    assert_eq!(preset.name, "Night");
    assert_eq!(preset.monitors["DP-1"].wallpaper, state.current["DP-1"]);
    assert_eq!(preset.monitors["DP-1"].frame, Some(PresetFrame { zoom: 175, x: 230, y: 710 }));
    assert_eq!(preset.monitors["DP-1"].flip, Flip { h: true, v: true });
    assert_eq!(preset.monitors["DP-1"].fit, Some(FitMode::Tile));
    assert_eq!(preset.monitors["DP-2"].fit, Some(FitMode::Contain));
    assert_eq!(fs::read(store.dirs().state.join("state.json")).unwrap(), state_bytes);
    assert_eq!(fs::read(store.dirs().config.join("config.json")).unwrap(), settings_bytes);
    let text = fs::read_to_string(store.presets_path()).unwrap();
    assert!(!text.contains("history") && !text.contains("slideshow") && !text.contains("1920"));
    settings.fit_mode = Some(FitMode::Fill);
    settings.monitor_fits.clear();
    store.save_settings(&settings).unwrap();
    assert_eq!(store.load_presets().unwrap(), vec![preset]);
}

#[test]
fn rename_update_and_delete_keep_ids_stable_and_never_reuse_them() {
    let (_dir, store) = fixture();
    let first = store.save_current_preset("First").unwrap();
    let second = store.save_current_preset("Second").unwrap();
    assert_ne!(first.id, second.id);
    let renamed = store.rename_preset(&first.id, "Renamed").unwrap();
    assert_eq!(renamed.id, first.id);
    let mut state = store.load_state().unwrap();
    state.current.insert("DP-1".into(), PathBuf::from("changed.png"));
    store.save_state(&state).unwrap();
    assert_eq!(store.load_presets().unwrap()[0], renamed);
    let updated = store.update_current_preset(&first.id).unwrap();
    assert_eq!(updated.id, first.id);
    assert_eq!(updated.name, "Renamed");
    assert_eq!(updated.monitors["DP-1"].wallpaper, PathBuf::from("changed.png"));
    store.delete_preset(&second.id).unwrap();
    let third = store.save_current_preset("Third").unwrap();
    assert_ne!(third.id, second.id);
    assert!(store.rename_preset("missing", "New").is_err());
    assert!(store.update_current_preset("missing").is_err());
    assert!(store.delete_preset("missing").is_err());
}

#[test]
fn rejects_bad_and_duplicate_names_without_changing_collection() {
    let (_dir, store) = fixture();
    let first = store.save_current_preset("Night").unwrap();
    let second = store.save_current_preset("Day").unwrap();
    let before = fs::read(store.presets_path()).unwrap();
    for name in ["", "  ", "bad\nname", "bad\0name", &"a".repeat(101), "night"] {
        assert!(store.save_current_preset(name).is_err(), "{name:?}");
    }
    assert!(store.rename_preset(&second.id, "NIGHT").is_err());
    assert_eq!(fs::read(store.presets_path()).unwrap(), before);
    assert_eq!(store.rename_preset(&first.id, "NIGHT").unwrap().id, first.id);
}

#[test]
fn missing_collection_is_empty_and_empty_setup_cannot_be_saved() {
    let dir = TempDir::new().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    assert!(store.load_presets().unwrap().is_empty());
    assert!(store.save_current_preset("Empty").is_err());
    assert!(!store.presets_path().exists());
}

#[test]
fn malformed_and_future_versions_are_preserved() {
    let (_dir, store) = fixture();
    fs::create_dir_all(&store.dirs().data).unwrap();
    for text in ["not json", r#"{"version":99,"next_id":1,"items":[]}"#, r#"{"version":1,"next_id":0,"items":[]}"#] {
        fs::write(store.presets_path(), text).unwrap();
        assert!(store.load_presets().is_err());
        assert!(store.save_current_preset("New").is_err());
        assert_eq!(fs::read_to_string(store.presets_path()).unwrap(), text);
    }
}

fn spanned_state() -> State {
    let path = PathBuf::from("original.png");
    let mut state = State::default();
    for (name, x) in [("DP-1", -600.0), ("DP-2", 0.0)] {
        state.current.insert(name.into(), path.clone());
        state.adjust.insert(
            name.into(),
            Adjust {
                flip: Flip { h: true, v: false },
                span: Some(Cut {
                    x: if x < 0.0 { 0 } else { 1000 },
                    y: 20,
                    width: 1000,
                    height: 500,
                    out_width: 1920,
                    out_height: 1080,
                }),
                ..Default::default()
            },
        );
    }
    state.current.insert("HDMI-A-1".into(), PathBuf::from("independent.png"));
    state.span = Some(Spanned {
        path,
        look: SpanLook { zoom: 140, x: 300, y: 600, flip: Flip { h: true, v: false } },
        monitors: vec!["DP-1".into(), "DP-2".into()],
        places: [
            ("DP-1".into(), SpanRect { x: -600.0, y: 20.0, w: 600.0, h: 340.0 }),
            ("DP-2".into(), SpanRect { x: 0.0, y: 20.0, w: 600.0, h: 340.0 }),
        ]
        .into(),
    });
    state
}

#[test]
fn mixed_span_snapshot_uses_saved_canvas_and_omits_derived_cuts() {
    let state = spanned_state();
    let settings = Settings { span_bezel: 8, ..Default::default() };
    let preset = capture::capture("p1".into(), "Span".into(), &state, &settings).unwrap();
    let span = preset.span.as_ref().unwrap();
    assert_eq!(span.layout["DP-1"], SpanPoint { x: -600.0, y: 20.0 });
    assert_eq!(span.bezel, 8);
    assert_eq!(span.look, state.span.as_ref().unwrap().look);
    assert_eq!(span.monitors, ["DP-1", "DP-2"]);
    assert!(preset.monitors["DP-1"].region.is_none());
    assert_eq!(preset.monitors["HDMI-A-1"].wallpaper, PathBuf::from("independent.png"));
    assert_eq!(serde_json::from_str::<Preset>(&serde_json::to_string(&preset).unwrap()).unwrap(), preset);
}

#[test]
fn legacy_span_members_are_explicit_and_separated_regions_keep_only_source_coordinates() {
    let mut state = spanned_state();
    state.span.as_mut().unwrap().monitors.clear();
    let preset = capture::capture("p1".into(), "Legacy".into(), &state, &Settings::default()).unwrap();
    assert_eq!(preset.span.unwrap().monitors, ["DP-1", "DP-2"]);
    state.span = None;
    let preset = capture::capture("p1".into(), "Separate".into(), &state, &Settings::default()).unwrap();
    assert!(preset.span.is_none());
    assert_eq!(preset.monitors["DP-2"].region, Some(SourceRegion { x: 1000, y: 20, width: 1000, height: 500 }));
}

#[test]
fn invalid_span_and_generated_cache_sources_are_rejected() {
    let (_dir, store) = fixture();
    let mut state = spanned_state();
    state.span.as_mut().unwrap().monitors.push("missing".into());
    assert!(capture::capture("p1".into(), "Bad".into(), &state, &Settings::default()).is_err());
    state = store.load_state().unwrap();
    state.current.insert("DP-1".into(), store.dirs().cache.join("derived.png"));
    store.save_state(&state).unwrap();
    assert!(store.save_current_preset("Cache").is_err());
}

#[test]
fn concurrent_writers_keep_every_snapshot_and_unique_ids() {
    let (_dir, store) = fixture();
    std::thread::scope(|scope| {
        for i in 0..8 {
            let store = &store;
            scope.spawn(move || store.save_current_preset(&format!("Preset {i}")).unwrap());
        }
    });
    let items = store.load_presets().unwrap();
    assert_eq!(items.len(), 8);
    assert_eq!(items.iter().map(|p| &p.id).collect::<std::collections::BTreeSet<_>>().len(), 8);
}

#[test]
fn invalid_stored_ids_names_and_geometry_are_not_overwritten() {
    let (_dir, store) = fixture();
    store.save_current_preset("Valid").unwrap();
    let original: serde_json::Value = serde_json::from_slice(&fs::read(store.presets_path()).unwrap()).unwrap();
    for kind in ["id", "duplicate", "name", "frame", "region"] {
        let mut value = original.clone();
        match kind {
            "id" => value["items"][0]["id"] = "p999".into(),
            "duplicate" => {
                let item = value["items"][0].clone();
                value["items"].as_array_mut().unwrap().push(item);
            }
            "name" => value["items"][0]["name"] = "  untrimmed  ".into(),
            "frame" => {
                value["items"][0]["monitors"]["DP-1"]["frame"] = serde_json::json!({"zoom": 0, "x": 500, "y": 500})
            }
            "region" => {
                value["items"][0]["monitors"]["DP-1"]["region"] =
                    serde_json::json!({"x": u32::MAX, "y": 0, "width": 10, "height": 10})
            }
            _ => unreachable!(),
        }
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(store.presets_path(), &bytes).unwrap();
        assert!(store.load_presets().is_err(), "{kind}");
        assert!(store.save_current_preset("New").is_err(), "{kind}");
        assert_eq!(fs::read(store.presets_path()).unwrap(), bytes);
    }
}
