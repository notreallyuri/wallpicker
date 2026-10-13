use std::task::Poll;
use std::time::Duration;

/// Maximum composited frames consumed in one event-loop dispatch. The decoder
/// still composites every source frame; this only skips obsolete presentations.
pub const MAX_CATCH_UP: usize = 4;
const MIN_DELAY: Duration = Duration::from_millis(10);

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Advance {
    pub frames: usize,
    pub rebased: bool,
    pub waiting: bool,
}

pub struct Timeline {
    deadline: Duration,
    delay: Duration,
    paused_at: Option<Duration>,
    last_now: Duration,
    waiting: bool,
}

impl Timeline {
    /// Starts frozen on the prepared first frame. Its full delay begins when
    /// the transition/application owner releases the initial pause.
    pub fn new(now: Duration, delay: Duration) -> Self {
        let delay = delay.max(MIN_DELAY);
        Self {
            deadline: now.saturating_add(delay),
            delay,
            paused_at: Some(now),
            last_now: now,
            waiting: false,
        }
    }

    fn now(&mut self, now: Duration) -> Duration {
        self.last_now = self.last_now.max(now);
        self.last_now
    }

    pub fn set_paused(&mut self, now: Duration, paused: bool) {
        let now = self.now(now);
        match (self.paused_at, paused) {
            (None, true) => self.paused_at = Some(now),
            (Some(start), false) => {
                if self.waiting {
                    // A frame may have arrived while paused, with its wake
                    // already handled. Retry on resume instead of waiting for
                    // another notification from a now-full decoder queue.
                    self.deadline = now;
                    self.waiting = false;
                } else {
                    self.deadline = self.deadline.saturating_add(now - start);
                }
                self.paused_at = None;
            }
            _ => {}
        }
    }

    pub fn paused(&self) -> bool {
        self.paused_at.is_some()
    }

    /// No timer while paused or waiting for a decoder notification. In
    /// particular, an empty overdue queue must not rearm an expired timer.
    pub fn next_deadline(&self) -> Option<Duration> {
        (!self.paused() && !self.waiting && self.deadline < Duration::MAX).then_some(self.deadline)
    }

    /// `next` must be nonblocking and consume one fully composited frame. It
    /// returns that frame's duration, preserving variable source timing.
    pub fn advance(&mut self, now: Duration, mut next: impl FnMut() -> Poll<Duration>) -> Advance {
        let now = self.now(now);
        let mut result = Advance::default();
        if self.paused() {
            return result;
        }
        while self.deadline <= now && self.deadline < Duration::MAX {
            if result.frames == MAX_CATCH_UP {
                self.deadline = now.saturating_add(self.delay);
                result.rebased = true;
                break;
            }
            match next() {
                Poll::Ready(delay) => {
                    self.delay = delay.max(MIN_DELAY);
                    // Once starved, start the newly available frame at `now`.
                    // Do not turn a slow decoder into repeated catch-up bursts.
                    let base = if self.waiting { now } else { self.deadline };
                    result.rebased |= self.waiting;
                    self.deadline = base.saturating_add(self.delay);
                    self.waiting = false;
                    result.frames += 1;
                }
                Poll::Pending => {
                    self.waiting = true;
                    break;
                }
            }
        }
        result.waiting = self.waiting;
        result
    }
}
