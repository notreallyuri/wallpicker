//! Isolated, pull-driven animation decoding. No worker exists in static sessions.
//!
//! RLIMIT_AS covers the child's entire address space (including codec scratch).
//! Parent admission reserves that limit, three source RGBA frames, and 1 MiB for
//! pump stack/control data. This is not a whole-daemon RSS or GPU-memory limit.
//! Two frames can queue; all receiving/queued/caller-held frames share three
//! slots. Retaining them applies backpressure instead of growing memory.
mod budget;
mod child;
mod process;
mod protocol;
mod slots;
#[cfg(test)]
mod tests;

pub use budget::{Budget, Usage};
use std::io::{self, Write};
use std::net::Shutdown;
use std::os::fd::OwnedFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use wallpicker_core::media::{Frame, Source};

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Hard bound including executable/libraries/stack and all codec allocations:
    /// this, plus `frame_allowance` times the source's RGBA frame size.
    pub address_space_bytes: u64,
    /// A codec's canvases grow with the picture: a GIF decoder holds a canvas,
    /// the previous one (for disposal), a patch and the frame it returns.
    pub frame_allowance: u64,
    /// Total response deadline per source frame, including startup on frame one.
    pub frame_timeout: Duration,
}

pub struct DecodedFrame {
    pub generation: u64,
    pub sequence: u64,
    frame: Frame,
    _slot: slots::Slot,
}

impl DecodedFrame {
    pub fn frame(&self) -> &Frame {
        &self.frame
    }
}

pub struct Worker {
    process: Arc<process::Process>,
    socket: UnixStream,
    slots: Arc<slots::Slots>,
    receiver: Option<mpsc::Receiver<Result<DecodedFrame, String>>>,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    /// `executable` must be this wallpicker build (production: current_exe).
    /// The wake callback runs on the pump thread: it should only notify the
    /// event loop. Callers must reject results from superseded generations.
    pub fn start(
        executable: &Path,
        source: Source,
        generation: u64,
        budget: &Budget,
        options: Options,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<Self> {
        Self::launch(executable, source, generation, budget, options, wake, false)
    }

    /// Decode exactly the first frame and stop before requesting the second.
    /// This blocks during preparation, never on the renderer event loop. The
    /// returned worker remains paused until its owner explicitly resumes it.
    pub fn prepare(
        executable: &Path,
        source: Source,
        generation: u64,
        budget: &Budget,
        options: Options,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<(Self, DecodedFrame)> {
        let worker = Self::launch(executable, source, generation, budget, options, wake, true)?;
        let first = worker
            .recv_timeout(options.frame_timeout)
            .map_err(io::Error::other)?
            .map_err(io::Error::other)?;
        Ok((worker, first))
    }

    fn launch(
        executable: &Path,
        source: Source,
        generation: u64,
        budget: &Budget,
        options: Options,
        wake: Arc<dyn Fn() + Send + Sync>,
        hold_first: bool,
    ) -> io::Result<Self> {
        if options.address_space_bytes == 0
            || options.address_space_bytes == libc::RLIM_INFINITY
            || options.frame_timeout.is_zero()
            || options.frame_timeout > Duration::from_secs(60)
        {
            return Err(io::Error::other("invalid decoder worker limits"));
        }
        let frame = frame_bytes(&source)? as u64;
        let address_space = frame
            .checked_mul(options.frame_allowance)
            .and_then(|n| n.checked_add(options.address_space_bytes))
            .filter(|n| *n != libc::RLIM_INFINITY)
            .ok_or_else(|| io::Error::other("invalid decoder worker limits"))?;
        let bytes = frame
            .checked_mul(3)
            .and_then(|n| n.checked_add(address_space))
            .and_then(|n| n.checked_add(1024 * 1024))
            .ok_or_else(|| io::Error::other("decoder reservation overflow"))?;
        let reservation = budget.reserve(bytes)?;
        let request = protocol::request(&source)?;
        let (mut socket, child_socket) = UnixStream::pair()?;
        socket.set_write_timeout(Some(options.frame_timeout))?;
        let mut command = Command::new(executable);
        command
            .arg("__decode")
            .stdin(Stdio::from(OwnedFd::from(child_socket.try_clone()?)))
            .stdout(Stdio::from(OwnedFd::from(child_socket)))
            .stderr(Stdio::null());
        process::constrain(&mut command, address_space);
        let process = Arc::new(process::Process::default());
        let slots = Arc::new(slots::Slots::default());
        let (sender, receiver) = mpsc::sync_channel(2);
        let mut worker = Self {
            process: process.clone(),
            socket: socket.try_clone()?,
            slots: slots.clone(),
            receiver: Some(receiver),
            thread: None,
        };
        worker.thread = Some(
            std::thread::Builder::new()
                .name("wallpicker-decode".into())
                .stack_size(256 * 1024)
                .spawn(move || {
                    let _reaper = process::Reaper(process.clone());
                    let result = (|| -> io::Result<()> {
                        // Spawn on this long-lived thread: Linux PDEATHSIG observes
                        // the creating thread's lifetime, not just the parent's PID.
                        process.start(&mut command)?;
                        drop(command);
                        socket.write_all(&request)?;
                        let mut sequence = 0;
                        while let Some(slot) = slots.acquire(&reservation) {
                            let deadline = Instant::now() + options.frame_timeout;
                            socket.write_all(&[protocol::NEXT])?;
                            let frame = protocol::read_frame(&mut socket, &source, deadline)?;
                            let decoded = DecodedFrame {
                                generation,
                                sequence,
                                frame,
                                _slot: slot,
                            };
                            // Pause before publishing: the preparer can receive
                            // and resume immediately without racing this gate.
                            if hold_first && sequence == 0 {
                                slots.pause(true);
                            }
                            if sender.send(Ok(decoded)).is_err() {
                                break;
                            }
                            wake();
                            sequence = sequence
                                .checked_add(1)
                                .ok_or_else(|| io::Error::other("frame sequence overflow"))?;
                        }
                        Ok(())
                    })();
                    let status = process.stop();
                    if let Err(error) = result {
                        let detail = status.map_or(String::new(), |s| format!(" ({s})"));
                        if sender.send(Err(format!("decoder worker: {error}{detail}"))).is_ok() {
                            wake();
                        }
                    }
                })?,
        );
        Ok(worker)
    }

    pub fn process_id(&self) -> Option<u32> {
        self.process.id()
    }

    pub fn try_next(&mut self) -> Option<Result<DecodedFrame, String>> {
        match self.receiver.as_ref()?.try_recv() {
            Ok(frame) => Some(frame),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.receiver = None;
                None
            }
        }
    }

    /// True after a nonblocking read observes the end of the queue (or cancel).
    /// Lets presentation distinguish starvation from an unexpectedly lost pump.
    pub fn is_closed(&self) -> bool {
        self.receiver.is_none()
    }

    /// Blocking consumer API for preparation/qualification, never the renderer loop.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<Result<DecodedFrame, String>, mpsc::RecvTimeoutError> {
        self.receiver
            .as_ref()
            .ok_or(mpsc::RecvTimeoutError::Disconnected)?
            .recv_timeout(timeout)
    }

    /// An already-started frame may finish. No further decode begins while paused.
    pub fn pause(&self, paused: bool) {
        self.slots.pause(paused);
    }

    /// Disconnect queue waiters, interrupt socket reads, kill and reap before
    /// returning. No detached decoder can accumulate during rapid replacement.
    pub fn cancel(&mut self) {
        self.receiver = None;
        self.slots.cancel();
        let _ = self.socket.shutdown(Shutdown::Both);
        self.process.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub(super) fn frame_bytes(source: &Source) -> io::Result<usize> {
    let n = u64::from(source.info().width)
        .checked_mul(u64::from(source.info().height))
        .and_then(|n| n.checked_mul(4))
        .filter(|n| *n > 0 && *n <= source.limits().frame_bytes)
        .ok_or_else(|| io::Error::other("decoder frame byte limit exceeded"))?;
    usize::try_from(n).map_err(io::Error::other)
}

/// Private CLI entry point, not a user-facing command.
#[doc(hidden)]
pub fn child_main() -> i32 {
    child::run()
}
