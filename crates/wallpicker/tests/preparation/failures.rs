use super::*;

#[test]
fn every_source_and_geometry_is_inspected_before_any_worker_starts() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let mut invalid = animated(&source, "B");
    invalid.picture.geometry.view = View::Region {
        x: 1,
        y: 0,
        width: 2,
        height: 1,
    };
    let bad = tmp.path().join("broken.png");
    std::fs::write(&bad, b"not pixels").unwrap();
    let bad = Output {
        monitor: "B".into(),
        picture: Picture::legacy(bad, None),
    };
    let preparer = preparer(2);
    let wakes = Arc::new(AtomicUsize::new(0));
    for output in [invalid, bad] {
        let count = wakes.clone();
        let result = preparer.prepare(
            batch(vec![animated(&source, "A"), output]),
            Arc::new(move |_| {
                count.fetch_add(1, Ordering::Relaxed);
            }),
        );
        assert!(result.is_err());
        assert_eq!(preparer.budget.usage(), Usage::default());
        assert_eq!(wakes.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn first_frame_failure_or_partial_admission_releases_all_earlier_workers() {
    let tmp = tempfile::tempdir().unwrap();
    let good = fixtures::gif(&tmp.path().join("good.gif"), 2, 1);
    let bad = corrupt_webp(&tmp.path().join("bad.webp"), 0);
    let request = batch(vec![animated(&good, "A"), animated(&bad, "B")]);
    for workers in [1, 2] {
        let preparer = preparer(workers);
        let error = preparer.prepare(request.clone(), quiet()).err().unwrap();
        if workers == 1 {
            assert!(error.contains("admission"), "{error}");
        } else {
            assert!(error.contains("decoder"), "{error}");
        }
        assert_eq!(preparer.budget.usage(), Usage::default());
        // The failed batch did not leave a reserved/zombie worker behind.
        drop(preparer.prepare(batch(vec![animated(&good, "A")]), quiet()).unwrap());
        assert_eq!(preparer.budget.usage(), Usage::default());
    }
}

#[test]
fn changed_identity_and_mislabeled_content_cannot_bypass_preflight() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("source.gif");
    let source = fixtures::gif(&path, 2, 1);
    let request = batch(vec![animated(&source, "A")]);
    let preparer = preparer(1);
    let lying = batch(vec![Output {
        monitor: "A".into(),
        picture: Picture::legacy(path.clone(), None),
    }]);
    assert!(preparer.prepare(lying.clone(), quiet()).is_err());
    assert!(PreparedBatch::without_playback(lying).is_err());
    let unchanged = PreparedBatch::without_playback(request.clone()).unwrap();
    assert!(unchanged.sessions().is_empty() && unchanged.session_for("A").is_none());
    fixtures::gif(&path, 2, 1);
    assert!(PreparedBatch::without_playback(request.clone()).err().unwrap().contains("changed"));
    assert!(preparer.prepare(request, quiet()).err().unwrap().contains("changed"));
    let still = tmp.path().join("still.png");
    wallpicker_core::compose::RgbImage::new(2, 1).save(&still).unwrap();
    let source = Source::open(&still, Limits::default()).unwrap();
    assert!(
        preparer
            .prepare(batch(vec![animated(&source, "A")]), quiet())
            .err()
            .unwrap()
            .contains("static")
    );
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn changed_static_prepared_file_is_rejected_at_handoff() {
    let tmp = tempfile::tempdir().unwrap();
    let original = tmp.path().join("original.png");
    let prepared_path = tmp.path().join("prepared.png");
    wallpicker_core::compose::RgbImage::new(2, 2).save(&original).unwrap();
    std::fs::copy(&original, &prepared_path).unwrap();
    let mut picture = Picture::legacy(prepared_path.clone(), None);
    picture.source = original;
    let prepared = PreparedBatch::without_playback(batch(vec![Output {
        monitor: "A".into(),
        picture,
    }]))
    .unwrap();
    std::fs::write(&prepared_path, b"replaced").unwrap();
    assert!(prepared.handoff().is_err());
}
