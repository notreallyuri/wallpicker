use super::*;
use crate::{
    settings::{SpanLook, SpanPoint, State},
    store::{Dirs, Store},
};
use std::{collections::BTreeMap, fs};

fn fixture() -> Preset {
    let wallpaper = MonitorWallpaper {
        wallpaper: "/source.jpg".into(),
        fit: Some(FitMode::Cover),
        flip: Flip::default(),
        frame: None,
        region: None,
    };
    Preset {
        id: "p1".into(),
        name: "Desk".into(),
        monitors: [
            ("DP-1".into(), wallpaper.clone()),
            ("DP-2".into(), wallpaper.clone()),
            ("HDMI-1".into(), wallpaper),
        ]
        .into(),
        span: Some(PresetSpan {
            wallpaper: "/source.jpg".into(),
            monitors: vec!["DP-1".into(), "DP-2".into()],
            look: SpanLook::default(),
            bezel: 8,
            layout: [
                ("DP-1".into(), SpanPoint { x: 0.0, y: 0.0 }),
                ("DP-2".into(), SpanPoint { x: 500.0, y: 10.0 }),
            ]
            .into(),
        }),
    }
}

#[test]
fn swaps_span_references_simultaneously_and_keeps_independent_adjustments() {
    let mut preset = fixture();
    let standalone = preset.monitors.get_mut("HDMI-1").unwrap();
    standalone.flip.h = true;
    standalone.frame = Some(PresetFrame {
        zoom: 230,
        x: 100,
        y: 800,
    });
    let mapping = [
        ("DP-1".into(), "DP-2".into()),
        ("DP-2".into(), "DP-1".into()),
        ("HDMI-1".into(), "DP-3".into()),
    ]
    .into();
    let result = reassign(&preset, &mapping).unwrap();
    assert_eq!(result.id, preset.id);
    assert_eq!(result.name, preset.name);
    assert_eq!(result.monitors["DP-3"], preset.monitors["HDMI-1"]);
    let span = result.span.unwrap();
    assert_eq!(span.monitors, ["DP-2", "DP-1"]);
    assert_eq!(span.layout["DP-1"], preset.span.as_ref().unwrap().layout["DP-2"]);
    assert_eq!(span.bezel, 8);
}

#[test]
fn rejects_unknown_sources_collisions_and_invalid_targets() {
    let preset = fixture();
    for pairs in [
        vec![("unknown", "DP-3")],
        vec![("DP-1", "DP-2")],
        vec![("DP-1", "DP-3"), ("DP-2", "DP-3")],
        vec![("DP-1", "")],
        vec![("DP-1", " padded ")],
        vec![("DP-1", "bad\nname")],
    ] {
        let mapping = pairs.into_iter().map(|(a, b)| (a.into(), b.into())).collect();
        assert!(reassign(&preset, &mapping).is_err());
    }
    assert_eq!(reassign(&preset, &BTreeMap::new()).unwrap(), preset);
}

#[test]
fn preview_does_not_write_and_save_preserves_live_state_and_rejects_stale_edits() {
    let root = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(root.path()));
    let mut state = State::default();
    state.current.insert("DP-1".into(), "/source.jpg".into());
    store.save_state(&state).unwrap();
    let original = store.save_current_preset("Desk").unwrap();
    let before = fs::read(store.presets_path()).unwrap();
    let mapping = [("DP-1".into(), "DP-3".into())].into();
    let preview = store.preview_preset_reassignment(&original.id, &mapping).unwrap();
    assert!(preview.monitors.contains_key("DP-3"));
    assert_eq!(fs::read(store.presets_path()).unwrap(), before);
    let saved = store.reassign_preset(&original.id, &mapping, Some(&original)).unwrap();
    assert_eq!(saved, preview);
    assert_eq!(store.load_state().unwrap().current, state.current);
    assert_eq!(store.preset_collection().unwrap().next_id, 2);
    assert_eq!(store.snapshots().unwrap()[0].presets, Some(1));
    assert!(store.reassign_preset(&original.id, &mapping, Some(&original)).is_err());
    assert_eq!(store.load_presets().unwrap(), [saved]);
}
