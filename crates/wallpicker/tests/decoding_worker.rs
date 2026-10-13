//! Real exec/IPC/resource limits. No compositor, daemon socket or user state.
#[path = "decoding/fixtures.rs"]
mod fixtures;
#[path = "decoding/soak.rs"]
mod soak;

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use wallpicker::decoding::{Budget, DecodedFrame, Options, Worker};
use wallpicker_core::media::Source;

const MIB: u64 = 1024 * 1024;
fn options() -> Options {
    Options {
        address_space_bytes: 128 * MIB,
        frame_allowance: 0,
        frame_timeout: Duration::from_secs(5),
    }
}
fn start(source: Source, budget: &Budget, generation: u64) -> Worker {
    Worker::start(
        Path::new(env!("CARGO_BIN_EXE_wallpicker")),
        source,
        generation,
        budget,
        options(),
        Arc::new(|| {}),
    )
    .unwrap()
}
fn next(worker: &Worker) -> DecodedFrame {
    worker
        .recv_timeout(Duration::from_secs(10))
        .expect("worker response")
        .expect("decoded frame")
}

#[test]
fn gif_and_webp_repeat_with_original_timing_and_tagged_results() {
    let tmp = tempfile::tempdir().unwrap();
    for source in [
        fixtures::gif(&tmp.path().join("wrong.jpg"), 2, 1),
        fixtures::webp(&tmp.path().join("source.webp"), 2, 1),
    ] {
        let budget = Budget::new(256 * MIB, 1);
        let worker = start(source, &budget, 42);
        for sequence in 0..12 {
            let frame = next(&worker);
            assert_eq!((frame.generation, frame.sequence), (42, sequence));
            let (color, millis) = if sequence % 2 == 0 {
                ([255, 0, 0, 255], 0)
            } else {
                ([0, 255, 0, 255], 70)
            };
            assert_eq!(frame.frame().pixels.get_pixel(0, 0).0, color);
            assert_eq!(frame.frame().delay, Some(Duration::from_millis(millis)));
            assert_eq!(
                frame.frame().playback_delay(),
                Some(Duration::from_millis(millis.max(10)))
            );
        }
        let pid = worker.process_id().unwrap();
        drop(worker);
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "child must be reaped before release"
        );
        assert_eq!(budget.usage().bytes, 0);
    }
}

#[test]
fn held_frames_bound_memory_and_keep_reservations_after_cancellation() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let budget = Budget::new(256 * MIB, 1);
    let mut worker = start(source.clone(), &budget, 1);
    let frames: Vec<_> = (0..3).map(|_| next(&worker)).collect();
    assert!(
        worker.recv_timeout(Duration::from_millis(50)).is_err(),
        "all slots are held by the caller"
    );
    worker.cancel();
    assert_eq!(budget.usage().workers, 1);
    assert!(
        Worker::start(
            Path::new(env!("CARGO_BIN_EXE_wallpicker")),
            source.clone(),
            2,
            &budget,
            options(),
            Arc::new(|| {})
        )
        .is_err()
    );
    drop(frames);
    assert_eq!(budget.usage().workers, 0);
    drop(start(source, &budget, 2));
    assert_eq!(budget.usage().workers, 0);
}

#[test]
fn pause_blocks_new_decodes_and_cancel_interrupts_queue_backpressure() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let budget = Budget::new(256 * MIB, 1);
    let mut worker = start(source, &budget, 1);
    let frames: Vec<_> = (0..3).map(|_| next(&worker)).collect();
    worker.pause(true);
    drop(frames);
    assert!(worker.recv_timeout(Duration::from_millis(50)).is_err());
    worker.pause(false);
    assert_eq!(next(&worker).sequence, 3);
    std::thread::sleep(Duration::from_millis(50)); // fill the two-entry queue
    let now = Instant::now();
    worker.cancel();
    assert!(now.elapsed() < Duration::from_secs(2));
    assert_eq!(budget.usage().bytes, 0);
}

#[test]
fn changed_source_fails_without_invalidating_already_owned_frames() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("source.gif");
    let source = fixtures::gif(&path, 2, 1);
    let budget = Budget::new(256 * MIB, 1);
    let worker = start(source.clone(), &budget, 1);
    let mut frames: Vec<_> = (0..3).map(|_| next(&worker)).collect();
    std::fs::write(&path, b"changed").unwrap();
    frames.pop();
    let error = worker.recv_timeout(Duration::from_secs(5)).unwrap().err().unwrap();
    assert!(error.contains("changed"), "{error}");
    assert_eq!(frames[0].frame().pixels.get_pixel(0, 0).0, [255, 0, 0, 255]);
    drop(worker);
    drop(frames);
    let worker = start(source, &budget, 2);
    assert!(worker.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
}

#[test]
fn admission_precedes_spawn_and_limit_failures_stay_in_child() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let budget = Budget::new(MIB, 1);
    assert!(
        Worker::start(
            Path::new("/does-not-exist"),
            source.clone(),
            1,
            &budget,
            options(),
            Arc::new(|| {})
        )
        .err()
        .unwrap()
        .to_string()
        .contains("admission")
    );
    assert_eq!(budget.usage().bytes, 0);
    let budget = Budget::new(256 * MIB, 1);
    let worker = Worker::start(
        Path::new(env!("CARGO_BIN_EXE_wallpicker")),
        source.clone(),
        1,
        &budget,
        Options {
            address_space_bytes: MIB,
            frame_allowance: 0,
            ..options()
        },
        Arc::new(|| {}),
    )
    .unwrap();
    assert!(worker.recv_timeout(Duration::from_secs(10)).unwrap().is_err());
    drop(worker);
    assert_eq!(budget.usage().bytes, 0);
    let worker = start(source, &budget, 2);
    drop(next(&worker));
    let limits = std::fs::read_to_string(format!("/proc/{}/limits", worker.process_id().unwrap())).unwrap();
    let memory = limits
        .lines()
        .find(|line| line.starts_with("Max address space"))
        .unwrap();
    assert_eq!(
        memory.split_whitespace().filter(|v| *v == "134217728").count(),
        2,
        "{memory}"
    );
    let core = limits
        .lines()
        .find(|line| line.starts_with("Max core file size"))
        .unwrap();
    assert_eq!(core.split_whitespace().filter(|v| *v == "0").count(), 2, "{core}");
}

#[test]
fn rapid_replacement_leaves_no_children_or_reserved_bytes() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let budget = Budget::new(256 * MIB, 1);
    for generation in 0..20 {
        let worker = start(source.clone(), &budget, generation);
        if generation % 2 == 0 {
            assert_eq!(next(&worker).generation, generation);
        }
        drop(worker);
        assert_eq!(budget.usage().bytes, 0);
    }
}

#[test]
fn stopped_decoder_times_out_or_can_be_cancelled_during_a_read() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let budget = Budget::new(256 * MIB, 1);
    for cancel in [false, true] {
        let mut worker = Worker::start(
            Path::new(env!("CARGO_BIN_EXE_wallpicker")),
            source.clone(),
            1,
            &budget,
            Options {
                frame_timeout: Duration::from_secs(1),
                ..options()
            },
            Arc::new(|| {}),
        )
        .unwrap();
        let mut frames: Vec<_> = (0..3).map(|_| next(&worker)).collect();
        let pid = worker.process_id().unwrap();
        // SAFETY: signal a live child owned by this worker; no memory access.
        assert_eq!(unsafe { libc::kill(pid as libc::pid_t, libc::SIGSTOP) }, 0);
        frames.pop();
        if cancel {
            std::thread::sleep(Duration::from_millis(30));
            let now = Instant::now();
            worker.cancel();
            assert!(now.elapsed() < Duration::from_secs(1));
        } else {
            assert!(worker.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
        }
        drop(worker);
        assert!(!Path::new(&format!("/proc/{pid}")).exists());
        drop(frames);
        assert_eq!(budget.usage().bytes, 0);
    }
}

#[test]
fn later_corrupt_payload_preserves_the_first_frame_and_reports_once() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("corrupt.webp");
    fixtures::webp(&path, 2, 1);
    let mut bytes = std::fs::read(&path).unwrap();
    let second = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, b)| *b == b"VP8L")
        .nth(1)
        .unwrap()
        .0;
    let size = u32::from_le_bytes(bytes[second + 4..second + 8].try_into().unwrap()) as usize;
    bytes[second + 13..second + 8 + size].fill(0xff);
    std::fs::write(&path, bytes).unwrap();
    let source = Source::open(&path, Default::default()).unwrap();
    let budget = Budget::new(256 * MIB, 1);
    let worker = start(source, &budget, 1);
    let first = next(&worker);
    assert!(worker.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
    assert_eq!(
        worker.recv_timeout(Duration::from_secs(5)).err(),
        Some(std::sync::mpsc::RecvTimeoutError::Disconnected)
    );
    assert_eq!(first.frame().pixels.get_pixel(0, 0).0, [255, 0, 0, 255]);
    drop(worker);
    drop(first);
    assert_eq!(budget.usage().bytes, 0);
}
