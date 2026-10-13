//! Private Wayland SHM presentation when the real software-EGL check rejects playback.
use super::*;

#[test]
#[ignore = "requires private KWin and LIBGL_ALWAYS_SOFTWARE=1; use test-playback-headless.py --fallback"]
fn private_wayland_representative_fallback() {
    assert_eq!(std::env::var("WALLPICKER_PRIVATE_COMPOSITOR").as_deref(), Ok("1"));
    assert_eq!(std::env::var("LIBGL_ALWAYS_SOFTWARE").as_deref(), Ok("1"));
    let tmp = tempfile::tempdir().unwrap();
    let generated = tmp.path().join("source.gif");
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, 4, 2, &[255, 0, 0, 0, 255, 0]).unwrap();
        for color in [0, 1] {
            encoder
                .write_frame(&gif::Frame {
                    width: 4,
                    height: 2,
                    delay: 3,
                    buffer: Cow::Owned(vec![color; 8]),
                    ..Default::default()
                })
                .unwrap();
        }
    }
    std::fs::write(&generated, bytes).unwrap();
    let input = std::env::var_os("WALLPICKER_TEST_ANIMATION")
        .map(PathBuf::from)
        .unwrap_or(generated);
    let source = Source::open(&input, Limits::default()).unwrap();
    let source_bytes = u64::from(source.info().width) * u64::from(source.info().height) * 3;
    let options = Options {
        capacity: NonZeroUsize::new(3).unwrap(),
        texture_bytes: 64 * 1024 * 1024,
        fallback_bytes: 128 * 1024 * 1024,
        interval: Duration::from_millis(20),
        configure_timeout: Duration::from_secs(3),
    };
    let preparer = Preparer {
        executable: std::env::var_os("WALLPICKER_TEST_EXECUTABLE").unwrap().into(),
        budget: Budget::new(512 * 1024 * 1024, 3),
        limits: Limits::default(),
        options: crate::decoding::Options {
            address_space_bytes: 128 * 1024 * 1024,
            frame_allowance: 0,
            frame_timeout: Duration::from_secs(5),
        },
    };
    let renderer = Renderer::start_with_playback(Arc::new(|_| None), options).unwrap();
    let status = wait(&renderer, |s| {
        s["screens"].as_array().unwrap().len() == 2
            && s["screens"].as_array().unwrap().iter().all(|s| s["size"].is_array())
    });
    let monitors: Vec<String> = status["screens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["monitor"].as_str().unwrap().into())
        .collect();
    apply(&renderer, &preparer, batch(&source, &monitors));
    let status = renderer.status().unwrap();
    for screen in status["screens"].as_array().unwrap() {
        assert_eq!(screen["playback"]["state"], "unavailable");
        assert!(screen["playback"]["reason"].as_str().unwrap().contains("software GLES"));
        assert_eq!(screen["source"], source.path().to_str().unwrap());
        assert_eq!(screen["playback"]["representative"], true);
    }
    assert_eq!(status["playback_workers"], 0);
    assert_eq!(status["playback_texture_bytes"], 0);
    assert_eq!(status["playback_timer"], false);
    assert_eq!(status["representative_bytes"], source_bytes);
    assert_eq!(preparer.budget.usage(), Usage::default());
    let application = status["screens"][0]["application"].clone();
    let mut adjusted = batch(&source, &monitors[..1]);
    adjusted.intent = Intent::Adjust;
    adjusted.outputs[0].picture.geometry.view = View::Frame {
        zoom: 200,
        x: 750,
        y: 250,
    };
    adjusted.outputs[0].picture.geometry.flip.h = true;
    let geometry = serde_json::to_value(adjusted.outputs[0].picture.geometry).unwrap();
    for (_, reply) in renderer
        .adjust_batch(PreparedAdjustment::new(adjusted, Limits::default()).unwrap())
        .unwrap()
    {
        reply.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    }
    let status = renderer.status().unwrap();
    assert_eq!(status["screens"][0]["application"], application);
    assert_eq!(status["screens"][0]["playback"]["geometry"], geometry);
    assert_eq!(status["representative_bytes"], source_bytes);
    assert_eq!(preparer.budget.usage(), Usage::default());
    configure(format!("output.{}.rotation.left", monitors[0]));
    wait(&renderer, |s| {
        s["screens"][0]["pixels"] == serde_json::json!([540, 960])
    });
    configure(format!("output.{}.rotation.none", monitors[0]));
    wait(&renderer, |s| {
        s["screens"][0]["pixels"] == serde_json::json!([960, 540])
    });

    // Failed members and repeated replacement release obsolete shared RGB storage.
    for _ in 0..3 {
        apply(&renderer, &preparer, batch(&source, &monitors));
    }
    let mut partial = monitors.clone();
    partial.push("missing-test-output".into());
    let replies = renderer
        .show_batch(
            preparer
                .prepare(batch(&source, &partial), renderer.decoder_wake())
                .unwrap(),
        )
        .unwrap();
    for (monitor, reply) in replies {
        assert_eq!(
            reply.recv_timeout(Duration::from_secs(5)).unwrap().is_err(),
            monitor == "missing-test-output"
        );
    }
    assert_eq!(renderer.status().unwrap()["representative_bytes"], source_bytes);
    assert_eq!(preparer.budget.usage(), Usage::default());

    // The retained first frame can redraw after the source disappears. No codec
    // is reopened on resize, and user-owned assets are never modified.
    let copy = tmp.path().join("removed.gif");
    std::fs::copy(&input, &copy).unwrap();
    let copied = Source::open(&copy, Limits::default()).unwrap();
    apply(&renderer, &preparer, batch(&copied, &monitors));
    std::fs::remove_file(&copy).unwrap();
    configure(format!("output.{}.rotation.left", monitors[0]));
    wait(&renderer, |s| {
        s["screens"][0]["pixels"] == serde_json::json!([540, 960])
    });
    configure(format!("output.{}.rotation.none", monitors[0]));
    wait(&renderer, |s| {
        s["screens"][0]["pixels"] == serde_json::json!([960, 540])
    });
    assert_eq!(preparer.budget.usage(), Usage::default());

    let still = tmp.path().join("still.png");
    wallpicker_core::compose::RgbImage::new(2, 2).save(&still).unwrap();
    for monitor in &monitors {
        renderer
            .show(monitor.clone(), (still.clone(), None), None)
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
    }
    assert_eq!(renderer.status().unwrap()["representative_bytes"], 0);
    apply(&renderer, &preparer, batch(&source, &monitors));
    drop(renderer);
    assert_eq!(preparer.budget.usage(), Usage::default());

    // Admission and fitting failures both preserve the previously displayed still.
    for bytes in [1, source_bytes] {
        let saved = still.clone();
        let renderer = Renderer::start_with_playback(
            Arc::new(move |_| Some((saved.clone(), None).into())),
            Options {
                fallback_bytes: bytes,
                ..options
            },
        )
        .unwrap();
        wait(&renderer, |s| {
            s["screens"].as_array().unwrap().len() == 2
                && s["screens"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|s| s["file"] == still.to_str().unwrap())
        });
        let result = renderer.show_batch(
            preparer
                .prepare(batch(&source, &monitors), renderer.decoder_wake())
                .unwrap(),
        );
        if bytes == 1 {
            assert!(result.err().unwrap().contains("still budget"));
        } else {
            for (_, reply) in result.unwrap() {
                assert!(reply.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
            }
        }
        let status = renderer.status().unwrap();
        assert_eq!(status["representative_bytes"], 0);
        assert_eq!(status["playback_workers"], 0);
        assert!(
            status["screens"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["file"] == still.to_str().unwrap())
        );
        assert_eq!(preparer.budget.usage(), Usage::default());
    }
}
