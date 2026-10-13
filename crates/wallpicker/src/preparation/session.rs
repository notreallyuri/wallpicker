use crate::decoding::{DecodedFrame, Worker};
use std::sync::Arc;
use wallpicker_core::media::FileIdentity;

/// Grouping is scoped to an application, never just a source path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionKey {
    pub application: String,
    pub identity: FileIdentity,
}

/// Transferable session ownership. Dropping it cancels/reaps the decoder;
/// externally retained frames continue to hold their admission reservation.
pub struct Session {
    pub(super) key: SessionKey,
    pub(super) worker: Worker,
    pub(super) first: Arc<DecodedFrame>,
}

impl Session {
    pub fn key(&self) -> &SessionKey {
        &self.key
    }
    pub fn generation(&self) -> u64 {
        self.first.generation
    }
    pub fn first(&self) -> &Arc<DecodedFrame> {
        &self.first
    }
    pub fn process_id(&self) -> Option<u32> {
        self.worker.process_id()
    }
    pub fn pause(&self, paused: bool) {
        self.worker.pause(paused);
    }
    pub fn try_next(&mut self) -> Option<Result<DecodedFrame, String>> {
        self.worker.try_next()
    }

    /// Let the presentation owner release the initial frame after handoff;
    /// session ownership need not pin it for the entire playback lifetime.
    pub fn into_parts(self) -> (SessionKey, Worker, Arc<DecodedFrame>) {
        (self.key, self.worker, self.first)
    }
}
