//! Calloop adapter. The host owns surfaces and implements the draw/swap hook;
//! this module owns source clocks, output gates, notifications and retirement.
mod cleanup;

use super::{Decision, OutputPacer, PauseReason, SourceSession, Target, Ticket, Visibility, visibility};
use crate::decoding::Worker;
use crate::preparation::Session;
use cleanup::Cleanup;
use smithay_client_toolkit::reexports::calloop::channel::{self, Sender};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{LoopHandle, RegistrationToken};
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub trait Host: Sized + 'static {
    fn playback(&mut self) -> &mut Runtime<Self>;
    /// Draw the source's current frame, request a callback tagged with `ticket`,
    /// and swap. This synchronous hook must not retain CPU frame history. On
    /// failure the output freezes; other outputs using the source can continue.
    fn present(&mut self, ticket: Ticket) -> Result<(), String>;
}

enum Event {
    Changed,
    Decoded(u64),
    Callback(Ticket),
    Deadline(u64),
    Retired(u64),
}

/// Cheap, thread-safe notifications. Frames remain in the worker's bounded queue.
#[derive(Clone)]
pub struct Signals(Sender<Event>);

impl Signals {
    pub fn decoder_wake(&self) -> Arc<dyn Fn(u64) + Send + Sync> {
        let signals = self.clone();
        Arc::new(move |generation| {
            let _ = signals.0.send(Event::Decoded(generation));
        })
    }

    pub fn frame_callback(&self, ticket: Ticket) {
        let _ = self.0.send(Event::Callback(ticket));
    }
}

struct Output {
    pacer: OutputPacer,
    initial: bool,
    error: Option<String>,
    suspended: bool,
}

/// Construct lazily for animated ownership, using the shared decoder admission
/// limit as `capacity`. No driver or cleanup thread is needed by a static host.
/// Drop after leaving the event loop: shutdown joins outstanding worker cleanup.
pub struct Runtime<H: Host> {
    handle: LoopHandle<'static, H>,
    epoch: Instant,
    signals: Signals,
    channel: RegistrationToken,
    timer: Option<(Duration, u64, RegistrationToken)>,
    timer_serial: u64,
    sources: BTreeMap<u64, SourceSession>,
    outputs: BTreeMap<Target, Output>,
    // Count unfinished retirements against admission too. Some holds a worker
    // only if cleanup failed; it must never be dropped on a live dispatch path.
    retiring: BTreeMap<u64, Option<Worker>>,
    cleanup: Cleanup,
    capacity: usize,
    error: Option<String>,
    paused: bool,
}

impl<H: Host> Runtime<H> {
    pub fn new(handle: LoopHandle<'static, H>, capacity: NonZeroUsize) -> Result<Self, String> {
        let (tx, rx) = channel::channel();
        let signals = Signals(tx);
        let cleanup = Cleanup::new(capacity.get(), signals.clone())?;
        let channel = handle
            .insert_source(rx, |event, _, host: &mut H| {
                if let channel::Event::Msg(event) = event {
                    Self::dispatch(host, event);
                }
            })
            .map_err(|e| e.error.to_string())?;
        Ok(Self {
            handle,
            epoch: Instant::now(),
            signals,
            channel,
            timer: None,
            timer_serial: 0,
            sources: BTreeMap::new(),
            outputs: BTreeMap::new(),
            retiring: BTreeMap::new(),
            cleanup,
            capacity: capacity.get(),
            error: None,
            paused: false,
        })
    }

    pub fn signals(&self) -> Signals {
        self.signals.clone()
    }
    pub fn source(&self, generation: u64) -> Option<&SourceSession> {
        self.sources.get(&generation)
    }
    pub fn output_error(&self, target: Target) -> Option<&str> {
        self.outputs.get(&target).and_then(|o| o.error.as_deref())
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn admitted(&self) -> usize {
        self.sources.len() + self.retiring.len()
    }
    pub fn available(&self) -> usize {
        if self.error.is_some() {
            0
        } else {
            self.capacity - self.admitted()
        }
    }

    /// Freeze one output during replacement without retiring its source or
    /// holding up other outputs. Resuming preserves its pending callback gate.
    pub fn suspend(&mut self, target: Target, suspended: bool) {
        if let Some(output) = self.outputs.get_mut(&target) {
            output.suspended = suspended;
            self.visibility(target.session);
            self.changed();
        }
    }

    fn visibility(&mut self, generation: u64) {
        let visible = self
            .outputs
            .iter()
            .any(|(t, o)| t.session == generation && o.error.is_none() && !o.suspended);
        if let Some(source) = self.sources.get_mut(&generation) {
            source.set_paused(PauseReason::NoVisibleOutputs, !visible, self.epoch.elapsed());
        }
    }
    pub fn timer_armed(&self) -> bool {
        self.timer.is_some()
    }

    /// Reject without consuming ownership if full or failed. Caller must return
    /// a rejected preparation to its off-loop owner; never drop it in dispatch.
    pub fn insert(&mut self, session: Session) -> Result<u64, Box<Session>> {
        let generation = session.generation();
        if self.admitted() >= self.capacity || self.error.is_some() || self.sources.contains_key(&generation) {
            return Err(Box::new(session));
        }
        let now = self.epoch.elapsed();
        let mut source = SourceSession::new(session, now);
        source.set_paused(PauseReason::NoVisibleOutputs, true, now);
        source.set_paused(PauseReason::Manual, self.paused, now);
        self.sources.insert(generation, source);
        Ok(generation)
    }

    /// `output` is a fresh global output generation for this attachment. The
    /// initial frozen frame is drawn even while the transition pause is held.
    pub fn attach(&mut self, generation: u64, output: u64, interval: Duration) -> Result<Target, String> {
        let target = Target {
            session: generation,
            output,
        };
        if self.outputs.contains_key(&target) {
            return Err("playback output generation is already attached".into());
        }
        let source = self.sources.get_mut(&generation).ok_or("unknown playback source")?;
        source.set_paused(PauseReason::NoVisibleOutputs, false, self.epoch.elapsed());
        self.outputs.insert(
            target,
            Output {
                pacer: OutputPacer::new(target, interval),
                initial: true,
                error: None,
                suspended: false,
            },
        );
        self.changed();
        Ok(target)
    }

    /// A new presentation cap for every attached output. Later attachments
    /// take theirs from the caller.
    pub fn set_interval(&mut self, interval: Duration) {
        for output in self.outputs.values_mut() {
            output.pacer.set_interval(interval);
        }
        self.changed();
    }

    /// Manual pause of every source, including ones admitted later.
    pub fn set_manual_pause(&mut self, paused: bool) {
        self.paused = paused;
        let now = self.epoch.elapsed();
        for source in self.sources.values_mut() {
            source.set_paused(PauseReason::Manual, paused, now);
        }
        self.changed();
    }
    pub fn manual_pause(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, generation: u64, reason: PauseReason, paused: bool) {
        if let Some(source) = self.sources.get_mut(&generation) {
            source.set_paused(reason, paused, self.epoch.elapsed());
            self.changed();
        }
    }

    /// Last consumer removal transfers the worker to bounded off-loop cleanup.
    /// The admission slot is released only after the completion notification.
    pub fn detach(&mut self, target: Target) {
        self.outputs.remove(&target);
        if !self.outputs.keys().any(|t| t.session == target.session) {
            self.retire(target.session);
        }
        self.changed();
    }

    /// A host with unconfigured incoming consumers owns the last-consumer
    /// decision. Remove this output but keep the source for that host's barrier.
    pub fn unbind(&mut self, target: Target) {
        self.outputs.remove(&target);
        self.visibility(target.session);
        self.changed();
    }

    /// Also handles admitted preparations that never gained an output.
    pub fn retire(&mut self, generation: u64) {
        self.outputs.retain(|target, _| target.session != generation);
        if let Some(source) = self.sources.remove(&generation) {
            let stranded = self.cleanup.retire(generation, source.into_worker()).err();
            if stranded.is_some() {
                self.error = Some("playback cleanup is unavailable; admission stopped".into());
            }
            self.retiring.insert(generation, stranded);
        }
        self.changed();
    }

    fn changed(&self) {
        let _ = self.signals.0.send(Event::Changed);
    }

    fn dispatch(host: &mut H, event: Event) {
        let runtime = host.playback();
        let now = runtime.epoch.elapsed();
        match event {
            Event::Decoded(generation) => {
                let Some(source) = runtime.sources.get_mut(&generation) else {
                    return;
                };
                source.advance(now, generation);
            }
            Event::Deadline(serial) => {
                if runtime.timer.as_ref().map(|(_, n, _)| *n) != Some(serial) {
                    return;
                }
                runtime.timer = None;
                for (generation, source) in &mut runtime.sources {
                    source.advance(now, *generation);
                }
            }
            Event::Callback(ticket) => {
                let Some(output) = runtime.outputs.get_mut(&ticket.target()) else {
                    return;
                };
                if !output.pacer.frame_callback(ticket) {
                    return;
                }
            }
            Event::Retired(generation) => {
                runtime.retiring.remove(&generation);
            }
            Event::Changed => {}
        }
        let plans: Vec<_> = runtime
            .outputs
            .iter()
            .filter_map(|(target, output)| {
                if output.error.is_some() || output.suspended {
                    return None;
                }
                let source = runtime.sources.get(&target.session)?;
                match output.pacer.plan(
                    now,
                    target.session,
                    source.current().sequence,
                    output.initial || source.running(),
                ) {
                    Decision::Present(ticket) => Some(ticket),
                    _ => None,
                }
            })
            .collect();
        for ticket in plans {
            let result = host.present(ticket);
            let runtime = host.playback();
            let now = runtime.epoch.elapsed();
            if let Some(output) = runtime.outputs.get_mut(&ticket.target()) {
                match result {
                    Ok(()) => {
                        if output.pacer.presented(ticket, now) {
                            output.initial = false;
                        } else {
                            output.error = Some("playback target changed during presentation".into());
                        }
                    }
                    Err(error) => output.error = Some(error),
                }
            }
        }
        let runtime = host.playback();
        let now = runtime.epoch.elapsed();
        for (generation, source) in &mut runtime.sources {
            if !Self::eligible(&runtime.outputs, *generation).any(|_| true) {
                source.set_paused(PauseReason::NoVisibleOutputs, true, now);
            }
            // Hidden, it decodes nothing; shown again, it continues from
            // where it stopped (no catch-up).
            let waiting = Self::eligible(&runtime.outputs, *generation).map(|o| o.pacer.waiting_since());
            let hidden = visibility(waiting, now) == Visibility::Hidden;
            if source.paused_by(PauseReason::Hidden) != hidden {
                source.set_paused(PauseReason::Hidden, hidden, now);
            }
        }
        runtime.arm();
    }

    /// The outputs a source can present to.
    fn eligible(outputs: &BTreeMap<Target, Output>, generation: u64) -> impl Iterator<Item = &Output> {
        outputs
            .iter()
            .filter(move |(t, o)| t.session == generation && o.error.is_none() && !o.suspended)
            .map(|(_, o)| o)
    }

    fn arm(&mut self) {
        let now = self.epoch.elapsed();
        let source_deadlines = self.sources.values().filter_map(SourceSession::next_deadline);
        let output_deadlines = self.outputs.iter().filter_map(|(target, output)| {
            if output.error.is_some() || output.suspended {
                return None;
            }
            let source = self.sources.get(&target.session)?;
            match output.pacer.plan(
                now,
                target.session,
                source.current().sequence,
                output.initial || source.running(),
            ) {
                Decision::WakeAt(time) => Some(time),
                // Another output's draw may have crossed this cap deadline
                // since plans were collected. Arrange a fresh dispatch now.
                Decision::Present(_) => Some(now),
                _ => None,
            }
        });
        let hidden_deadlines = self.sources.keys().filter_map(|generation| {
            match visibility(Self::eligible(&self.outputs, *generation).map(|o| o.pacer.waiting_since()), now) {
                Visibility::HiddenAt(at) => Some(at),
                _ => None,
            }
        });
        let next = source_deadlines.chain(output_deadlines).chain(hidden_deadlines).min();
        if self.timer.as_ref().map(|(at, _, _)| *at) == next {
            return;
        }
        if let Some((_, _, token)) = self.timer.take() {
            self.handle.remove(token);
        }
        let Some((at, deadline)) = next.and_then(|at| self.epoch.checked_add(at).map(|deadline| (at, deadline))) else {
            return;
        };
        let Some(serial) = self.timer_serial.checked_add(1) else {
            self.error = Some("playback timer generation exhausted".into());
            return;
        };
        self.timer_serial = serial;
        match self
            .handle
            .insert_source(Timer::from_deadline(deadline), move |_, _, host| {
                Self::dispatch(host, Event::Deadline(serial));
                TimeoutAction::Drop
            }) {
            Ok(token) => self.timer = Some((at, serial, token)),
            Err(error) => self.error = Some(format!("playback timer unavailable: {}", error.error)),
        }
    }
}

impl<H: Host> Drop for Runtime<H> {
    fn drop(&mut self) {
        self.handle.remove(self.channel);
        if let Some((_, _, token)) = self.timer.take() {
            self.handle.remove(token);
        }
        // Shutdown only, after dispatch has ended. Active and stranded workers
        // are joined by field drops; Cleanup drains and joins its bounded queue.
    }
}
