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

mod drawing;
mod gpu;
mod image;
mod layer;
mod output;
mod protocol;
mod screen;
mod state;
mod transition;

#[cfg(test)]
mod gpu_tests;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::Value;
use smithay_client_toolkit::reexports::calloop::channel::{self, Sender};
use wallpicker_core::settings::FitMode;

use image::{Made, map_big_allocations};
use state::setup;
pub use transition::Transition;

/// A file to show and its fit.
pub type Picture = (PathBuf, Option<FitMode>);

/// What a monitor shows in the saved state, as the renderer gets it
/// (`hyprpaper::saved_for`).
pub type Saved = Arc<dyn Fn(&str) -> Option<Picture> + Send + Sync>;

/// How a show went: Ok once the monitor has the picture.
type Reply = mpsc::Sender<Result<(), String>>;

enum Msg {
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
}

impl Renderer {
    /// Connect to the Wayland session and start drawing (each monitor what
    /// `saved` says it shows). An error when there's no session, or it
    /// lacks the layer shell.
    pub fn start(saved: Saved) -> Result<Renderer, String> {
        map_big_allocations();
        let (tx, rx) = channel::channel();
        let loop_tx = tx.clone();
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("renderer".into())
            .spawn(move || {
                let run = || -> Result<(), String> {
                    let (mut event_loop, mut state) = setup(rx, loop_tx, saved)?;
                    let _ = ready_tx.send(Ok(()));
                    while state.running {
                        event_loop
                            .dispatch(None, &mut state)
                            .map_err(|e| e.to_string())?;
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
        picture: Picture,
        transition: Option<Transition>,
    ) -> Result<mpsc::Receiver<Result<(), String>>, String> {
        let (reply, answer) = mpsc::channel();
        self.tx
            .send(Msg::Show {
                monitor,
                picture,
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
