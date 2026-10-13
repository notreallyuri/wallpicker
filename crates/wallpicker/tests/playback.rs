//! Real admitted workers feeding the headless renderer owner. Source time is
//! injected; only decoder delivery uses real time. No Wayland connection.
#[path = "decoding/fixtures.rs"]
mod fixtures;

use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};
use wallpicker::decoding::{Budget, Options, Usage};
use wallpicker::playback::{Decision, OutputPacer, PauseReason, SourceSession, Target, Ticket};
use wallpicker::preparation::Preparer;
use wallpicker_core::{
    media::{Limits, Source},
    render::{Batch, Content, Geometry, Intent, Output, Picture},
};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn preparer() -> Preparer {
    Preparer {
        executable: env!("CARGO_BIN_EXE_wallpicker").into(),
        budget: Budget::new(512 * 1024 * 1024, 3),
        options: Options {
            address_space_bytes: 128 * 1024 * 1024,
            frame_allowance: 0,
            frame_timeout: Duration::from_secs(5),
        },
        limits: Limits::default(),
    }
}
fn prepare(preparer: &Preparer, source: &Source) -> (SourceSession, mpsc::Receiver<u64>, u32) {
    let outputs = ["A", "B"].map(|monitor| Output {
        monitor: monitor.into(),
        picture: Picture {
            source: source.path().into(),
            geometry: Geometry::default(),
            content: Content::Animated {
                identity: source.identity().clone(),
            },
        },
    });
    let request = Batch::new(Intent::Apply, Default::default(), outputs.into());
    let (tx, rx) = mpsc::channel();
    let prepared = preparer
        .prepare(
            request,
            Arc::new(move |generation| {
                let _ = tx.send(generation);
            }),
        )
        .unwrap();
    let mut handoff = prepared.handoff().unwrap();
    assert_eq!(handoff.groups, [Some(0), Some(0)]);
    assert_eq!(handoff.sessions.len(), 1);
    let session = handoff.sessions.pop().unwrap();
    let pid = session.process_id().unwrap();
    (SourceSession::new(session, ms(0)), rx, pid)
}
fn until(session: &mut SourceSession, rx: &mpsc::Receiver<u64>, now: Duration, done: impl Fn(&SourceSession) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        session.advance(now, session.generation());
        if done(session) {
            return;
        }
        assert!(session.error().is_none(), "{:?}", session.error());
        assert!(Instant::now() < deadline, "decoder did not deliver a frame");
        assert_eq!(
            rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap(),
            session.generation()
        );
    }
}
fn ticket(decision: Decision) -> Ticket {
    match decision {
        Decision::Present(ticket) => ticket,
        other => panic!("expected presentation, got {other:?}"),
    }
}

#[test]
fn gif_and_webp_share_one_source_clock_across_fast_and_stalled_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    for source in [
        fixtures::gif(&tmp.path().join("source.gif"), 2, 1),
        fixtures::webp(&tmp.path().join("source.webp"), 2, 1),
    ] {
        let preparer = preparer();
        let (mut session, rx, _) = prepare(&preparer, &source);
        assert_eq!(session.current().sequence, 0);
        assert_eq!(session.next_deadline(), None);
        let generation = session.generation();
        let mut fast = OutputPacer::new(
            Target {
                session: generation,
                output: 1,
            },
            ms(0),
        );
        let mut slow = OutputPacer::new(
            Target {
                session: generation,
                output: 2,
            },
            ms(0),
        );
        session.set_paused(PauseReason::Transition, false, ms(100));
        assert_eq!(session.next_deadline(), Some(ms(110)));
        let stalled = ticket(slow.plan(ms(100), generation, 0, session.running()));
        assert!(slow.presented(stalled, ms(100)));
        let initial = ticket(fast.plan(ms(100), generation, 0, session.running()));
        assert!(fast.presented(initial, ms(100)));
        assert!(fast.frame_callback(initial));
        let mut now = ms(100);
        for sequence in 1..=40 {
            now = session.next_deadline().unwrap();
            until(&mut session, &rx, now, |s| s.current().sequence == sequence);
            let expected = if sequence % 2 == 0 {
                [255, 0, 0, 255]
            } else {
                [0, 255, 0, 255]
            };
            assert_eq!(session.current().frame().pixels.get_pixel(0, 0).0, expected);
            let drawn = ticket(fast.plan(now, generation, sequence, session.running()));
            assert!(fast.presented(drawn, now));
            assert!(fast.frame_callback(drawn));
            assert_eq!(fast.plan(now, generation, sequence, true), Decision::Idle);
            assert_eq!(slow.plan(now, generation, sequence, true), Decision::Idle);
            assert_eq!(preparer.budget.usage().workers, 1);
        }
        assert!(slow.frame_callback(stalled));
        assert_eq!(
            ticket(slow.plan(now, generation, session.current().sequence, true)).sequence(),
            40
        );
        // Transfer retirement away from the future Wayland event loop. The
        // reservation remains until kill/reap and all frame leases finish.
        let worker = session.into_worker();
        std::thread::spawn(move || drop(worker)).join().unwrap();
        assert_eq!(preparer.budget.usage(), Usage::default());
    }
}

#[test]
fn manual_lock_and_transition_pauses_are_independent_and_preserve_phase() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let preparer = preparer();
    let (mut session, rx, _) = prepare(&preparer, &source);
    session.set_paused(PauseReason::Manual, true, ms(20));
    session.set_paused(PauseReason::Transition, false, ms(100));
    assert!(!session.running());
    assert_eq!(session.next_deadline(), None);
    session.set_paused(PauseReason::SessionLock, true, ms(200));
    session.set_paused(PauseReason::Manual, false, ms(300));
    assert!(!session.running());
    session.set_paused(PauseReason::SessionLock, false, ms(500));
    assert_eq!(session.next_deadline(), Some(ms(510)));
    until(&mut session, &rx, ms(510), |s| s.current().sequence == 1);
    assert_eq!(session.next_deadline(), Some(ms(580)));
    session.set_paused(PauseReason::NoVisibleOutputs, true, ms(530));
    session.advance(ms(1000), session.generation());
    assert_eq!(session.current().sequence, 1);
    assert_eq!(session.next_deadline(), None);
    session.set_paused(PauseReason::NoVisibleOutputs, false, ms(1530));
    assert_eq!(session.next_deadline(), Some(ms(1580)));
    until(&mut session, &rx, ms(1580), |s| s.current().sequence == 2);
    drop(session);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn stalled_worker_suppresses_timers_and_recovers_without_catch_up_debt() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let preparer = preparer();
    let (mut session, rx, pid) = prepare(&preparer, &source);
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGSTOP) }, 0);
    session.set_paused(PauseReason::Transition, false, ms(0));
    assert!(session.advance(ms(10), session.generation()).waiting);
    assert_eq!(session.next_deadline(), None);
    assert!(session.advance(ms(10_000), session.generation()).waiting);
    assert_eq!(session.current().sequence, 0);
    assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGCONT) }, 0);
    until(&mut session, &rx, ms(10_000), |s| s.current().sequence == 1);
    assert_eq!(session.next_deadline(), Some(ms(10_070)));
    drop(session);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn new_application_has_new_clock_and_ignores_replaced_worker_wake() {
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let preparer = preparer();
    let (mut old, rx, _) = prepare(&preparer, &source);
    old.set_paused(PauseReason::Transition, false, ms(0));
    until(&mut old, &rx, ms(10), |s| s.current().sequence == 1);
    let (mut new, _, _) = prepare(&preparer, &source);
    assert_ne!(old.key(), new.key());
    assert_ne!(old.generation(), new.generation());
    new.set_paused(PauseReason::Transition, false, ms(20));
    new.advance(ms(60_000), old.generation());
    assert_eq!(new.current().sequence, 0);
    assert_eq!(new.next_deadline(), Some(ms(30)), "stale wake cannot move clock");
    drop((old, new));
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn later_decode_failure_freezes_last_frame_and_cannot_be_resumed() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("corrupt.webp");
    fixtures::webp(&path, 2, 1);
    let mut bytes = std::fs::read(&path).unwrap();
    let start = bytes
        .windows(4)
        .enumerate()
        .filter(|(_, b)| *b == b"VP8L")
        .nth(1)
        .unwrap()
        .0;
    let len = u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()) as usize;
    bytes[start + 13..start + 8 + len].fill(0xff);
    std::fs::write(&path, bytes).unwrap();
    let source = Source::open(&path, Limits::default()).unwrap();
    let preparer = preparer();
    let (mut session, rx, _) = prepare(&preparer, &source);
    session.set_paused(PauseReason::Transition, false, ms(0));
    until(&mut session, &rx, ms(10), |s| s.error().is_some());
    assert_eq!(session.current().sequence, 0);
    assert_eq!(session.current().frame().pixels.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(session.next_deadline(), None);
    assert!(!session.running());
    session.set_paused(PauseReason::Manual, false, ms(1000));
    session.advance(ms(1000), session.generation());
    assert!(session.error().is_some());
    assert_eq!(session.current().sequence, 0);
    assert_eq!(session.next_deadline(), None);
    drop(session);
    assert_eq!(preparer.budget.usage(), Usage::default());
}
