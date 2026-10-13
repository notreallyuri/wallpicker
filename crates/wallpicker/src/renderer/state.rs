//! Wayland connection, event loop, incoming requests, and worker results.

use super::animation::{Live, Options};
use super::gpu::Device;
use super::image::{Made, RECENT_FOR, Recent, Want, make};
use super::playback::Runtime;
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
    pub(super) gpu_unavailable: Option<String>,
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
    pub(super) tx: Sender<Msg>,
    saved: Saved,
    recent: Recent,
    /// For the timer that forgets [`Recent`].
    pub(super) handle: LoopHandle<'static, State>,
    forget: Option<RegistrationToken>,
    pub(super) running: bool,
    pub(super) animation_options: Option<Options>,
    pub(super) playback: Option<Runtime<State>>,
    pub(super) live: Option<Live>,
    pub(super) fallback_budget: Option<super::fallback::Budget>,
    /// Manual pause, for playback started later too.
    pub(super) paused: bool,
}

pub(super) fn setup(
    rx: Channel<Msg>,
    tx: Sender<Msg>,
    saved: Saved,
    options: Option<Options>,
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
        gpu_unavailable: None,
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        alpha_modifier: globals.bind::<WpAlphaModifierV1, State, Quiet>(&qh, 1..=1, Quiet).ok(),
        compositor,
        layer_shell: LayerShell::bind(&globals, &qh).map_err(|_| missing("layer shell (wlr-layer-shell)"))?,
        shm: Shm::bind(&globals, &qh).map_err(|_| missing("wl_shm"))?,
        viewporter: globals.bind::<WpViewporter, State, Quiet>(&qh, 1..=1, Quiet).ok(),
        qh,
        screens: Vec::new(),
        in_flight: Vec::new(),
        tx,
        saved,
        recent: Arc::default(),
        handle: event_loop.handle(),
        forget: None,
        running: true,
        animation_options: options,
        playback: None,
        live: None,
        fallback_budget: options.map(|o| super::fallback::Budget::new(o.fallback_bytes)),
        paused: false,
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
    pub(super) fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Adjust { batch, reply } => {
                let answers = self.accept_adjustment(batch);
                let _ = reply.send(answers);
            }
            Msg::Animation { handoff, reply } => {
                let result = self.accept_animation(&mut handoff.lock().unwrap());
                let _ = reply.send(result);
            }
            Msg::Fallback { prepared, reply } => {
                let answers = self.accept_fallback(prepared);
                let _ = reply.send(answers);
            }
            Msg::Decoded(generation) => {
                if let Some(runtime) = &self.playback {
                    runtime.signals().decoder_wake()(generation);
                }
            }
            Msg::Presented(ticket, result) => self.animation_presented(ticket, result),
            Msg::Interval(interval) => {
                if let Some(options) = &mut self.animation_options {
                    options.interval = interval;
                }
                if let Some(runtime) = &mut self.playback {
                    runtime.set_interval(interval);
                }
            }
            Msg::Pause(paused) => {
                self.paused = paused;
                if let Some(runtime) = &mut self.playback {
                    runtime.set_manual_pause(paused);
                }
            }
            Msg::Batch(messages) => {
                for message in messages {
                    self.handle(message);
                }
            }
            Msg::Show {
                monitor,
                picture,
                transition,
                reply,
            } => {
                self.show_want(&monitor, Want::File(picture), transition, reply);
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
                            "render_error": s.render_error,
                            "source": s.shown.picture().map(|p| &p.picture.source),
                            "application": s.shown.picture().map(|p| &p.application),
                            "intent": s.shown.picture().map(|p| p.intent),
                            "fading": s.overlay.is_some() || s.incoming.as_ref().is_some_and(|a| a.transitioning()),
                            "shader": s.overlay.as_ref().is_some_and(|o| o.gpu.is_some()) || s.incoming.as_ref().is_some_and(|a| a.transitioning()),
                            "animation_transition": self.animation_transition_status(s.incoming.as_ref()),
                            "file": s.shown.picture().and_then(|p| p.picture.static_file().ok().map(|(path, _)| path)),
                            "playback": self.animation_status(s),
                        })
                    })
                    .collect();
                let _ = reply.send(json!({ "ok": true, "screens": screens,
                    "playback_workers": self.playback.as_ref().map_or(0, Runtime::admitted),
                    "playback_timer": self.playback.as_ref().is_some_and(Runtime::timer_armed),
                    "playback_texture_bytes": self.live.as_ref().map_or(0, |l| l.budget.used()),
                    "representative_bytes": self.fallback_budget.as_ref().map_or(0, |b| b.used()),
                    "playback_error": self.playback.as_ref().and_then(Runtime::error),
                    "playback_available": self.animation_options.is_some(),
                    "playback_paused": self.paused,
                    "playback_interval_ms": self.animation_options.map(|o| o.interval.as_secs_f64() * 1000.0),
                }));
            }
            Msg::Stop => self.running = false,
            Msg::Made { id, generation, made } => self.made(id, generation, made),
        }
    }

    pub(super) fn show_want(
        &mut self,
        monitor: &str,
        want: Want,
        transition: Option<super::Transition>,
        reply: super::Reply,
    ) {
        let Some(i) = self.screens.iter().position(|s| s.name == monitor) else {
            let _ = reply.send(Err(format!("no monitor named {monitor}")));
            return;
        };
        self.cancel_animation(i);
        let screen = &mut self.screens[i];
        screen.answer(Err("superseded by a newer wallpaper request".into()));
        screen.asked = Some((want, transition));
        screen.waiting.push(reply);
        self.redraw(i, true);
    }

    /// Ask a worker for screen `i`'s frame: when `again`, or when its size
    /// in pixels isn't what it shows.
    pub(super) fn redraw(&mut self, i: usize, again: bool) {
        if self.screens[i].animation.is_some() && self.screens[i].asked.is_none() {
            self.resize_animation(i);
            return;
        }
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
            .map_or_else(|| screen.shown.clone(), |(want, _)| want);
        let (id, generation, name) = (screen.id, screen.generation, screen.name.clone());
        let (saved, recent, tx) = (self.saved.clone(), self.recent.clone(), self.tx.clone());
        std::thread::spawn(move || {
            let made = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                make(&name, want, size, &saved, &recent)
            }))
            .unwrap_or_else(|_| Err("drawing it failed".into()));
            let _ = tx.send(Msg::Made { id, generation, made });
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
                let fallback = matches!(screen.shown, Want::Still(..))
                    || screen
                        .asked
                        .as_ref()
                        .is_some_and(|(want, _)| matches!(want, Want::Still(..)));
                // Only a failed new picture leaves something else to draw. When
                // what it shows failed itself, it would fail again at once.
                let asked = screen.asked.take().is_some();
                screen.render_error = Some(e.clone());
                screen.answer(Err(e));
                // A permanent fitting/budget failure must not spin up another
                // worker repeatedly. Keep the last buffer until a new request
                // or output change provides a reason to try again.
                if fallback || !asked {
                    return;
                }
                return self.redraw(i, false);
            }
        };
        // The show's transition (not a redraw's, nor at a new size).
        let transition = self.screens[i].asked.as_ref().and_then(|(_, t)| *t);
        self.suspend_animation(i, true);
        match self.draw(i, &made, transition) {
            Ok(true) => {
                self.remove_animation(i);
                let screen = &mut self.screens[i];
                screen.asked = None;
                screen.render_error = None;
                screen.shown = match made.still {
                    Some(still) => Want::Still(made.picture, still),
                    None => Want::File(made.picture),
                };
                screen.drawn = Some(made.size);
                screen.answer(Ok(()));
            }
            // Made for a size it no longer has: again, at this one.
            Ok(false) => {
                self.suspend_animation(i, false);
                self.redraw(i, true);
            }
            Err(e) => {
                self.suspend_animation(i, false);
                self.screens[i].asked = None;
                self.screens[i].render_error = Some(e.clone());
                self.screens[i].answer(Err(e));
            }
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
                state.recent.lock().unwrap_or_else(|e| e.into_inner()).take();
                state.forget = None;
                TimeoutAction::Drop
            })
            .ok();
    }
}
