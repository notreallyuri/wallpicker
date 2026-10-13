use super::*;
use crate::{
    cmd::Runner,
    settings::{Adjust, Cut, Flip, Frame},
    store::{Dirs, Store},
    wallpapers,
};
use std::{cell::RefCell, path::Path};

#[derive(Default)]
pub(super) struct Desktop {
    batches: RefCell<Vec<Batch>>,
    fail: RefCell<Vec<String>>,
    pub(super) spawns: RefCell<u32>,
}

impl Runner for Desktop {
    fn run(&self, _: &str, _: &[&str]) -> Result<String> {
        Ok(
            r#"[{"name":"DP-1","width":8,"height":4,"x":0,"y":0},{"name":"DP-2","width":8,"height":4,"x":8,"y":0}]"#
                .into(),
        )
    }
    fn render(&self, batch: &Batch) -> Result<Report> {
        self.batches.borrow_mut().push(batch.clone());
        let mut report = Report::default();
        for output in &batch.outputs {
            if self.fail.borrow().contains(&output.monitor) {
                report.failed.push((output.monitor.clone(), "failed".into()));
            } else {
                report.applied.push(output.monitor.clone());
            }
        }
        Ok(report)
    }
    fn spawn(&self, _: &str, _: &[&str]) -> Result<()> {
        *self.spawns.borrow_mut() += 1;
        Ok(())
    }
}

fn picture(root: &Path, name: &str) -> PathBuf {
    let path = root.join(name);
    image::RgbImage::from_fn(16, 8, |x, _| {
        if x < 8 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    })
    .save(&path)
    .unwrap();
    path
}

fn animation(path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut encoder = image::codecs::gif::GifEncoder::new(file);
    for color in [[255, 0, 0, 255], [0, 0, 255, 255]] {
        encoder
            .encode_frame(image::Frame::new(image::RgbaImage::from_pixel(
                16,
                8,
                image::Rgba(color),
            )))
            .unwrap();
    }
}

#[test]
fn source_geometry_survives_serialization_and_animation_never_creates_a_static_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("moving.webp"); // content is GIF
    animation(&path);
    let adjust = Adjust {
        flip: Flip { h: true, v: false },
        frame: Some(Frame {
            zoom: 150,
            x: 400,
            y: 600,
            width: 8,
            height: 4,
        }),
        ..Default::default()
    };
    let cache = dir.path().join("cache");
    let pic = Preparation::default()
        .picture(&cache, &path, &adjust, Some(FitMode::Cover))
        .unwrap();
    assert_eq!(pic.source, path);
    assert!(matches!(pic.content, Content::Animated { .. }));
    assert!(!cache.exists());
    assert_eq!(
        pic.geometry.view,
        View::Frame {
            zoom: 150,
            x: 400,
            y: 600
        }
    );
    let batch = Batch::new(
        Intent::Apply,
        Default::default(),
        vec![Output {
            monitor: "DP-1".into(),
            picture: pic,
        }],
    );
    let decoded: Batch = serde_json::from_value(serde_json::to_value(&batch).unwrap()).unwrap();
    decoded.validate().unwrap();
    assert_eq!(decoded.outputs, batch.outputs);
    assert_eq!(decoded.application, batch.application);
    // Never flattened into a cached copy: the renderer gets the source itself.
    assert!(
        decoded.outputs[0]
            .picture
            .static_file()
            .unwrap_err()
            .to_string()
            .contains(PLAYBACK_UNAVAILABLE)
    );
}

#[test]
fn all_outputs_share_a_batch_reapplication_restarts_and_adjustments_are_explicit() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let path = picture(dir.path(), "image.png");
    let desktop = Desktop::default();
    let outputs = ["DP-1".into(), "DP-2".into()];
    wallpapers::apply_and_remember(&desktop, &store, &outputs, &path).unwrap();
    wallpapers::reapply(&desktop, &store, &|_, _| true).unwrap();
    wallpapers::apply_and_remember(&desktop, &store, &outputs, &path).unwrap();
    let batches = desktop.batches.borrow();
    assert_eq!(batches.len(), 3);
    assert!(batches.iter().all(|b| b.outputs.len() == 2));
    assert_eq!(batches[0].intent, Intent::Apply);
    assert_eq!(batches[1].intent, Intent::Adjust);
    assert_eq!(batches[1].transition.kind, "none");
    assert_ne!(batches[0].application, batches[2].application);
    assert!(
        batches
            .iter()
            .flat_map(|b| &b.outputs)
            .all(|o| o.picture.source == path)
    );
}

#[test]
fn corrupt_sources_fail_their_monitor_and_animated_sources_reach_the_renderer() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let original = picture(dir.path(), "old.png");
    store.set_current("DP-1", &original).unwrap();
    let before = std::fs::read(store.dirs().state.join("state.json")).unwrap();
    let desktop = Desktop::default();
    let path = dir.path().join("bad.webp");
    std::fs::write(&path, b"broken").unwrap();
    let report = wallpapers::apply_and_remember(&desktop, &store, &["DP-1".into()], &path).unwrap();
    assert!(report.applied.is_empty());
    assert_eq!(report.failed[0].0, "DP-1");
    assert!(desktop.batches.borrow().is_empty());
    assert_eq!(std::fs::read(store.dirs().state.join("state.json")).unwrap(), before);
    // The request keeps the animation, never a flattened copy: the daemon
    // decides whether it plays or shows its first frame.
    animation(&path);
    let report = wallpapers::apply_and_remember(&desktop, &store, &["DP-1".into()], &path).unwrap();
    assert_eq!(report.applied, ["DP-1"]);
    let batches = desktop.batches.borrow();
    assert!(matches!(batches[0].outputs[0].picture.content, Content::Animated { .. }));
    assert_eq!(store.load_state().unwrap().current["DP-1"], path);
}

#[test]
fn partial_span_commits_only_successful_cuts_and_preserves_failed_history() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let old = picture(dir.path(), "old.png");
    let new = picture(dir.path(), "new.png");
    store.set_current("DP-1", &old).unwrap();
    store.set_current("DP-2", &old).unwrap();
    let adjust = Adjust {
        flip: Flip { h: false, v: true },
        ..Default::default()
    };
    store.set_adjust("DP-2", adjust).unwrap();
    let before = store.load_state().unwrap();
    let desktop = Desktop::default();
    desktop.fail.borrow_mut().push("DP-2".into());
    let report = wallpapers::span_and_remember(&desktop, &store, &new, Default::default(), Default::default()).unwrap();
    assert_eq!(report.applied, ["DP-1"]);
    assert_eq!(desktop.batches.borrow().len(), 1);
    let state = store.load_state().unwrap();
    assert_eq!(state.current["DP-1"], new);
    assert!(state.adjust["DP-1"].span.is_some());
    assert_eq!(state.current["DP-2"], old);
    assert_eq!(state.adjust["DP-2"], adjust);
    assert_eq!(state.history.get("DP-2"), before.history.get("DP-2"));
    assert!(state.span.is_none());
}

#[test]
fn failed_span_preflight_changes_neither_preferences_nor_assignments() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let old = picture(dir.path(), "old.png");
    store.set_current("DP-1", &old).unwrap();
    store.save_settings(&Default::default()).unwrap();
    let settings = std::fs::read(store.dirs().config.join("config.json")).unwrap();
    let state = std::fs::read(store.dirs().state.join("state.json")).unwrap();
    let bad = dir.path().join("bad.png");
    std::fs::write(&bad, "broken").unwrap();
    let desktop = Desktop::default();
    assert!(wallpapers::span_and_remember(&desktop, &store, &bad, Default::default(), Default::default()).is_err());
    assert_eq!(
        std::fs::read(store.dirs().config.join("config.json")).unwrap(),
        settings
    );
    assert_eq!(std::fs::read(store.dirs().state.join("state.json")).unwrap(), state);
    assert!(desktop.batches.borrow().is_empty());
}

#[test]
fn region_wins_over_frame_and_resize_keeps_portable_geometry() {
    let adjust = Adjust {
        span: Some(Cut {
            x: 8,
            y: 0,
            width: 8,
            height: 8,
            out_width: 8,
            out_height: 4,
        }),
        frame: Some(Frame {
            zoom: 200,
            x: 500,
            y: 500,
            width: 8,
            height: 4,
        }),
        flip: Flip { h: true, v: true },
        ..Default::default()
    };
    let geometry = Geometry::from_adjust(&adjust, None);
    let resized = geometry.adjust((16, 8));
    assert!(resized.frame.is_none());
    assert_eq!(resized.span.unwrap().out_width, 16);
    assert_eq!(resized.span.unwrap().x, 8);
    assert_eq!(resized.flip, adjust.flip);
    let framed = Geometry::from_adjust(&Adjust { span: None, ..adjust }, None).adjust((4, 8));
    assert_eq!(framed.frame.unwrap().width, 4);
    assert_eq!(framed.frame.unwrap().zoom, 200);
}

#[test]
fn restored_outputs_share_identity_and_preserve_original_paths() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(Dirs::under(dir.path()));
    let path = picture(dir.path(), "image.png");
    for monitor in ["DP-1", "DP-2"] {
        store.set_current(monitor, &path).unwrap();
    }
    let first = wallpapers::saved_for_application(&store, "DP-1", "restore-1").unwrap();
    let second = wallpapers::saved_for_application(&store, "DP-2", "restore-1").unwrap();
    assert_eq!(first.application, second.application);
    assert_eq!(first.intent, Intent::Restore);
    assert_eq!(first.picture.source, path);
}

#[test]
fn oversized_targets_are_rejected_before_transform_cache_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let path = picture(dir.path(), "image.png");
    let cache = dir.path().join("cache");
    let adjust = Adjust {
        frame: Some(Frame {
            zoom: 100,
            x: 500,
            y: 500,
            width: u32::MAX,
            height: 8,
        }),
        ..Default::default()
    };
    assert!(Preparation::default().picture(&cache, &path, &adjust, None).is_err());
    assert!(!cache.exists());
}

#[test]
fn static_preflight_reads_headers_and_verifying_decodes_completely() {
    let dir = tempfile::tempdir().unwrap();
    let path = picture(dir.path(), "cut.png");
    // The header survives; the pixel data stops partway.
    let bytes = std::fs::read(&path).unwrap();
    let pixels = bytes.windows(4).position(|w| w == b"IDAT").unwrap() + 6;
    std::fs::write(&path, &bytes[..pixels]).unwrap();
    assert_eq!(Preparation::default().inspect(&path).unwrap(), (16, 8));
    assert!(Preparation::verifying().inspect(&path).is_err());
}
