//! The production calloop adapter with real timers/decoders and a recording draw
//! hook. No Wayland/GPU required; callback tickets use the same Signals API.
#[path = "decoding/fixtures.rs"]
mod fixtures;

use smithay_client_toolkit::reexports::calloop::EventLoop;
use std::num::NonZeroUsize;
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::time::{Duration, Instant};
use wallpicker::decoding::{Budget, Options, Usage};
use wallpicker::playback::{Host, PauseReason, Runtime, Target, Ticket};
use wallpicker::preparation::{Preparer, Session};
use wallpicker_core::{
    media::{Limits, Source},
    render::{Batch, Content, Geometry, Intent, Output, Picture},
};

struct TestHost {
    driver: Runtime<Self>,
    drawn: Vec<Ticket>,
    fail: Option<Target>,
    stalled: Option<Target>,
}

impl Host for TestHost {
    fn playback(&mut self) -> &mut Runtime<Self> {
        &mut self.driver
    }
    fn present(&mut self, ticket: Ticket) -> Result<(), String> {
        if self.fail == Some(ticket.target()) {
            return Err("simulated EGL swap failure".into());
        }
        let frame = self.driver.source(ticket.target().session).unwrap().current();
        assert_eq!(frame.sequence, ticket.sequence());
        let color = if frame.sequence.is_multiple_of(2) {
            [255, 0, 0, 255]
        } else {
            [0, 255, 0, 255]
        };
        assert_eq!(frame.frame().pixels.get_pixel(0, 0).0, color);
        self.drawn.push(ticket);
        if self.stalled != Some(ticket.target()) {
            self.driver.signals().frame_callback(ticket);
        }
        Ok(())
    }
}

fn setup(capacity: usize) -> (EventLoop<'static, TestHost>, TestHost, Preparer) {
    let event_loop = EventLoop::try_new().unwrap();
    let host = TestHost {
        driver: Runtime::new(event_loop.handle(), NonZeroUsize::new(capacity).unwrap()).unwrap(),
        drawn: vec![],
        fail: None,
        stalled: None,
    };
    let preparer = Preparer {
        executable: env!("CARGO_BIN_EXE_wallpicker").into(),
        budget: Budget::new(512 * 1024 * 1024, 3),
        options: Options {
            address_space_bytes: 128 * 1024 * 1024,
            frame_allowance: 0,
            frame_timeout: Duration::from_secs(5),
        },
        limits: Limits::default(),
    };
    (event_loop, host, preparer)
}
fn prepare(host: &TestHost, preparer: &Preparer, source: &Source) -> Session {
    prepare_with_wake(preparer, source, host.driver.signals().decoder_wake())
}
fn prepare_with_wake(preparer: &Preparer, source: &Source, wake: Arc<dyn Fn(u64) + Send + Sync>) -> Session {
    let batch = Batch::new(
        Intent::Apply,
        Default::default(),
        vec![Output {
            monitor: "A".into(),
            picture: Picture {
                source: source.path().into(),
                geometry: Geometry::default(),
                content: Content::Animated {
                    identity: source.identity().clone(),
                },
            },
        }],
    );
    preparer
        .prepare(batch, wake)
        .unwrap()
        .handoff()
        .unwrap()
        .sessions
        .pop()
        .unwrap()
}
fn insert(host: &mut TestHost, session: Session) -> u64 {
    host.driver
        .insert(session)
        .unwrap_or_else(|_| panic!("unexpected admission failure"))
}
fn until(event_loop: &mut EventLoop<'static, TestHost>, host: &mut TestHost, done: impl Fn(&TestHost) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done(host) {
        assert!(Instant::now() < deadline, "event loop did not reach expected state");
        event_loop.dispatch(Some(Duration::from_millis(50)), host).unwrap();
        assert!(host.driver.error().is_none(), "{:?}", host.driver.error());
    }
}
fn drain(event_loop: &mut EventLoop<'static, TestHost>, host: &mut TestHost) {
    for _ in 0..5 {
        event_loop.dispatch(Some(Duration::ZERO), host).unwrap();
    }
}

#[test]
fn empty_and_frozen_hosts_have_no_timer_and_initial_frame_draws_once() {
    let (mut event_loop, mut host, preparer) = setup(1);
    drain(&mut event_loop, &mut host);
    assert!(!host.driver.timer_armed());
    assert!(host.drawn.is_empty());
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let session = prepare(&host, &preparer, &source);
    let generation = insert(&mut host, session);
    let target = host.driver.attach(generation, 1, Duration::ZERO).unwrap();
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 1);
    drain(&mut event_loop, &mut host);
    assert!(!host.driver.timer_armed());
    assert_eq!(host.driver.source(generation).unwrap().current().sequence, 0);
    let first = host.drawn[0];
    for _ in 0..4 {
        host.driver.signals().frame_callback(first);
    }
    drain(&mut event_loop, &mut host);
    assert_eq!(host.drawn.len(), 1);
    host.driver.detach(target);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    assert_eq!(preparer.budget.usage(), Usage::default());
    assert!(!host.driver.timer_armed());
}

#[test]
fn real_timers_keep_fast_output_moving_while_other_callback_is_stalled() {
    let (mut event_loop, mut host, preparer) = setup(1);
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::webp(&tmp.path().join("source.webp"), 2, 1);
    let session = prepare(&host, &preparer, &source);
    let generation = insert(&mut host, session);
    let fast = host.driver.attach(generation, 1, Duration::ZERO).unwrap();
    let slow = host.driver.attach(generation, 2, Duration::ZERO).unwrap();
    host.stalled = Some(slow);
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 2);
    let stalled = *host.drawn.iter().find(|t| t.target() == slow).unwrap();
    host.driver.set_paused(generation, PauseReason::Transition, false);
    until(&mut event_loop, &mut host, |h| {
        h.drawn.iter().any(|t| t.target() == fast && t.sequence() >= 4)
    });
    assert_eq!(host.drawn.iter().filter(|t| t.target() == slow).count(), 1);
    host.driver.signals().frame_callback(stalled);
    until(&mut event_loop, &mut host, |h| {
        h.drawn.iter().filter(|t| t.target() == slow).count() == 2
    });
    assert!(host.drawn.iter().rev().find(|t| t.target() == slow).unwrap().sequence() >= 4);
    host.driver.set_paused(generation, PauseReason::Manual, true);
    drain(&mut event_loop, &mut host);
    assert!(!host.driver.timer_armed());
    let count = host.drawn.len();
    event_loop
        .dispatch(Some(Duration::from_millis(100)), &mut host)
        .unwrap();
    assert_eq!(host.drawn.len(), count);
    host.driver.detach(fast);
    assert_eq!(host.driver.admitted(), 1);
    host.driver.detach(slow);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn retirement_counts_against_admission_and_stale_events_cannot_touch_replacement() {
    let (mut event_loop, mut host, preparer) = setup(1);
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let session = prepare(&host, &preparer, &source);
    let old = insert(&mut host, session);
    let output = host.driver.attach(old, 1, Duration::ZERO).unwrap();
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 1);
    let callback = host.drawn[0];
    let replacement = prepare(&host, &preparer, &source);
    host.driver.detach(output);
    assert_eq!(host.driver.admitted(), 1, "wait for cleanup acknowledgement");
    let replacement = host
        .driver
        .insert(replacement)
        .expect_err("retirement still occupies its slot");
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    let new = insert(&mut host, *replacement);
    assert_ne!(new, old);
    let new_output = host.driver.attach(new, 2, Duration::ZERO).unwrap();
    host.driver.signals().decoder_wake()(old);
    host.driver.signals().frame_callback(callback);
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 2);
    drain(&mut event_loop, &mut host);
    assert_eq!(host.driver.source(new).unwrap().current().sequence, 0);
    assert!(!host.driver.timer_armed());
    host.driver.detach(new_output);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn failed_output_freezes_independently_and_last_failed_consumer_stops_source() {
    let (mut event_loop, mut host, preparer) = setup(1);
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let session = prepare(&host, &preparer, &source);
    let generation = insert(&mut host, session);
    let failed = host.driver.attach(generation, 1, Duration::ZERO).unwrap();
    let healthy = host.driver.attach(generation, 2, Duration::ZERO).unwrap();
    host.fail = Some(failed);
    until(&mut event_loop, &mut host, |h| !h.drawn.is_empty());
    assert!(host.driver.output_error(failed).unwrap().contains("swap failure"));
    host.driver.set_paused(generation, PauseReason::Transition, false);
    until(&mut event_loop, &mut host, |h| {
        h.drawn.iter().any(|t| t.sequence() >= 2)
    });
    assert!(host.drawn.iter().all(|t| t.target() == healthy));
    host.driver.detach(healthy);
    drain(&mut event_loop, &mut host);
    assert!(!host.driver.source(generation).unwrap().running());
    assert!(!host.driver.timer_armed());
    host.driver.detach(failed);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn shutdown_joins_active_and_retiring_workers_even_without_more_dispatch() {
    let (mut event_loop, mut host, preparer) = setup(2);
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let first = prepare(&host, &preparer, &source);
    let second = prepare(&host, &preparer, &source);
    let first = insert(&mut host, first);
    let second = insert(&mut host, second);
    let target = host.driver.attach(first, 1, Duration::ZERO).unwrap();
    host.driver.attach(second, 2, Duration::ZERO).unwrap();
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 2);
    host.driver.detach(target);
    drop(host);
    assert_eq!(preparer.budget.usage(), Usage::default());
    drop(event_loop);
}

#[test]
fn unconfigured_consumers_keep_source_alive_until_the_host_retires_it() {
    let (mut event_loop, mut host, preparer) = setup(1);
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let session = prepare(&host, &preparer, &source);
    let generation = insert(&mut host, session);
    let first = host.driver.attach(generation, 1, Duration::ZERO).unwrap();
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 1);
    host.driver.unbind(first);
    drain(&mut event_loop, &mut host);
    assert_eq!(host.driver.admitted(), 1);
    assert!(!host.driver.timer_armed());
    assert!(host.driver.source(generation).is_some());
    host.driver.attach(generation, 2, Duration::ZERO).unwrap();
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 2);
    host.driver.set_paused(generation, PauseReason::Transition, false);
    until(&mut event_loop, &mut host, |h| {
        h.drawn.iter().any(|t| t.sequence() >= 1)
    });
    host.driver.retire(generation);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn replacement_suspends_one_output_and_resumes_without_restarting_source() {
    let (mut event_loop, mut host, preparer) = setup(1);
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let session = prepare(&host, &preparer, &source);
    let generation = insert(&mut host, session);
    let first = host.driver.attach(generation, 1, Duration::ZERO).unwrap();
    let second = host.driver.attach(generation, 2, Duration::ZERO).unwrap();
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 2);
    host.driver.suspend(first, true);
    host.driver.set_paused(generation, PauseReason::Transition, false);
    until(&mut event_loop, &mut host, |h| {
        h.drawn.iter().any(|t| t.target() == second && t.sequence() >= 2)
    });
    assert_eq!(host.drawn.iter().filter(|t| t.target() == first).count(), 1);
    host.driver.suspend(second, true);
    drain(&mut event_loop, &mut host);
    assert!(!host.driver.timer_armed());
    let sequence = host.driver.source(generation).unwrap().current().sequence;
    host.driver.suspend(first, false);
    until(&mut event_loop, &mut host, |h| {
        h.drawn.iter().any(|t| t.target() == first && t.sequence() >= sequence)
    });
    assert!(host.driver.source(generation).unwrap().current().sequence >= sequence);
    host.driver.retire(generation);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    assert_eq!(preparer.budget.usage(), Usage::default());
}

#[test]
fn blocked_worker_join_cannot_block_another_source_or_release_admission_early() {
    let (mut event_loop, mut host, preparer) = setup(2);
    // The guard also unblocks cleanup if an assertion panics, before host Drop.
    struct Release(Arc<(Mutex<bool>, Condvar)>);
    impl Drop for Release {
        fn drop(&mut self) {
            *self.0.0.lock().unwrap() = true;
            self.0.1.notify_all();
        }
    }
    let release = Release(Arc::new((Mutex::new(false), Condvar::new())));
    let gate = release.0.clone();
    let (entered, blocked) = mpsc::channel();
    let calls = AtomicUsize::new(0);
    let wake = host.driver.signals().decoder_wake();
    let wake = Arc::new(move |generation| {
        wake(generation);
        if calls.fetch_add(1, Ordering::SeqCst) == 1 {
            entered.send(()).unwrap();
            // Bound a regression failure too: joining on the event-loop thread
            // should fail the test, not leave the test process hung forever.
            let (_guard, timeout) = gate
                .1
                .wait_timeout_while(gate.0.lock().unwrap(), Duration::from_secs(5), |open| !*open)
                .unwrap();
            assert!(!timeout.timed_out(), "event loop blocked on worker cleanup");
        }
    });
    let tmp = tempfile::tempdir().unwrap();
    let source = fixtures::gif(&tmp.path().join("source.gif"), 2, 1);
    let first = prepare_with_wake(&preparer, &source, wake);
    let second = prepare(&host, &preparer, &source);
    let first = insert(&mut host, first);
    let second = insert(&mut host, second);
    let old = host.driver.attach(first, 1, Duration::ZERO).unwrap();
    let healthy = host.driver.attach(second, 2, Duration::ZERO).unwrap();
    until(&mut event_loop, &mut host, |h| h.drawn.len() == 2);
    host.driver.set_paused(first, PauseReason::Transition, false);
    blocked.recv_timeout(Duration::from_secs(5)).unwrap();
    host.driver.detach(old); // Must return without joining the blocked pump.
    host.driver.set_paused(second, PauseReason::Transition, false);
    until(&mut event_loop, &mut host, |h| {
        h.drawn.iter().any(|t| t.target() == healthy && t.sequence() >= 2)
    });
    assert_eq!(
        host.driver.admitted(),
        2,
        "retirement remains charged until join finishes"
    );
    drop(release);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 1);
    host.driver.detach(healthy);
    until(&mut event_loop, &mut host, |h| h.driver.admitted() == 0);
    assert_eq!(preparer.budget.usage(), Usage::default());
}
