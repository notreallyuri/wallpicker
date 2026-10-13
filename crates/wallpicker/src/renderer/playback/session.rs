use super::{Advance, Timeline};
use crate::decoding::{DecodedFrame, Worker};
use crate::preparation::{Session, SessionKey};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

/// Independent reasons: releasing a lock/transition must not undo manual pause.
#[derive(Clone, Copy)]
pub enum PauseReason {
    Transition = 1,
    Manual = 2,
    SessionLock = 4,
    NoVisibleOutputs = 8,
    /// The compositor stopped asking for frames ([`super::Visibility`]).
    Hidden = 16,
}

/// One current source frame and one clock, irrespective of output count. The
/// worker owns its bounded queue. Slow outputs sample `current()` when ready;
/// they do not retain a separate CPU frame history.
///
/// Drop joins the decoder thread. Use into_worker to transfer retired ownership
/// to the future renderer's cleanup owner, away from the Wayland loop.
pub struct SourceSession {
    key: SessionKey,
    worker: Worker,
    current: Arc<DecodedFrame>,
    timeline: Timeline,
    pauses: u8,
    error: Option<String>,
}

impl SourceSession {
    pub fn new(session: Session, now: Duration) -> Self {
        let (key, worker, current) = session.into_parts();
        worker.pause(true);
        let timeline = Timeline::new(now, current.frame().playback_delay().expect("prepared animation"));
        Self {
            key,
            worker,
            current,
            timeline,
            pauses: PauseReason::Transition as u8,
            error: None,
        }
    }

    pub fn key(&self) -> &SessionKey {
        &self.key
    }
    pub fn generation(&self) -> u64 {
        self.current.generation
    }
    pub fn current(&self) -> &DecodedFrame {
        &self.current
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn running(&self) -> bool {
        self.pauses == 0 && self.error.is_none()
    }
    pub fn paused_by(&self, reason: PauseReason) -> bool {
        self.pauses & reason as u8 != 0
    }
    pub fn next_deadline(&self) -> Option<Duration> {
        self.timeline.next_deadline()
    }

    pub fn set_paused(&mut self, reason: PauseReason, paused: bool, now: Duration) {
        if paused {
            self.pauses |= reason as u8;
        } else {
            self.pauses &= !(reason as u8);
        }
        let paused = !self.running();
        self.timeline.set_paused(now, paused);
        self.worker.pause(paused);
    }

    /// Call on source deadlines or tagged decoder notifications. A stale wake
    /// does no work; a starved source sleeps until its next decoder notification.
    /// Frame consumption is bounded even if the worker is faster than the loop.
    pub fn advance(&mut self, now: Duration, generation: u64) -> Advance {
        if generation != self.generation() {
            return Advance::default();
        }
        let Self {
            worker,
            current,
            timeline,
            error,
            ..
        } = self;
        let result = timeline.advance(now, || match worker.try_next() {
            Some(Ok(frame)) => {
                if frame.generation != generation || frame.sequence != current.sequence.saturating_add(1) {
                    *error = Some("decoder returned an obsolete or out-of-order frame".into());
                    return Poll::Pending;
                }
                let Some(delay) = frame.frame().playback_delay() else {
                    *error = Some("decoder returned an untimed animation frame".into());
                    return Poll::Pending;
                };
                *current = Arc::new(frame);
                Poll::Ready(delay)
            }
            Some(Err(message)) => {
                *error = Some(message);
                Poll::Pending
            }
            None => {
                if worker.is_closed() {
                    *error = Some("decoder stopped before producing its next frame".into());
                }
                Poll::Pending
            }
        });
        if error.is_some() {
            timeline.set_paused(now, true);
            worker.pause(true);
        }
        result
    }

    /// Release the retained frame and transfer the worker to an off-loop owner
    /// for cancellation/reaping. No synchronous kill/join happens in this call.
    pub fn into_worker(self) -> Worker {
        self.worker
    }
}
