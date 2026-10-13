//! Three frame slots include queued, receiving and caller-held frames. A caller
//! may share a frame via Arc, but cannot extract its allocation from the lease.
use super::budget::Reservation;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Default)]
struct State {
    held: usize,
    paused: bool,
    cancelled: bool,
}

#[derive(Default)]
pub(super) struct Slots {
    state: Mutex<State>,
    ready: Condvar,
}

pub(super) struct Slot {
    slots: Arc<Slots>,
    _reservation: Arc<Reservation>,
}

impl Slots {
    pub(super) fn acquire(self: &Arc<Self>, reservation: &Arc<Reservation>) -> Option<Slot> {
        let mut state = self.state.lock().unwrap();
        while !state.cancelled && (state.paused || state.held == 3) {
            state = self.ready.wait(state).unwrap();
        }
        if state.cancelled {
            return None;
        }
        state.held += 1;
        Some(Slot {
            slots: self.clone(),
            _reservation: reservation.clone(),
        })
    }

    pub(super) fn pause(&self, paused: bool) {
        self.state.lock().unwrap().paused = paused;
        self.ready.notify_all();
    }

    pub(super) fn cancel(&self) {
        self.state.lock().unwrap().cancelled = true;
        self.ready.notify_all();
    }
}

impl Drop for Slot {
    fn drop(&mut self) {
        self.slots.state.lock().unwrap().held -= 1;
        self.slots.ready.notify_all();
    }
}
