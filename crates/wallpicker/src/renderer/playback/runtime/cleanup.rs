use super::{Event, Signals};
use crate::decoding::Worker;
use std::sync::mpsc::{self, SyncSender};
use std::thread::JoinHandle;

pub(super) struct Cleanup {
    sender: Option<SyncSender<(u64, Worker)>>,
    thread: Option<JoinHandle<()>>,
}

impl Cleanup {
    pub(super) fn new(capacity: usize, signals: Signals) -> Result<Self, String> {
        let (sender, receiver) = mpsc::sync_channel::<(u64, Worker)>(capacity);
        let thread = std::thread::Builder::new()
            .name("wallpicker-retire".into())
            .spawn(move || {
                while let Ok((generation, worker)) = receiver.recv() {
                    drop(worker); // cancel, kill and reap off the Wayland loop
                    let _ = signals.0.send(Event::Retired(generation));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            sender: Some(sender),
            thread: Some(thread),
        })
    }

    pub(super) fn retire(&self, generation: u64, worker: Worker) -> Result<(), Worker> {
        self.sender
            .as_ref()
            .unwrap()
            .try_send((generation, worker))
            .map_err(|e| match e {
                mpsc::TrySendError::Full((_, worker)) | mpsc::TrySendError::Disconnected((_, worker)) => worker,
            })
    }
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
