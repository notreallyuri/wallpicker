//! Owned first-frame preflight, with real isolated decoders and no desktop.
#[path = "preparation/failures.rs"]
mod failures;
#[path = "decoding/fixtures.rs"]
mod fixtures;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};
use wallpicker::decoding::{Budget, Options, Usage};
use wallpicker::preparation::{PreparedAdjustment, PreparedBatch, Preparer};
use wallpicker_core::{
    media::{Limits, Source},
    render::{Batch, Content, Geometry, Intent, Output, Picture, View},
};

const MIB: u64 = 1024 * 1024;
fn preparer(workers: usize) -> Preparer {
    Preparer {
        executable: env!("CARGO_BIN_EXE_wallpicker").into(),
        budget: Budget::new(512 * MIB, workers),
        options: Options {
            address_space_bytes: 128 * MIB,
            frame_allowance: 0,
            frame_timeout: Duration::from_secs(5),
        },
        limits: Limits::default(),
    }
}
fn animated(source: &Source, monitor: &str) -> Output {
    Output {
        monitor: monitor.into(),
        picture: Picture {
            source: source.path().into(),
            geometry: Geometry::default(),
            content: Content::Animated {
                identity: source.identity().clone(),
            },
        },
    }
}
fn batch(outputs: Vec<Output>) -> Batch {
    Batch::new(Intent::Apply, Default::default(), outputs)
}
fn quiet() -> Arc<dyn Fn(u64) + Send + Sync> {
    Arc::new(|_| {})
}
fn corrupt_webp(path: &std::path::Path, frame: usize) -> Source {
    fixtures::webp(path, 2, 1);
    let mut bytes = std::fs::read(path).unwrap();
    let start = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, b)| *b == b"VP8L")
        .nth(frame)
        .unwrap()
        .0;
    let len = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()) as usize;
    bytes[start + 13..start + 8 + len].fill(0xff);
    std::fs::write(path, bytes).unwrap();
    Source::open(path, Limits::default()).unwrap()
}

#[test]
fn shared_source_has_one_paused_worker_and_keeps_per_output_geometry() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 4, 1);
    let mut right = animated(&source, "DP-2");
    right.picture.geometry.view = View::Region {
        x: 2,
        y: 0,
        width: 2,
        height: 1,
    };
    let request = batch(vec![animated(&source, "DP-1"), right]);
    let application = request.application.clone();
    let preparer = preparer(1);
    let prepared = preparer.prepare(request, quiet()).unwrap();
    assert_eq!(prepared.sessions().len(), 1);
    assert_eq!(preparer.budget.usage().workers, 1);
    assert!(std::ptr::eq(
        prepared.session_for("DP-1").unwrap(),
        prepared.session_for("DP-2").unwrap()
    ));
    assert_eq!(prepared.sessions()[0].key().application, application);
    assert_eq!(prepared.sessions()[0].first().sequence, 0);
    assert_eq!(prepared.sessions()[0].first().frame().pixels.dimensions(), (4, 1));

    // Move across a channel just as renderer ownership transfer will do.
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(prepared).unwrap();
    let handoff = rx.recv().unwrap().handoff().unwrap();
    assert_eq!(handoff.groups, vec![Some(0), Some(0)]);
    assert!(matches!(
        handoff.batch.outputs[1].picture.geometry.view,
        View::Region { x: 2, .. }
    ));
    let session = handoff.sessions.into_iter().next().unwrap();
    let (_, worker, first) = session.into_parts();
    assert!(
        worker.recv_timeout(Duration::from_millis(50)).is_err(),
        "preflight must not decode ahead"
    );
    worker.pause(false);
    let second = worker.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    assert_eq!(second.sequence, 1);
    assert_eq!(second.frame().pixels.get_pixel(0, 0).0, [0, 255, 0, 255]);
    drop(second);
    drop(worker);
    assert_eq!(
        preparer.budget.usage().workers,
        1,
        "retained first frame remains reserved"
    );
    drop(first);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn independent_sources_and_new_applications_have_distinct_sessions() {
    let tmp = tempfile::tempdir().unwrap();
    let gif = fixtures::gif(&tmp.path().join("one.gif"), 2, 1);
    let webp = fixtures::webp(&tmp.path().join("two.webp"), 2, 1);
    let preparer = preparer(3);
    let first = preparer
        .prepare(batch(vec![animated(&gif, "A"), animated(&webp, "B")]), quiet())
        .unwrap();
    assert_eq!(first.sessions().len(), 2);
    assert_ne!(first.sessions()[0].generation(), first.sessions()[1].generation());
    let second = preparer.prepare(batch(vec![animated(&gif, "C")]), quiet()).unwrap();
    assert_ne!(first.sessions()[0].key(), second.sessions()[0].key());
    assert_ne!(first.sessions()[0].generation(), second.sessions()[0].generation());
    assert_eq!(preparer.budget.usage().workers, 3);
    drop((first, second));
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn static_and_empty_batches_need_no_worker_or_decoder_budget() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("still.png");
    wallpicker_core::compose::RgbImage::new(2, 2).save(&path).unwrap();
    let mut preparer = preparer(0);
    preparer.executable = "/does-not-exist".into();
    for outputs in [
        vec![],
        vec![Output {
            monitor: "A".into(),
            picture: Picture::legacy(path.clone(), None),
        }],
    ] {
        let prepared = preparer.prepare(batch(outputs), quiet()).unwrap();
        assert!(prepared.sessions().is_empty());
        assert!(prepared.handoff().unwrap().groups.iter().all(Option::is_none));
    }
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn mixed_batch_preserves_static_files_alongside_shared_animation() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("still.png");
    wallpicker_core::compose::RgbImage::new(2, 2).save(&path).unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let preparer = preparer(1);
    let prepared = preparer
        .prepare(
            batch(vec![
                Output {
                    monitor: "static".into(),
                    picture: Picture::legacy(path.clone(), None),
                },
                animated(&source, "A"),
                animated(&source, "B"),
            ]),
            quiet(),
        )
        .unwrap();
    assert!(prepared.session_for("static").is_none());
    let handoff = prepared.handoff().unwrap();
    assert_eq!(handoff.groups, vec![None, Some(0), Some(0)]);
    assert_eq!(handoff.batch.outputs[0].picture.static_file().unwrap().0, path);
    assert_eq!(handoff.sessions.len(), 1);
    drop(handoff);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn hard_link_aliases_share_a_worker_but_each_path_is_checked_at_handoff() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("source.gif");
    fixtures::gif(&path, 2, 1);
    let alias = tmp.path().join("alias.gif");
    std::fs::hard_link(&path, &alias).unwrap();
    let source = Source::open(&path, Limits::default()).unwrap();
    let other = Source::open(&alias, Limits::default()).unwrap();
    let preparer = preparer(1);
    let prepared = preparer
        .prepare(batch(vec![animated(&source, "A"), animated(&other, "B")]), quiet())
        .unwrap();
    assert_eq!(prepared.sessions().len(), 1);
    // Replacing just the alias must not hide behind the primary source's identity.
    let replacement = tmp.path().join("replacement.gif");
    std::fs::copy(&path, &replacement).unwrap();
    std::fs::rename(replacement, alias).unwrap();
    assert!(prepared.handoff().is_err());
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn later_corruption_is_deferred_until_the_owner_starts_playback() {
    let tmp = tempfile::tempdir().unwrap();
    let source = corrupt_webp(&tmp.path().join("late.webp"), 1);
    let preparer = preparer(1);
    let prepared = preparer.prepare(batch(vec![animated(&source, "A")]), quiet()).unwrap();
    let mut handoff = prepared.handoff().unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let session = &mut handoff.sessions[0];
    assert!(session.try_next().is_none());
    assert!(session.process_id().is_some(), "worker must still be alive and paused");
    session.pause(false);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(result) = session.try_next() {
            assert!(result.is_err());
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(session.first().frame().pixels.get_pixel(0, 0).0, [255, 0, 0, 255]);
    drop(handoff);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn adjustment_preflight_works_with_all_decoder_slots_occupied() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 4, 2);
    let preparer = preparer(1);
    let live = preparer.prepare(batch(vec![animated(&source, "A")]), quiet()).unwrap();
    let usage = preparer.budget.usage();
    let mut request = batch(vec![animated(&source, "A"), animated(&source, "B")]);
    request.intent = Intent::Adjust;
    request.outputs[0].picture.geometry.view = View::Frame {
        zoom: 200,
        x: 750,
        y: 250,
    };
    let adjusted = PreparedAdjustment::new(request.clone(), Limits::default()).unwrap();
    assert_eq!(adjusted.handoff().unwrap().outputs, request.outputs);
    assert_eq!(preparer.budget.usage(), usage);
    assert_eq!(live.sessions()[0].first().sequence, 0);
    request.outputs[1].picture.geometry.view = View::Region {
        x: 3,
        y: 0,
        width: 2,
        height: 2,
    };
    assert!(PreparedAdjustment::new(request, Limits::default()).is_err());
    drop(live);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn adjustment_rechecks_identity_and_rejects_apply_intent_and_corrupt_static_members() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 2);
    let mut request = batch(vec![animated(&source, "A")]);
    assert!(PreparedAdjustment::new(request.clone(), Limits::default()).is_err());
    request.intent = Intent::Adjust;
    let prepared = PreparedAdjustment::new(request.clone(), Limits::default()).unwrap();
    let bad = tmp.path().join("bad.png");
    std::fs::write(&bad, b"broken").unwrap();
    request.outputs.push(Output {
        monitor: "B".into(),
        picture: Picture::legacy(bad, None),
    });
    assert!(PreparedAdjustment::new(request, Limits::default()).is_err());
    std::fs::write(source.path(), b"changed since inspection").unwrap();
    assert!(prepared.handoff().is_err());
}
