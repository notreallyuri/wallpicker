use std::time::Duration;

/// Output generation must change when its surface/geometry or session changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Target {
    pub session: u64,
    pub output: u64,
}

/// Identifies one successful swap and its Wayland frame callback. Serial tags
/// also reject duplicate callbacks from an earlier swap on the same target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ticket {
    target: Target,
    serial: u64,
    sequence: u64,
}

impl Ticket {
    pub fn target(self) -> Target {
        self.target
    }
    pub fn sequence(self) -> u64 {
        self.sequence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Idle,
    WakeAt(Duration),
    Present(Ticket),
}

pub struct OutputPacer {
    target: Target,
    interval: Duration,
    last_presented: Option<(u64, Duration)>,
    pending: Option<Ticket>,
    serial: u64,
}

impl OutputPacer {
    /// The presentation interval is explicit policy, independent of source
    /// durations. Zero disables the cap; callbacks still provide backpressure.
    pub fn new(target: Target, interval: Duration) -> Self {
        Self {
            target,
            interval,
            last_presented: None,
            pending: None,
            serial: 0,
        }
    }

    /// A new presentation cap, from the next decision on.
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    /// Consult after advancing the shared source. Request a callback only for
    /// Present, never for Idle/WakeAt. No frame data is retained per output.
    pub fn plan(&self, now: Duration, generation: u64, sequence: u64, running: bool) -> Decision {
        if !running || generation != self.target.session || self.pending.is_some() {
            return Decision::Idle;
        }
        if let Some((shown, when)) = self.last_presented {
            if sequence <= shown {
                return Decision::Idle;
            }
            let due = when.saturating_add(self.interval);
            if now < due {
                return Decision::WakeAt(due);
            }
        }
        let Some(serial) = self.serial.checked_add(1) else {
            return Decision::Idle;
        };
        Decision::Present(Ticket {
            target: self.target,
            serial,
            sequence,
        })
    }

    /// Call after a successful draw/swap. A failed swap leaves this gate ready
    /// for the caller's failure/fallback policy, without inventing a callback.
    pub fn presented(&mut self, ticket: Ticket, now: Duration) -> bool {
        if self.plan(now, ticket.target.session, ticket.sequence, true) != Decision::Present(ticket) {
            return false;
        }
        self.serial = ticket.serial;
        self.last_presented = Some((ticket.sequence, now));
        self.pending = Some(ticket);
        true
    }

    pub fn frame_callback(&mut self, ticket: Ticket) -> bool {
        if self.pending != Some(ticket) {
            return false;
        }
        self.pending = None;
        true
    }
}
