//! Wayland connection, event loop, incoming requests, and worker results.

use super::gpu::Device;
use super::image::{Made, RECENT_FOR, Recent, Want, make};
use super::protocol::Quiet;
use super::screen::Screen;
use super::{Msg, Saved};
use serde_json::{Value, json};
use smithay_client_toolkit::compositor::CompositorState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::reexports::calloop::channel::{self, Channel, Sender};
use smithay_client_toolkit::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay_client_toolkit::reexports::calloop::{EventLoop, LoopHandle, RegistrationToken};
use smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::wl_buffer;
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_v1::WpAlphaModifierV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewporter::WpViewporter;
use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use smithay_client_toolkit::shm::{Shm, raw::RawPool};
use std::rc::Rc;
use std::sync::Arc;

pub(super) struct State {
    pub(super) connection: Connection,
    pub(super) gpu: Option<Rc<Device>>,
    pub(super) gpu_unavailable: bool,
    pub(super) registry: RegistryState,
    pub(super) outputs: OutputState,
    pub(super) compositor: CompositorState,
    pub(super) layer_shell: LayerShell,
    pub(super) shm: Shm,
    pub(super) viewporter: Option<WpViewporter>,
    /// For transitions: without them, a cut.
    pub(super) alpha_modifier: Option<WpAlphaModifierV1>,
    pub(super) qh: QueueHandle<State>,
    pub(super) screens: Vec<Screen>,
    /// Buffers the compositor hasn't released yet, with their memory.
    pub(super) in_flight: Vec<(wl_buffer::WlBuffer, RawPool)>,
    /// For the workers' frames.
    tx: Sender<Msg>,
    saved: Saved,
    recent: Recent,
    /// For the timer that forgets [`Recent`].
    handle: LoopHandle<'static, State>,
    forget: Option<RegistrationToken>,
    pub(super) running: bool,
}

pub(super) fn setup(
    rx: Channel<Msg>,
    tx: Sender<Msg>,
    saved: Saved,
) -> Result<(EventLoop<'static, State>, State), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
    let (globals, queue) = registry_queue_init::<State>(&conn).map_err(|e| e.to_string())?;
    let qh = queue.handle();
    let missing = |what: &str| format!("the compositor has no {what}");
    let event_loop = EventLoop::<State>::try_new().map_err(|e| e.to_string())?;
    let compositor = CompositorState::bind(&globals, &qh).map_err(|_| missing("wl_compositor"))?;
    let state = State {
        connection: conn.clone(),
        gpu: None,
        gpu_unavailable: false,
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        alpha_modifier: globals
            .bind::<WpAlphaModifierV1, State, Quiet>(&qh, 1..=1, Quiet)
            .ok(),
        compositor,
        layer_shell: LayerShell::bind(&globals, &qh)
            .map_err(|_| missing("layer shell (wlr-layer-shell)"))?,
        shm: Shm::bind(&globals, &qh).map_err(|_| missing("wl_shm"))?,
        viewporter: globals
            .bind::<WpViewporter, State, Quiet>(&qh, 1..=1, Quiet)
            .ok(),
        qh,
        screens: Vec::new(),
        in_flight: Vec::new(),
        tx,
        saved,
        recent: Arc::default(),
        handle: event_loop.handle(),
        forget: None,
        running: true,
    };
    WaylandSource::new(conn, queue)
        .insert(event_loop.handle())
        .map_err(|e| e.error.to_string())?;
    event_loop
        .handle()
        .insert_source(rx, |event, _, state: &mut State| match event {
            channel::Event::Msg(msg) => state.handle(msg),
            channel::Event::Closed => state.running = false,
        })
        .map_err(|e| e.error.to_string())?;
    Ok((event_loop, state))
}

impl State {
    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Show {
                monitor,
                picture,
                transition,
                reply,
            } => {
                let Some(i) = self.screens.iter().position(|s| s.name == monitor) else {
                    let _ = reply.send(Err(format!("no monitor named {monitor}")));
                    return;
                };
                let screen = &mut self.screens[i];
                screen.asked = Some((picture, transition));
                screen.waiting.push(reply);
                self.redraw(i, true);
            }
            Msg::Status(reply) => {
                let screens: Vec<Value> = self
                    .screens
                    .iter()
                    .map(|s| {
                        json!({
                            "monitor": s.name,
                            "size": s.size,
                            "pixels": s.drawn,
                            "fading": s.overlay.is_some(),
                            "shader": s.overlay.as_ref().is_some_and(|o| o.gpu.is_some()),
                            "file": match &s.shown { Want::File((p, _)) => Some(p), Want::Saved => None },
                        })
                    })
                    .collect();
                let _ = reply.send(json!({ "ok": true, "screens": screens }));
            }
            Msg::Stop => self.running = false,
            Msg::Made {
                id,
                generation,
                made,
            } => self.made(id, generation, made),
        }
    }

    /// Ask a worker for screen `i`'s frame: when `again`, or when its size
    /// in pixels isn't what it shows.
    pub(super) fn redraw(&mut self, i: usize, again: bool) {
        let Some((size, _)) = self.target(&self.screens[i]) else {
            return; // Not configured yet: that redraws.
        };
        let screen = &mut self.screens[i];
        if !again && screen.drawn == Some(size) {
            return;
        }
        screen.generation += 1;
        let want = screen
            .asked
            .clone()
            .map_or_else(|| screen.shown.clone(), |(p, _)| Want::File(p));
        let (id, generation, name) = (screen.id, screen.generation, screen.name.clone());
        let (saved, recent, tx) = (self.saved.clone(), self.recent.clone(), self.tx.clone());
        std::thread::spawn(move || {
            let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                make(&name, want, size, &saved, &recent)
            }))
            .unwrap_or_else(|_| Err("drawing it failed".into()));
            let _ = tx.send(Msg::Made {
                id,
                generation,
                made,
            });
        });
    }

    fn made(&mut self, id: u32, generation: u64, made: Result<Option<Made>, String>) {
        let Some(i) = self
            .screens
            .iter()
            .position(|s| s.id == id && s.generation == generation)
        else {
            return; // Gone, or something newer is on its way.
        };
        self.forget_recent_later();
        let made = match made {
            Ok(Some(made)) => made,
            Ok(None) => return self.screens[i].answer(Ok(())),
            Err(e) => {
                eprintln!("wallpicker: {}: {e}", self.screens[i].name);
                let screen = &mut self.screens[i];
                // What it showed stays; at its size, if that changed meanwhile.
                screen.asked = None;
                screen.answer(Err(e));
                return self.redraw(i, false);
            }
        };
        // The show's transition (not a redraw's, nor at a new size).
        let transition = self.screens[i].asked.as_ref().and_then(|(_, t)| *t);
        match self.draw(i, &made, transition) {
            Ok(true) => {
                let screen = &mut self.screens[i];
                screen.asked = None;
                screen.shown = Want::File(made.picture);
                screen.drawn = Some(made.size);
                screen.answer(Ok(()));
            }
            // Made for a size it no longer has: again, at this one.
            Ok(false) => self.redraw(i, true),
            Err(e) => self.screens[i].answer(Err(e)),
        }
    }

    /// The decoded picture dropped [`RECENT_FOR`] after the last frame.
    fn forget_recent_later(&mut self) {
        if let Some(t) = self.forget.take() {
            self.handle.remove(t);
        }
        let timer = Timer::from_duration(RECENT_FOR);
        self.forget = self
            .handle
            .insert_source(timer, |_, _, state| {
                state
                    .recent
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();
                state.forget = None;
                TimeoutAction::Drop
            })
            .ok();
    }
}
