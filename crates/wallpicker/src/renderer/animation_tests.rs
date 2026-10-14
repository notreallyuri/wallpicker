//! Actual Wayland + hardware EGL, on an explicitly isolated compositor only.
#[path = "fallback_tests.rs"]
mod fallback_tests;
use super::{Renderer, animation::Options};
use crate::decoding::{Budget, Usage};
use crate::preparation::{PreparedAdjustment, Preparer};
use serde_json::Value;
use std::{
    borrow::Cow,
    num::NonZeroUsize,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use wallpicker_core::{
    media::{Limits, Source},
    render::{Batch, Content, Geometry, Intent, Output, Picture, View},
};

fn wait(renderer: &Renderer, condition: impl Fn(&Value) -> bool) -> Value {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let status = renderer.status().unwrap();
        if condition(&status) {
            return status;
        }
        assert!(Instant::now() < until, "renderer timed out: {status}");
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn batch(source: &Source, monitors: &[String]) -> Batch {
    let outputs = monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| Output {
            monitor: monitor.clone(),
            picture: Picture {
                source: source.path().into(),
                paused: false,
                geometry: if index == 1 {
                    Geometry {
                        view: View::Region {
                            x: 0,
                            y: 0,
                            width: source.info().width / 2,
                            height: source.info().height,
                        },
                        ..Default::default()
                    }
                } else {
                    Geometry::default()
                },
                content: Content::Animated {
                    identity: source.identity().clone(),
                },
            },
        })
        .collect();
    Batch::new(
        Intent::Apply,
        wallpicker_core::settings::Transition {
            kind: "none".into(),
            duration_ms: 0,
        },
        outputs,
    )
}
fn apply(renderer: &Renderer, preparer: &Preparer, batch: Batch) {
    let prepared = preparer.prepare(batch, renderer.decoder_wake()).unwrap();
    for (_, reply) in renderer.show_batch(prepared).unwrap() {
        reply.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    }
}
fn configure(argument: String) {
    let result = std::process::Command::new("timeout")
        .args(["10s", "kscreen-doctor", &argument])
        .env("QT_QPA_PLATFORM", "wayland")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "private output configuration failed: {} {}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
#[ignore = "requires a private headless Wayland compositor and hardware EGL; see test-playback-headless.py"]
fn private_wayland_animated_lifecycle() {
    assert_eq!(
        std::env::var("WALLPICKER_PRIVATE_COMPOSITOR").as_deref(),
        Ok("1"),
        "use the isolated compositor script"
    );
    let tmp = tempfile::tempdir().unwrap();
    let generated = tmp.path().join("source.gif");
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, 7, 5, &[255, 0, 0, 0, 255, 0]).unwrap();
        encoder.set_repeat(gif::Repeat::Finite(1)).unwrap();
        for color in [0, 1] {
            encoder
                .write_frame(&gif::Frame {
                    width: 7,
                    height: 5,
                    delay: 3,
                    buffer: Cow::Owned(vec![color; 35]),
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
    let options = Options {
        capacity: NonZeroUsize::new(3).unwrap(),
        texture_bytes: 64 * 1024 * 1024,
        fallback_bytes: 128 * 1024 * 1024,
        interval: Duration::from_millis(20),
        configure_timeout: Duration::from_secs(3),
    };
    let renderer = Renderer::start_with_playback(Arc::new(|_| None), options).unwrap();
    let preparer = Preparer {
        executable: std::env::var_os("WALLPICKER_TEST_EXECUTABLE")
            .expect("worker executable")
            .into(),
        budget: Budget::new(512 * 1024 * 1024, 3),
        options: crate::decoding::Options {
            address_space_bytes: 128 * 1024 * 1024,
            frame_allowance: 0,
            frame_timeout: Duration::from_secs(5),
        },
        limits: Limits::default(),
    };
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
    let status = wait(&renderer, |s| {
        s["screens"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["playback"]["presentations"].as_u64().unwrap_or(0) >= 4)
    });
    assert_eq!(status["playback_workers"], 1);
    assert_eq!(
        status["playback_texture_bytes"],
        u64::from(source.info().width) * u64::from(source.info().height) * 4
    );
    let screens = status["screens"].as_array().unwrap();
    assert_eq!(
        screens[0]["playback"]["generation"],
        screens[1]["playback"]["generation"]
    );
    assert_eq!(screens[0]["playback"]["uploads"], screens[1]["playback"]["uploads"]);
    let old_generation = screens[1]["playback"]["generation"].clone();
    let presentations: u64 = screens
        .iter()
        .map(|s| s["playback"]["presentations"].as_u64().unwrap())
        .sum();
    assert!(
        screens[0]["playback"]["uploads"].as_u64().unwrap() < presentations,
        "source upload was not shared"
    );
    eprintln!("shared animation: {status}");

    // Adjustments retain the shared session, application and texture reservation.
    // The transition setting is deliberately non-instant: Adjust ignores it.
    let application = screens[0]["application"].clone();
    for (zoom, x, y) in [(150, 0, 1000), (300, 800, 200), (100, 500, 500)] {
        let before = renderer.status().unwrap();
        let mut request = batch(&source, &monitors[..1]);
        request.intent = Intent::Adjust;
        request.transition = Default::default();
        request.outputs[0].picture.geometry.view = View::Frame { zoom, x, y };
        request.outputs[0].picture.geometry.flip.h = true;
        request.outputs[0].picture.geometry.fit = Some(wallpicker_core::settings::FitMode::Contain);
        let geometry = serde_json::to_value(request.outputs[0].picture.geometry).unwrap();
        let prepared = PreparedAdjustment::new(request, Limits::default()).unwrap();
        for (_, reply) in renderer.adjust_batch(prepared).unwrap() {
            reply.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
        }
        let adjusted = renderer.status().unwrap();
        assert_eq!(adjusted["playback_workers"], 1);
        assert_eq!(adjusted["playback_texture_bytes"], before["playback_texture_bytes"]);
        assert_eq!(adjusted["screens"][0]["application"], application);
        assert_eq!(adjusted["screens"][0]["intent"], "adjust");
        assert_eq!(adjusted["screens"][0]["playback"]["geometry"], geometry);
        for i in 0..2 {
            assert_eq!(adjusted["screens"][i]["playback"]["generation"], old_generation);
            assert!(
                adjusted["screens"][i]["playback"]["sequence"].as_u64().unwrap()
                    >= before["screens"][i]["playback"]["sequence"].as_u64().unwrap()
            );
        }
        assert_eq!(
            adjusted["screens"][1]["playback"]["output_generation"],
            before["screens"][1]["playback"]["output_generation"]
        );
    }
    // An otherwise valid request for a different identity cannot replace a live
    // source under Adjust intent. Other members still receive their own result.
    let different_path = tmp.path().join("different.gif");
    std::fs::copy(&input, &different_path).unwrap();
    let different = Source::open(&different_path, Limits::default()).unwrap();
    let mut request = batch(&source, &monitors);
    request.intent = Intent::Adjust;
    request.outputs[1].picture = batch(&different, &monitors[1..]).outputs.remove(0).picture;
    let before = renderer.status().unwrap();
    for (monitor, reply) in renderer
        .adjust_batch(PreparedAdjustment::new(request, Limits::default()).unwrap())
        .unwrap()
    {
        let result = reply.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(result.is_err(), monitor == monitors[1]);
    }
    let adjusted = renderer.status().unwrap();
    assert_eq!(
        adjusted["screens"][1]["playback"]["output_generation"],
        before["screens"][1]["playback"]["output_generation"]
    );
    assert_eq!(adjusted["playback_workers"], 1);

    // Real output rotation resizes the EGL window without resetting its source.
    let output_generation = screens[0]["playback"]["output_generation"].clone();
    configure(format!("output.{}.rotation.left", monitors[0]));
    let status = wait(&renderer, |s| {
        s["screens"][0]["pixels"] == serde_json::json!([540, 960])
            && s["screens"][0]["playback"]["output_generation"] != output_generation
    });
    assert_eq!(status["screens"][0]["playback"]["generation"], old_generation);
    configure(format!("output.{}.scale.2", monitors[0]));
    wait(&renderer, |s| s["screens"][0]["size"] == serde_json::json!([270, 480]));
    configure(format!("output.{}.rotation.none", monitors[0]));
    configure(format!("output.{}.scale.1", monitors[0]));
    wait(&renderer, |s| {
        s["screens"][0]["pixels"] == serde_json::json!([960, 540])
            && s["screens"][0]["size"] == serde_json::json!([960, 540])
    });

    // Reapply starts a new source clock only for the selected output.
    apply(&renderer, &preparer, batch(&source, &monitors[..1]));
    let status = wait(&renderer, |s| {
        s["screens"][0]["playback"]["presentations"].as_u64().unwrap_or(0) >= 2
    });
    assert_ne!(status["screens"][0]["playback"]["generation"], old_generation);
    assert_eq!(status["screens"][1]["playback"]["generation"], old_generation);
    assert_eq!(status["playback_workers"], 2);

    // Static replacement retires just its own source and preserves the other.
    let still = tmp.path().join("still.png");
    wallpicker_core::compose::RgbImage::from_raw(2, 2, vec![20; 12])
        .unwrap()
        .save(&still)
        .unwrap();
    renderer
        .show(monitors[0].clone(), (still.clone(), None), None)
        .unwrap()
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    wait(&renderer, |s| {
        s["playback_workers"] == 1 && s["screens"][0]["playback"].is_null()
    });

    // A solo source also keeps advancing through repeated adjustment handoffs.
    let before = renderer.status().unwrap();
    let mut request = batch(&source, &monitors[1..]);
    request.intent = Intent::Adjust;
    request.outputs[0].picture.geometry.flip.v = true;
    for _ in 0..3 {
        for (_, reply) in renderer
            .adjust_batch(PreparedAdjustment::new(request.clone(), Limits::default()).unwrap())
            .unwrap()
        {
            reply.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
        }
    }
    let adjusted = wait(&renderer, |s| {
        s["screens"][1]["playback"]["sequence"].as_u64().unwrap()
            > before["screens"][1]["playback"]["sequence"].as_u64().unwrap()
    });
    assert_eq!(adjusted["screens"][1]["playback"]["generation"], old_generation);
    assert_eq!(
        adjusted["screens"][1]["application"],
        before["screens"][1]["application"]
    );
    assert_eq!(adjusted["playback_workers"], 1);

    // Replacement while old callbacks remain queued must not revive old targets.
    for _ in 0..3 {
        apply(&renderer, &preparer, batch(&source, &monitors));
        wait(&renderer, |s| s["playback_workers"] == 1);
    }
    // Failed members leave the shared first-frame barrier immediately.
    let mut partial = monitors.clone();
    partial.push("missing-test-output".into());
    let prepared = preparer
        .prepare(batch(&source, &partial), renderer.decoder_wake())
        .unwrap();
    let replies = renderer.show_batch(prepared).unwrap();
    for (monitor, reply) in replies {
        let result = reply.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(result.is_err(), monitor == "missing-test-output");
    }
    wait(&renderer, |s| {
        s["playback_workers"] == 1
            && s["screens"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["playback"]["state"] == "playing")
    });
    // Removing and re-adding a virtual output tears down its EGL surface first.
    configure(format!("output.{}.disable", monitors[0]));
    wait(&renderer, |s| {
        s["screens"].as_array().unwrap().len() == 1 && s["playback_workers"] == 1
    });
    configure(format!("output.{}.enable", monitors[0]));
    wait(&renderer, |s| {
        s["screens"].as_array().unwrap().len() == 2
            && s["screens"].as_array().unwrap().iter().all(|s| s["size"].is_array())
    });
    apply(&renderer, &preparer, batch(&source, &monitors));
    wait(&renderer, |s| s["playback_workers"] == 1);
    // Modifying a private copy after acceptance must freeze the last displayed
    // frame and report the decoder failure, without changing the user's input.
    let changing = tmp.path().join("changing.gif");
    std::fs::copy(&input, &changing).unwrap();
    let changing_source = Source::open(&changing, Limits::default()).unwrap();
    apply(&renderer, &preparer, batch(&changing_source, &monitors));
    std::fs::write(&changing, b"changed after initial presentation").unwrap();
    let frozen = wait(&renderer, |s| {
        s["screens"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["playback"]["state"] == "frozen")
    });
    assert!(
        frozen["screens"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["playback"]["error"].is_string())
    );
    assert_eq!(frozen["playback_timer"], false);
    let uploads = frozen["screens"][0]["playback"]["uploads"].clone();
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(renderer.status().unwrap()["screens"][0]["playback"]["uploads"], uploads);
    for monitor in &monitors {
        renderer
            .show(monitor.clone(), (still.clone(), None), None)
            .unwrap()
            .recv_timeout(Duration::from_secs(5))
            .unwrap()
            .unwrap();
    }
    let status = wait(&renderer, |s| {
        s["playback_workers"] == 0 && s["playback_texture_bytes"] == 0 && s["playback_timer"] == false
    });
    assert!(
        status["screens"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["playback"].is_null())
    );
    assert_eq!(preparer.budget.usage(), Usage::default());
    transition_checks(&renderer, &preparer, &source, &monitors);
    // Shutdown with a running shared source joins cleanup and destroys EGL first.
    apply(&renderer, &preparer, batch(&source, &monitors));
    drop(renderer);
    assert_eq!(preparer.budget.usage(), Usage::default());

    // GPU admission failure must leave the existing still and no leased workers.
    let saved = still.clone();
    let renderer = Renderer::start_with_playback(
        Arc::new(move |_| Some((saved.clone(), None).into())),
        Options {
            texture_bytes: 1,
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
    let prepared = preparer
        .prepare(batch(&source, &monitors), renderer.decoder_wake())
        .unwrap();
    assert!(renderer.show_batch(prepared).err().unwrap().contains("texture budget"));
    assert_eq!(preparer.budget.usage(), Usage::default());
    let status = renderer.status().unwrap();
    assert_eq!(status["playback_workers"], 0);
    assert_eq!(status["playback_texture_bytes"], 0);
    assert!(
        status["screens"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["file"] == still.to_str().unwrap())
    );
}

fn transition_checks(renderer: &Renderer, preparer: &Preparer, source: &Source, monitors: &[String]) {
    // Start over the still left by the lifecycle test; subsequent iterations
    // exercise animation -> animation. All incoming clocks remain at frame zero
    // until their frozen endpoints have completed on every member.
    for kind in [
        "fade", "wipe", "left", "right", "top", "bottom", "grow", "outer", "wave",
    ] {
        let before = renderer.status().unwrap();
        let mut request = batch(source, monitors);
        request.transition = wallpicker_core::settings::Transition {
            kind: kind.into(),
            duration_ms: 250,
        };
        let replies = renderer
            .show_batch(preparer.prepare(request, renderer.decoder_wake()).unwrap())
            .unwrap();
        let during = wait(renderer, |s| {
            s["screens"]
                .as_array()
                .unwrap()
                .iter()
                .all(|s| s["animation_transition"]["presentations"].as_u64().unwrap_or(0) >= 2)
        });
        for (i, screen) in during["screens"].as_array().unwrap().iter().enumerate() {
            assert_eq!(screen["animation_transition"]["sequence"], 0, "{kind}");
            assert_eq!(screen["animation_transition"]["playing"], false, "{kind}");
            assert_eq!(screen["animation_transition"]["uploads"], 1, "{kind}");
            if before["screens"][i]["playback"].is_object() {
                assert_eq!(
                    screen["playback"]["generation"],
                    before["screens"][i]["playback"]["generation"]
                );
            }
        }
        let frozen = during["screens"][0]["playback"]["presentations"].clone();
        std::thread::sleep(Duration::from_millis(30));
        let mid = renderer.status().unwrap();
        assert_eq!(
            mid["screens"][0]["playback"]["presentations"], frozen,
            "outgoing target must stay frozen: {kind}"
        );
        for (_, reply) in &replies {
            assert!(
                matches!(reply.try_recv(), Err(std::sync::mpsc::TryRecvError::Empty)),
                "{kind} acknowledged before endpoint"
            );
        }
        if kind == "fade" {
            // Resize an incoming transition over a static wallpaper while its
            // old static resize worker may still be producing a result.
            configure(format!("output.{}.rotation.left", monitors[0]));
        }
        for (_, reply) in replies {
            reply.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
        }
        let after = wait(renderer, |s| {
            s["playback_workers"] == 1
                && s["screens"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|s| s["playback"]["sequence"].as_u64().unwrap_or(0) >= 1)
        });
        for screen in after["screens"].as_array().unwrap() {
            assert!(screen["animation_transition"].is_null());
            assert_eq!(
                screen["playback"]["generation"],
                during["screens"][0]["animation_transition"]["generation"]
            );
            assert_eq!(screen["fading"], false);
        }
        if kind == "fade" {
            let rotated = after["screens"]
                .as_array()
                .unwrap()
                .iter()
                .find(|s| s["monitor"] == monitors[0])
                .unwrap();
            assert_eq!(rotated["pixels"], serde_json::json!([540, 960]), "{after}");
            configure(format!("output.{}.rotation.none", monitors[0]));
            wait(renderer, |s| {
                s["screens"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|s| s["pixels"] == serde_json::json!([960, 540]))
            });
        }
    }
    // Supersession while callbacks/deadlines are outstanding cannot promote the
    // abandoned endpoint or leave a shared source paused/retained indefinitely.
    let mut request = batch(source, monitors);
    request.transition = wallpicker_core::settings::Transition {
        kind: "left".into(),
        duration_ms: 1500,
    };
    let replies = renderer
        .show_batch(preparer.prepare(request, renderer.decoder_wake()).unwrap())
        .unwrap();
    let abandoned = wait(renderer, |s| {
        s["screens"][0]["animation_transition"]["presentations"]
            .as_u64()
            .unwrap_or(0)
            >= 2
    });
    let generation = abandoned["screens"][0]["animation_transition"]["generation"].clone();
    apply(renderer, preparer, batch(source, monitors));
    for (_, reply) in replies {
        assert!(reply.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
    }
    let status = wait(renderer, |s| s["playback_workers"] == 1);
    for screen in status["screens"].as_array().unwrap() {
        assert_ne!(screen["playback"]["generation"], generation);
        assert!(screen["animation_transition"].is_null());
    }

    // A disappearing member must leave the endpoint barrier and return failure,
    // allowing the surviving member to begin playback after its own endpoint.
    let mut request = batch(source, monitors);
    request.transition = wallpicker_core::settings::Transition {
        kind: "wave".into(),
        duration_ms: 600,
    };
    let replies = renderer
        .show_batch(preparer.prepare(request, renderer.decoder_wake()).unwrap())
        .unwrap();
    wait(renderer, |s| {
        s["screens"][0]["animation_transition"]["presentations"]
            .as_u64()
            .unwrap_or(0)
            >= 2
    });
    configure(format!("output.{}.disable", monitors[0]));
    for (monitor, reply) in replies {
        assert_eq!(
            reply.recv_timeout(Duration::from_secs(5)).unwrap().is_err(),
            monitor == monitors[0]
        );
    }
    wait(renderer, |s| {
        s["playback_workers"] == 1
            && s["screens"].as_array().unwrap().len() == 1
            && s["screens"][0]["playback"]["sequence"].as_u64().unwrap_or(0) >= 1
    });
    configure(format!("output.{}.enable", monitors[0]));
    wait(renderer, |s| {
        s["screens"].as_array().unwrap().len() == 2
            && s["screens"].as_array().unwrap().iter().all(|s| s["size"].is_array())
    });
}
