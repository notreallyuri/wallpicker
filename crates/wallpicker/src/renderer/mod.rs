//! wallpicker's own renderer (the wallpicker backend): `wallpicker daemon`
//! draws each monitor's wallpaper on the background layer (wlr-layer-shell)
//! from shared memory, at the output's own pixels (`wp_viewporter` scales
//! the buffer to the surface; without it, the integer buffer scale). Its own
//! thread and event loop (calloop): idle, it sleeps.
//!
//! Pictures are decoded and fitted on a worker thread (compose.rs). The
//! decoded picture is kept a few seconds, for the next monitor (applying to
//! all of them asks for each in turn). A buffer is let go once the
//! compositor releases it (it has its own copy by then), so nothing is held
//! between changes: a new size or scale reads the file again. A monitor that
//! appears shows what the saved state has for it.
//!
//! A show can come in with a transition (transition/): the new picture on
//! a temporary background layer above the old one, stepped on frame callbacks,
//! using compositor opacity for Fade and EGL/GLES shaders for other effects, then put
//! on the layer surface itself and the temporary layer dropped. Without the
//! protocol it needs (`wp_alpha_modifier_v1`), a cut.

mod animation;
mod drawing;
mod fallback;
mod gpu;
mod image;
mod layer;
mod output;
pub mod playback;
mod protocol;
mod screen;
mod state;
mod transition;

#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod gpu_tests;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::Value;
use smithay_client_toolkit::reexports::calloop::channel::{self, Sender};

pub(crate) use animation::Options as PlaybackOptions;
use image::{Made, map_big_allocations};
use state::setup;
pub use transition::Transition;

/// Original source, prepared content, geometry and application intent.
pub type Picture = wallpicker_core::render::Selection;

/// What a monitor shows in the saved state, as the renderer gets it
/// (`wallpapers::saved_for`).
pub type Saved = Arc<dyn Fn(&str) -> Option<Picture> + Send + Sync>;

/// How a show went: Ok once the monitor has the picture.
type Reply = mpsc::Sender<Result<(), String>>;
type PendingShow = (String, mpsc::Receiver<Result<(), String>>);

enum Msg {
    Adjust {
        batch: wallpicker_core::render::Batch,
        reply: mpsc::Sender<Vec<PendingShow>>,
    },
    Animation {
        handoff: Arc<Mutex<Option<crate::preparation::Handoff>>>,
        reply: mpsc::Sender<Result<animation::Admission, String>>,
    },
    Fallback {
        prepared: fallback::Prepared,
        reply: mpsc::Sender<Vec<PendingShow>>,
    },
    Decoded(u64),
    Presented(playback::Ticket, Result<(), String>),
    Pause(bool),
    Interval(Duration),
    Batch(Vec<Msg>),
    Show {
        monitor: String,
        picture: Picture,
        transition: Option<Transition>,
        reply: Reply,
    },
    Status(mpsc::Sender<Value>),
    Stop,
    /// A worker's frame for screen `id`, made for its `generation`; None
    /// when there was nothing to show (no saved picture).
    Made {
        id: u32,
        generation: u64,
        made: Result<Option<Made>, String>,
    },
}

/// The running renderer. Dropped, it stops.
pub struct Renderer {
    tx: Sender<Msg>,
    thread: Option<JoinHandle<()>>,
    animation_enabled: bool,
}

impl Renderer {
    /// Queue the whole batch in one event-loop message. Preparation and protocol
    /// checks happen before any output is changed; replies remain per output.
    pub fn show_batch(&self, prepared: crate::preparation::PreparedBatch) -> Result<Vec<PendingShow>, String> {
        if !prepared.sessions().is_empty() && self.animation_enabled {
            let batch = prepared.batch();
            if batch.intent == wallpicker_core::render::Intent::Adjust {
                return Err("animated adjustments require worker-free PreparedAdjustment preflight".into());
            }
            // Caller keeps ownership through GPU/admission validation. Rejection
            // leaves workers here, so cleanup never joins on the Wayland loop.
            let handoff = Arc::new(Mutex::new(Some(prepared.handoff()?)));
            let (reply, answer) = mpsc::channel();
            self.tx
                .send(Msg::Animation {
                    handoff: handoff.clone(),
                    reply,
                })
                .map_err(|_| "the renderer has stopped".to_string())?;
            let result = answer
                .recv()
                .map_err(|_| "the renderer stopped during admission".to_string())?;
            return match result? {
                animation::Admission::Accepted(answers) => Ok(answers),
                animation::Admission::Fallback(offer) => {
                    let loan = handoff
                        .lock()
                        .unwrap()
                        .take()
                        .ok_or("fallback handoff already consumed")?;
                    // RGB conversion and decoder cancellation/join happen on
                    // this caller, before any fallback output is changed.
                    let prepared = offer.prepare(loan)?;
                    let (reply, answer) = mpsc::channel();
                    self.tx
                        .send(Msg::Fallback { prepared, reply })
                        .map_err(|_| "the renderer has stopped".to_string())?;
                    answer
                        .recv()
                        .map_err(|_| "the renderer stopped during fallback".to_string())
                }
            };
        }
        // Without playback, animated outputs are drawn as their first frame.
        // Dropping any sessions releases workers/reservations on this caller thread.
        let batch = prepared.handoff()?.batch;
        let transition = if batch.intent == wallpicker_core::render::Intent::Adjust {
            None
        } else {
            Transition::from_setting(&batch.transition.kind, batch.transition.duration_ms.min(10_000))
        };
        let mut messages = Vec::new();
        let mut answers = Vec::new();
        for output in &batch.outputs {
            let (reply, answer) = mpsc::channel();
            messages.push(Msg::Show {
                monitor: output.monitor.clone(),
                picture: batch.selection(output.picture.clone()),
                transition,
                reply,
            });
            answers.push((output.monitor.clone(), answer));
        }
        self.tx
            .send(Msg::Batch(messages))
            .map_err(|_| "the renderer has stopped".to_string())?;
        Ok(answers)
    }

    /// Adjust live geometry without creating a decoder or restarting its clock.
    /// Replies acknowledge the first successful swap of each adjusted output.
    pub fn adjust_batch(&self, prepared: crate::preparation::PreparedAdjustment) -> Result<Vec<PendingShow>, String> {
        let batch = prepared.handoff()?;
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(Msg::Adjust { batch, reply })
            .map_err(|_| "the renderer has stopped".to_string())?;
        answer
            .recv()
            .map_err(|_| "the renderer stopped during adjustment".to_string())
    }

    /// Without playback: animations are only ever their first frame.
    #[cfg(test)]
    pub fn start(saved: Saved) -> Result<Renderer, String> {
        Self::start_inner(saved, None)
    }

    /// Connect to the Wayland session and start drawing (each monitor what
    /// `saved` says it shows), able to play prepared animations. Nothing is
    /// allocated for playback until the first one. An error when there's no
    /// session, or it lacks the layer shell.
    pub(crate) fn start_with_playback(saved: Saved, options: PlaybackOptions) -> Result<Renderer, String> {
        Self::start_inner(saved, Some(options))
    }

    /// Hold (or release) every animation's frame, including later ones.
    pub(crate) fn pause(&self, paused: bool) -> Result<(), String> {
        self.tx
            .send(Msg::Pause(paused))
            .map_err(|_| "the renderer has stopped".to_string())
    }

    /// At most one presentation per `interval` on each output (zero: as
    /// often as the display refreshes), for playing animations too.
    pub(crate) fn set_interval(&self, interval: Duration) -> Result<(), String> {
        self.tx
            .send(Msg::Interval(interval))
            .map_err(|_| "the renderer has stopped".to_string())
    }

    /// Decoder notifications for [`crate::preparation::Preparer::prepare`].
    pub(crate) fn decoder_wake(&self) -> Arc<dyn Fn(u64) + Send + Sync> {
        let tx = self.tx.clone();
        Arc::new(move |generation| {
            let _ = tx.send(Msg::Decoded(generation));
        })
    }

    fn start_inner(saved: Saved, options: Option<animation::Options>) -> Result<Renderer, String> {
        map_big_allocations();
        let (tx, rx) = channel::channel();
        let loop_tx = tx.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("renderer".into())
            .spawn(move || {
                let run = || -> Result<(), String> {
                    let (mut event_loop, mut state) = setup(rx, loop_tx, saved, options)?;
                    let _ = ready_tx.send(Ok(()));
                    while state.running {
                        event_loop.dispatch(None, &mut state).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                };
                if let Err(e) = run() {
                    // Either it never started (start() says so) or it stopped.
                    if ready_tx.send(Err(e.clone())).is_err() {
                        eprintln!("wallpicker: the renderer stopped: {e}");
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Renderer {
                tx,
                thread: Some(thread),
                animation_enabled: options.is_some(),
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("the renderer stopped while starting".into()),
        }
    }

    /// Whether it's still drawing (its connection may have gone).
    pub fn running(&self) -> bool {
        self.thread.as_ref().is_some_and(|t| !t.is_finished())
    }

    /// Show `picture` on `monitor`, coming in with `transition`; the answer
    /// comes once it's up (as the transition starts).
    pub fn show(
        &self,
        monitor: String,
        picture: impl Into<Picture>,
        transition: Option<Transition>,
    ) -> Result<mpsc::Receiver<Result<(), String>>, String> {
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(Msg::Show {
                monitor,
                picture: picture.into(),
                transition,
                reply,
            })
            .map_err(|_| "the renderer has stopped".to_string())?;
        Ok(answer)
    }

    /// Each monitor: its name, size, and the file it shows.
    pub fn status(&self) -> Result<Value, String> {
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(Msg::Status(reply))
            .map_err(|_| "the renderer has stopped".to_string())?;
        answer
            .recv_timeout(Duration::from_secs(5))
            .map_err(|_| "the renderer didn't answer".to_string())
    }
}

impl Drop for Renderer {
    /// Its surfaces go with it: what's underneath shows again.
    fn drop(&mut self) {
        let _ = self.tx.send(Msg::Stop);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}
