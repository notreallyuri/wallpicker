use super::*;
use crate::{
    presets::{Collection, MonitorWallpaper, Preset, PresetFrame},
    settings::{Flip, State},
    store::{Dirs, Store},
};
use std::fs;

fn saved() -> Collection {
    Collection {
        version: 1,
        next_id: 4,
        items: vec![Preset {
            id: "p3".into(),
            name: "Night".into(),
            span: None,
            monitors: [(
                "DP-1".into(),
                MonitorWallpaper {
                    wallpaper: "/old/a.jpg".into(),
                    fit: None,
                    flip: Flip { h: true, v: false },
                    frame: Some(PresetFrame {
                        zoom: 150,
                        x: 250,
                        y: 750,
                    }),
                    region: None,
                },
            )]
            .into(),
        }],
    }
}

#[test]
fn relocation_preserves_ids_and_adjustments_and_reports_missing() {
    let mut backup = export(&Catalog::default(), &["/old".into()], String::new(), &|_| Some(10));
    backup.include_presets(saved(), &|_| Some(10)).unwrap();
    let mut report = ImportReport::default();
    let result = backup
        .import_presets(
            &Collection::default(),
            &["/new".into()],
            &[("/new/a.jpg".into(), 10)],
            true,
            &mut report,
        )
        .unwrap()
        .unwrap();
    assert_eq!(result.items[0].id, "p3");
    assert_eq!(result.next_id, 4);
    assert_eq!(
        result.items[0].monitors["DP-1"].frame,
        saved().items[0].monitors["DP-1"].frame
    );
    assert_eq!(result.items[0].monitors["DP-1"].wallpaper, Path::new("/new/a.jpg"));
    assert_eq!(report.preset_images_moved, 1);
    let missing = backup
        .import_presets(&Collection::default(), &[], &[], true, &mut report)
        .unwrap()
        .unwrap();
    assert_eq!(missing, saved());
    assert_eq!(report.preset_images_missing, ["/old/a.jpg"]);
}

#[test]
fn invalid_and_conflicting_imports_leave_both_files_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    store
        .update_catalog(|c| c.set_favorite(Path::new("/current.jpg"), true))
        .unwrap();
    fs::write(store.presets_path(), serde_json::to_vec(&saved()).unwrap()).unwrap();
    let before = (
        fs::read(store.dirs().data.join("library.json")).unwrap(),
        fs::read(store.presets_path()).unwrap(),
    );
    let mut backup = store.export_backup(String::new()).unwrap();
    backup.presets.as_mut().unwrap().items[0].name = "Different".into();
    assert!(store.import_backup(&backup, &[], &[], false).is_err());
    backup.presets.as_mut().unwrap().next_id = 0;
    assert!(store.import_backup(&backup, &[], &[], true).is_err());
    assert_eq!(before.0, fs::read(store.dirs().data.join("library.json")).unwrap());
    assert_eq!(before.1, fs::read(store.presets_path()).unwrap());
}

#[test]
fn daily_snapshots_cover_preset_edits_and_legacy_restores_preserve_presets() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let mut state = State::default();
    state.current.insert("DP-1".into(), "/old/a.jpg".into());
    store.save_state(&state).unwrap();
    let preset = store.save_current_preset("First").unwrap();
    store.rename_preset(&preset.id, "Renamed").unwrap();
    let snapshot = store.snapshots().unwrap().remove(0);
    assert_eq!(snapshot.presets, Some(1));
    store.restore_snapshot(&snapshot.id).unwrap();
    assert_eq!(store.load_presets().unwrap()[0].name, "First");
    let old = export(&Catalog::default(), &[], String::new(), &|_| None);
    store.import_backup(&old, &[], &[], true).unwrap();
    assert_eq!(store.load_presets().unwrap()[0].name, "First");
    fs::write(store.snapshots_dir().join("library-2000-01-01.json"), r#"{"items":{}}"#).unwrap();
    store.restore_snapshot("library-2000-01-01").unwrap();
    assert_eq!(store.load_presets().unwrap()[0].name, "First");
}

#[test]
fn span_relocation_updates_canvas_and_every_member_together() {
    use crate::{
        presets::PresetSpan,
        settings::{SpanLook, SpanPoint},
    };
    let mut collection = saved();
    let preset = &mut collection.items[0];
    let monitor = preset.monitors.get_mut("DP-1").unwrap();
    monitor.frame = None;
    monitor.flip = Flip::default();
    let second = monitor.clone();
    preset.monitors.insert("DP-2".into(), second);
    preset.span = Some(PresetSpan {
        wallpaper: "/old/a.jpg".into(),
        monitors: vec!["DP-1".into(), "DP-2".into()],
        look: SpanLook::default(),
        bezel: 12,
        layout: [
            ("DP-1".into(), SpanPoint { x: 0.0, y: 0.0 }),
            ("DP-2".into(), SpanPoint { x: 500.0, y: 0.0 }),
        ]
        .into(),
    });
    let mut backup = export(&Catalog::default(), &["/old".into()], String::new(), &|_| Some(10));
    backup.include_presets(collection, &|_| Some(10)).unwrap();
    let restored = backup
        .import_presets(
            &Collection::default(),
            &["/new".into()],
            &[("/new/a.jpg".into(), 10)],
            true,
            &mut ImportReport::default(),
        )
        .unwrap()
        .unwrap();
    let preset = &restored.items[0];
    assert_eq!(preset.span.as_ref().unwrap().wallpaper, Path::new("/new/a.jpg"));
    assert!(preset.monitors.values().all(|m| m.wallpaper == Path::new("/new/a.jpg")));
    assert_eq!(preset.span.as_ref().unwrap().bezel, 12);
}

#[test]
fn second_write_failure_rolls_back_library() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    store
        .update_catalog(|c| c.set_favorite(Path::new("/current.jpg"), true))
        .unwrap();
    let path = store.dirs().data.join("library.json");
    let original = fs::read(&path).unwrap();
    fs::create_dir(store.presets_path()).unwrap();
    assert!(store.write_backup_data(&Catalog::default(), Some(&saved())).is_err());
    assert_eq!(fs::read(path).unwrap(), original);
}
