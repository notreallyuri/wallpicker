//! Reservations survive cancellation while callers still retain frame buffers.
use std::io;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Budget(Arc<Pool>);

struct Pool {
    bytes: u64,
    workers: usize,
    used: Mutex<Usage>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    pub bytes: u64,
    pub workers: usize,
}

pub(super) struct Reservation {
    pool: Arc<Pool>,
    bytes: u64,
}

impl Budget {
    /// No workers/allocations are started by constructing an admission pool.
    /// Limits are experimental policy inputs, not shipping playback defaults.
    pub fn new(bytes: u64, workers: usize) -> Self {
        Self(Arc::new(Pool {
            bytes,
            workers,
            used: Mutex::new(Usage::default()),
        }))
    }

    pub fn usage(&self) -> Usage {
        *self.0.used.lock().unwrap()
    }

    pub(super) fn reserve(&self, bytes: u64) -> io::Result<Arc<Reservation>> {
        let mut used = self.0.used.lock().unwrap();
        if used.workers >= self.0.workers || bytes > self.0.bytes.saturating_sub(used.bytes) {
            return Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "decoder admission budget exhausted",
            ));
        }
        used.bytes += bytes;
        used.workers += 1;
        Ok(Arc::new(Reservation {
            pool: self.0.clone(),
            bytes,
        }))
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        let mut used = self.pool.used.lock().unwrap();
        used.bytes -= self.bytes;
        used.workers -= 1;
    }
}
