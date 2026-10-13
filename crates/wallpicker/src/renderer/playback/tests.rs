use super::*;
use std::task::Poll;
use std::time::Duration;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}
fn clock(delay: u64) -> Timeline {
    let mut clock = Timeline::new(ms(0), ms(delay));
    clock.set_paused(ms(0), false);
    clock
}
fn ticket(decision: Decision) -> Ticket {
    match decision {
        Decision::Present(ticket) => ticket,
        other => panic!("expected presentation, got {other:?}"),
    }
}
fn target(output: u64) -> Target {
    Target { session: 7, output }
}

#[test]
fn first_frame_starts_at_release_and_variable_delays_are_preserved() {
    let mut clock = Timeline::new(ms(50), ms(0));
    assert_eq!(clock.next_deadline(), None);
    assert_eq!(clock.advance(ms(150), || panic!("paused")), Advance::default());
    clock.set_paused(ms(200), false);
    assert_eq!(clock.next_deadline(), Some(ms(210)));
    clock.advance(ms(209), || panic!("early"));
    assert_eq!(clock.advance(ms(210), || Poll::Ready(ms(70))).frames, 1);
    assert_eq!(clock.next_deadline(), Some(ms(280)));
    assert_eq!(clock.advance(ms(280), || Poll::Ready(ms(1))).frames, 1);
    assert_eq!(clock.next_deadline(), Some(ms(290)));
}

#[test]
fn catch_up_is_bounded_and_rebases_instead_of_rearming_overdue_timer() {
    let mut clock = clock(10);
    let mut decoded = 0;
    let result = clock.advance(ms(60_000), || {
        decoded += 1;
        Poll::Ready(ms(20))
    });
    assert_eq!(decoded, MAX_CATCH_UP);
    assert_eq!(
        result,
        Advance {
            frames: MAX_CATCH_UP,
            rebased: true,
            waiting: false
        }
    );
    assert_eq!(clock.next_deadline(), Some(ms(60_020)));
    clock.advance(ms(60_000), || panic!("must sleep"));
}

#[test]
fn moderate_lateness_keeps_source_phase_and_consumes_in_order() {
    let mut clock = clock(10);
    let mut delays = [ms(20), ms(40)].into_iter();
    let result = clock.advance(ms(35), || Poll::Ready(delays.next().unwrap()));
    assert_eq!(result.frames, 2);
    assert!(!result.rebased);
    assert_eq!(clock.next_deadline(), Some(ms(70)));
}

#[test]
fn starvation_waits_for_notification_then_rebases_one_new_frame() {
    let mut clock = clock(10);
    assert!(clock.advance(ms(10), || Poll::Pending).waiting);
    assert_eq!(clock.next_deadline(), None);
    assert!(clock.advance(ms(500), || Poll::Pending).waiting);
    let result = clock.advance(ms(900), || Poll::Ready(ms(70)));
    assert_eq!(
        result,
        Advance {
            frames: 1,
            rebased: true,
            waiting: false
        }
    );
    assert_eq!(clock.next_deadline(), Some(ms(970)));
}

#[test]
fn pause_preserves_remaining_delay_and_repeated_pause_does_not_move_epoch() {
    let mut clock = clock(70);
    clock.set_paused(ms(20), true);
    clock.set_paused(ms(50), true);
    clock.advance(ms(1000), || panic!("paused"));
    assert_eq!(clock.next_deadline(), None);
    clock.set_paused(ms(1020), false);
    clock.set_paused(ms(1030), false);
    assert_eq!(clock.next_deadline(), Some(ms(1070)));
    clock.advance(ms(1069), || panic!("no catch-up debt"));
    assert_eq!(clock.advance(ms(1070), || Poll::Ready(ms(10))).frames, 1);
}

#[test]
fn resume_retries_a_starved_queue_even_if_its_wake_arrived_while_paused() {
    let mut clock = clock(10);
    assert!(clock.advance(ms(10), || Poll::Pending).waiting);
    clock.set_paused(ms(20), true);
    // Worker notification is handled during pause; the frame remains queued.
    clock.advance(ms(30), || panic!("paused frame must stay queued"));
    assert_eq!(clock.next_deadline(), None);
    clock.set_paused(ms(1000), false);
    assert_eq!(clock.next_deadline(), Some(ms(1000)), "resume must arrange a retry");
    assert_eq!(clock.advance(ms(1000), || Poll::Ready(ms(70))).frames, 1);
    assert_eq!(clock.next_deadline(), Some(ms(1070)));
}

#[test]
fn backwards_injected_time_and_overflow_cannot_create_spin() {
    let mut clock = clock(10);
    clock.advance(ms(9), || panic!("early"));
    clock.set_paused(ms(1), true); // clamped to 9
    clock.set_paused(ms(20), false);
    assert_eq!(clock.next_deadline(), Some(ms(21)));
    clock.advance(Duration::MAX, || Poll::Ready(Duration::MAX));
    assert_eq!(clock.next_deadline(), None);
    clock.advance(Duration::MAX, || panic!("no representable future time"));
}

#[test]
fn early_callback_does_not_request_repeated_presentation() {
    let mut output = OutputPacer::new(target(1), ms(0));
    let first = ticket(output.plan(ms(0), 7, 0, true));
    assert!(output.presented(first, ms(0)));
    assert!(output.frame_callback(first));
    for t in 1..70 {
        assert_eq!(output.plan(ms(t), 7, 0, true), Decision::Idle);
    }
    assert!(matches!(output.plan(ms(70), 7, 1, true), Decision::Present(_)));
}

#[test]
fn output_cap_skips_presentations_without_altering_source_clock() {
    let mut source = clock(10);
    let mut output = OutputPacer::new(target(1), ms(35));
    let first = ticket(output.plan(ms(0), 7, 0, true));
    assert!(output.presented(first, ms(0)));
    assert!(output.frame_callback(first));
    for sequence in 1..=3 {
        assert_eq!(source.advance(ms(sequence * 10), || Poll::Ready(ms(10))).frames, 1);
        assert_eq!(
            output.plan(ms(sequence * 10), 7, sequence, true),
            Decision::WakeAt(ms(35))
        );
    }
    let latest = ticket(output.plan(ms(35), 7, 3, true));
    assert_eq!(latest.sequence(), 3);
    assert!(output.presented(latest, ms(35)));
    assert_eq!(source.next_deadline(), Some(ms(40)));
}

#[test]
fn a_new_cap_applies_to_the_next_decision() {
    let mut output = OutputPacer::new(target(1), ms(100));
    let first = ticket(output.plan(ms(0), 7, 0, true));
    assert!(output.presented(first, ms(0)));
    assert!(output.frame_callback(first));
    assert_eq!(output.plan(ms(20), 7, 1, true), Decision::WakeAt(ms(100)));
    // Raised: the waiting frame is due already; lowered again: it waits.
    output.set_interval(ms(10));
    assert!(matches!(output.plan(ms(20), 7, 1, true), Decision::Present(_)));
    output.set_interval(ms(50));
    assert_eq!(output.plan(ms(20), 7, 1, true), Decision::WakeAt(ms(50)));
    // No cap: as soon as the callback allows.
    output.set_interval(ms(0));
    assert!(matches!(output.plan(ms(1), 7, 1, true), Decision::Present(_)));
}

#[test]
fn slow_output_does_not_block_other_output_or_hold_frame_history() {
    let mut fast = OutputPacer::new(target(1), ms(0));
    let mut slow = OutputPacer::new(target(2), ms(0));
    let stalled = ticket(slow.plan(ms(0), 7, 0, true));
    slow.presented(stalled, ms(0));
    for sequence in 0..10 {
        let t = ms(sequence * 10);
        let drawn = ticket(fast.plan(t, 7, sequence, true));
        assert!(fast.presented(drawn, t));
        assert!(fast.frame_callback(drawn));
        assert_eq!(slow.plan(t, 7, sequence, true), Decision::Idle);
    }
    assert!(slow.frame_callback(stalled));
    assert_eq!(ticket(slow.plan(ms(100), 7, 9, true)).sequence(), 9);
}

#[test]
fn generations_and_callback_serials_reject_obsolete_work() {
    let mut output = OutputPacer::new(target(1), ms(0));
    assert_eq!(output.plan(ms(0), 6, 0, true), Decision::Idle);
    let first = ticket(output.plan(ms(0), 7, 0, true));
    output.presented(first, ms(0));
    assert!(!output.presented(first, ms(0)));
    assert!(output.frame_callback(first));
    let second = ticket(output.plan(ms(10), 7, 1, true));
    output.presented(second, ms(10));
    assert!(!output.frame_callback(first));
    assert_eq!(output.plan(ms(20), 7, 2, true), Decision::Idle);
    let mut replacement = OutputPacer::new(target(2), ms(0));
    assert!(!replacement.presented(second, ms(20)));
    assert!(!replacement.frame_callback(second));
    let mut new_session = OutputPacer::new(Target { session: 8, output: 1 }, ms(0));
    assert!(!new_session.frame_callback(second));
    assert!(!new_session.presented(second, ms(20)));
    assert!(output.frame_callback(second));
    assert_eq!(output.plan(ms(20), 7, 0, true), Decision::Idle);
}

#[test]
fn paused_output_schedules_nothing_and_failed_swap_does_not_consume_gate() {
    let mut output = OutputPacer::new(target(1), ms(20));
    assert_eq!(output.plan(ms(0), 7, 0, false), Decision::Idle);
    let proposed = ticket(output.plan(ms(0), 7, 0, true));
    assert_eq!(ticket(output.plan(ms(10), 7, 0, true)), proposed);
    assert!(output.presented(proposed, ms(10)));
    assert!(output.frame_callback(proposed));
    assert_eq!(output.plan(ms(20), 7, 1, false), Decision::Idle);
    assert_eq!(output.plan(ms(20), 7, 1, true), Decision::WakeAt(ms(30)));
}

#[test]
fn a_source_is_hidden_once_every_output_waits_a_second_for_its_callback() {
    let mut shown = OutputPacer::new(target(1), ms(0));
    let mut covered = OutputPacer::new(target(2), ms(0));
    assert_eq!(visibility([], ms(0)), Visibility::Shown);
    assert_eq!(visibility([covered.waiting_since()], ms(0)), Visibility::Shown);
    let first = ticket(covered.plan(ms(100), 7, 0, true));
    assert!(covered.presented(first, ms(100)));
    assert_eq!(covered.waiting_since(), Some(ms(100)));
    assert_eq!(visibility([covered.waiting_since()], ms(500)), Visibility::HiddenAt(ms(1100)));
    assert_eq!(visibility([covered.waiting_since()], ms(1100)), Visibility::Hidden);
    // One output still asking for frames keeps the source shown.
    let drawn = ticket(shown.plan(ms(0), 7, 0, true));
    assert!(shown.presented(drawn, ms(0)));
    assert!(shown.frame_callback(drawn));
    assert_eq!(
        visibility([shown.waiting_since(), covered.waiting_since()], ms(5000)),
        Visibility::Shown
    );
    // Both waiting: hidden when the later one has waited long enough.
    let drawn = ticket(shown.plan(ms(600), 7, 1, true));
    assert!(shown.presented(drawn, ms(600)));
    assert_eq!(
        visibility([shown.waiting_since(), covered.waiting_since()], ms(1100)),
        Visibility::HiddenAt(ms(1600))
    );
    // Its callback arrives: shown again.
    assert!(covered.frame_callback(first));
    assert_eq!(covered.waiting_since(), None);
    assert_eq!(
        visibility([shown.waiting_since(), covered.waiting_since()], ms(9000)),
        Visibility::Shown
    );
}
