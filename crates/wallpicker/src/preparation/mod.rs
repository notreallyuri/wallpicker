//! Batch preflight and owned decoder sessions, independent of Wayland/GPU state.
//! Construct off the renderer loop; dropping a failed/unaccepted preparation
//! cancels and reaps all workers. Static-only batches never create workers.
mod inspect;
mod session;

use crate::decoding::{Budget, Options, Worker};
use inspect::Inspection;
pub use session::{Session, SessionKey};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use wallpicker_core::{
    media::Limits,
    render::{Batch, Intent},
};

pub struct Preparer {
    pub executable: PathBuf,
    pub budget: Budget,
    pub options: Options,
    pub limits: Limits,
}

/// All outputs and first frames are ready, but playback has not started.
/// Runtime resources are private and cannot be forged through JSON requests.
pub struct PreparedBatch {
    batch: Batch,
    inspection: Inspection,
    sessions: Vec<Session>,
}

/// Ownership handed to a presentation owner after a final identity check.
/// Output `i` uses `groups[i]`, or None for a static picture.
pub struct Handoff {
    pub batch: Batch,
    pub groups: Vec<Option<usize>>,
    pub sessions: Vec<Session>,
}

/// Geometry-only preflight. It validates sources and static content off-loop,
/// but never admits a decoder: animated outputs must reuse a live session.
pub struct PreparedAdjustment {
    batch: Batch,
    inspection: Inspection,
}

impl PreparedAdjustment {
    pub fn new(batch: Batch, limits: Limits) -> Result<Self, String> {
        if batch.intent != Intent::Adjust {
            return Err("adjustment preparation requires Adjust intent".into());
        }
        let inspection = Inspection::new(&batch, limits)?;
        Ok(Self { batch, inspection })
    }

    pub fn batch(&self) -> &Batch {
        &self.batch
    }

    /// Recheck every alias immediately before queuing the renderer operation.
    pub fn handoff(self) -> Result<Batch, String> {
        self.inspection.revalidate()?;
        Ok(self.batch)
    }
}

impl Preparer {
    /// All file/geometry checks precede worker startup. One paused worker is
    /// shared by equal identities within this application. A failed first frame
    /// or admission releases earlier workers before returning an error.
    pub fn prepare(&self, batch: Batch, wake: Arc<dyn Fn(u64) + Send + Sync>) -> Result<PreparedBatch, String> {
        let inspection = Inspection::new(&batch, self.limits)?;
        let mut sessions = Vec::new();
        for source in &inspection.sources {
            static NEXT: AtomicU64 = AtomicU64::new(1);
            let generation = NEXT
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
                .map_err(|_| "decoder generation exhausted")?;
            let notify = wake.clone();
            let (worker, first) = Worker::prepare(
                &self.executable,
                source.clone(),
                generation,
                &self.budget,
                self.options,
                Arc::new(move || notify(generation)),
            )
            .map_err(|e| e.to_string())?;
            sessions.push(Session {
                key: SessionKey {
                    application: batch.application.clone(),
                    identity: source.identity().clone(),
                },
                worker,
                first: Arc::new(first),
            });
        }
        inspection.revalidate()?;
        Ok(PreparedBatch {
            batch,
            inspection,
            sessions,
        })
    }
}

impl PreparedBatch {
    /// With playback off (or failed to admit): the same preflight
    /// and identity checks, but no workers. Animated outputs keep their
    /// identity and the renderer shows their first frame as a still.
    pub fn without_playback(batch: Batch) -> Result<Self, String> {
        let mut inspection = Inspection::new(&batch, Limits::default())?;
        // No sessions back these groups.
        inspection.groups.fill(None);
        Ok(Self {
            batch,
            inspection,
            sessions: vec![],
        })
    }

    pub fn batch(&self) -> &Batch {
        &self.batch
    }
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }
    pub fn session_for(&self, monitor: &str) -> Option<&Session> {
        let output = self.batch.outputs.iter().position(|o| o.monitor == monitor)?;
        self.inspection.groups[output].map(|group| &self.sessions[group])
    }

    /// Recheck every source alias and prepared static path immediately before
    /// ownership transfer. Failure drops the preparation and its workers.
    pub fn handoff(self) -> Result<Handoff, String> {
        self.inspection.revalidate()?;
        Ok(Handoff {
            batch: self.batch,
            groups: self.inspection.groups,
            sessions: self.sessions,
        })
    }
}
