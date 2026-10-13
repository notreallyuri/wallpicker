//! Logical RGBA texture bytes, charged once per shared source texture. Driver
//! staging, EGL swapchain buffers and transition copies require separate budgets.
use std::{cell::Cell, rc::Rc};

#[derive(Clone)]
pub(in crate::renderer) struct Budget(Rc<Pool>);
struct Pool {
    limit: u64,
    used: Cell<u64>,
}
pub(super) struct Reservation {
    pool: Rc<Pool>,
    bytes: u64,
}

impl Budget {
    pub(in crate::renderer) fn new(bytes: u64) -> Self {
        Self(Rc::new(Pool {
            limit: bytes,
            used: Cell::new(0),
        }))
    }
    pub(in crate::renderer) fn used(&self) -> u64 {
        self.0.used.get()
    }
    pub(super) fn reserve(&self, bytes: u64) -> Result<Reservation, String> {
        let next = self
            .used()
            .checked_add(bytes)
            .filter(|n| *n <= self.0.limit)
            .ok_or("playback texture budget exhausted")?;
        self.0.used.set(next);
        Ok(Reservation {
            pool: self.0.clone(),
            bytes,
        })
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.pool.used.set(self.pool.used.get() - self.bytes);
    }
}
