//! Animated playback and handoff on persistent Wayland/EGL surfaces. The daemon
//! starts the renderer with these options; whether a request plays is its policy.
mod transition;
use super::gpu::{
    Device,
    playback::{Budget, Painter, Surface, Texture},
};
use super::image::Want;
use super::output::mode_pixels;
use super::playback::{Host, PauseReason, Runtime, Signals, Target, Ticket};
use super::protocol::Quiet;
use super::screen::{Place, Screen};
use super::state::State;
use super::{Msg, PendingShow, Picture, Transition};
use crate::preparation::Handoff;
use serde_json::{Value, json};
use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::reexports::calloop::{
    RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle, protocol::wl_callback};
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::shell::{
    WaylandSurface,
    wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerSurface},
};
use std::{collections::BTreeMap, num::NonZeroUsize, rc::Rc, sync::mpsc, time::Duration};
use wallpicker_core::render::{Batch, Content, Geometry, Intent};

/// Explicit resource policy; the daemon's values are provisional until measured.
#[derive(Clone, Copy)]
pub(crate) struct Options {
    pub capacity: NonZeroUsize,
    pub texture_bytes: u64,
    pub fallback_bytes: u64,
    pub interval: Duration,
    pub configure_timeout: Duration,
}

pub(super) enum Admission {
    Accepted(Vec<PendingShow>),
    Fallback(super::fallback::Offer),
}

pub(super) struct Live {
    device: Rc<Device>,
    painter: Rc<Painter>,
    pub budget: Budget,
    textures: BTreeMap<u64, Rc<Texture>>,
    next_output: u64,
    staging: bool,
}

impl Live {
    fn target(&mut self, session: u64) -> Result<Target, String> {
        self.next_output = self
            .next_output
            .checked_add(1)
            .ok_or("playback output generations exhausted")?;
        Ok(Target {
            session,
            output: self.next_output,
        })
    }
}

pub(super) struct Active {
    surface: Surface,
    target: Target,
    texture: Rc<Texture>,
    geometry: Geometry,
    size: (u32, u32),
    presentations: u64,
    error: Option<String>,
}

/// A separate layer keeps the previous wallpaper intact until the first swap.
/// It becomes the screen's persistent layer after acceptance, without copying.
pub(super) struct Incoming {
    active: Option<Active>,
    target: Target,
    texture: Rc<Texture>,
    picture: Picture,
    pub layer: LayerSurface,
    viewport: Option<WpViewport>,
    logical: (u32, u32),
    timer: Option<RegistrationToken>,
    notified: bool,
    reveal: Option<transition::Reveal>,
}

impl Incoming {
    pub(super) fn transitioning(&self) -> bool {
        self.reveal.is_some()
    }
}

impl Drop for Incoming {
    fn drop(&mut self) {
        self.active.take(); // EGL/native window before layer or viewport destruction.
        if let Some(viewport) = self.viewport.take() {
            viewport.destroy();
        }
    }
}

struct Callback {
    signals: Signals,
    ticket: Ticket,
}
impl Dispatch2<wl_callback::WlCallback, State> for Callback {
    fn event(
        &self,
        _: &mut State,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            self.signals.frame_callback(self.ticket);
        }
    }
}

impl Host for State {
    fn playback(&mut self) -> &mut Runtime<Self> {
        self.playback.as_mut().expect("installed playback host")
    }

    fn present(&mut self, ticket: Ticket) -> Result<(), String> {
        let runtime = self.playback.as_ref().ok_or("playback host unavailable")?;
        let source = runtime
            .source(ticket.target().session)
            .ok_or("obsolete playback source")?;
        let frame = source.current();
        let signals = runtime.signals();
        let qh = &self.qh;
        for screen in &mut self.screens {
            let incoming = screen.incoming.as_mut().filter(|a| a.target == ticket.target());
            let (active, layer, notify) = if let Some(incoming) = incoming {
                (incoming.active.as_mut(), &incoming.layer, Some(&mut incoming.notified))
            } else {
                (
                    screen.animation.as_mut().filter(|a| a.target == ticket.target()),
                    &screen.layer,
                    None,
                )
            };
            let Some(active) = active else { continue };
            let result = (|| {
                if frame.sequence != ticket.sequence() {
                    return Err("obsolete source presentation".into());
                }
                active.texture.upload(
                    frame.generation,
                    frame.sequence,
                    frame.frame().pixels.dimensions(),
                    frame.frame().pixels.as_raw(),
                )?;
                active.surface.draw(&active.texture, active.geometry, || {
                    layer.wl_surface().frame(qh, Callback { signals, ticket });
                })?;
                active.presentations = active.presentations.saturating_add(1);
                Ok(())
            })();
            if let Some(notified) = notify.filter(|n| !**n) {
                *notified = true;
                let _ = self.tx.send(Msg::Presented(ticket, result.clone()));
            }
            return result;
        }
        Err("obsolete playback output".into())
    }
}

impl State {
    pub(super) fn accept_adjustment(&mut self, batch: Batch) -> Vec<PendingShow> {
        let mut answers = Vec::new();
        for output in &batch.outputs {
            let (reply, answer) = mpsc::channel();
            answers.push((output.monitor.clone(), answer));
            let mut picture = batch.selection(output.picture.clone());
            // Without playback, or where nothing plays (it was applied as a still),
            // an animated output is a first-frame still like any other.
            let playing = self.screens.iter().find(|s| s.name == output.monitor).is_some_and(|s| {
                s.animation.is_some() || s.incoming.is_some() || matches!(s.shown, Want::Still(..))
            });
            let (Content::Animated { identity }, Some(_), true) =
                (&output.picture.content, self.animation_options, playing)
            else {
                self.handle(Msg::Show {
                    monitor: output.monitor.clone(),
                    picture,
                    transition: None,
                    reply,
                });
                continue;
            };
            if let Some(screen) = self.screens.iter().find(|s| s.name == output.monitor)
                && let Want::Still(current, still) = &screen.shown
            {
                if screen.asked.is_some() || screen.incoming.is_some() {
                    let _ = reply.send(Err("wallpaper replacement is pending; retry the adjustment".into()));
                } else if current.picture.content != output.picture.content {
                    let _ = reply.send(Err("adjustment source does not match the representative still".into()));
                } else {
                    picture.application = current.application.clone();
                    self.show_want(&output.monitor, Want::Still(picture, still.clone()), None, reply);
                }
                continue;
            }
            // Validate against the currently displayed session, not path equality
            // or the new request's application ID. A pending Apply/Restore must
            // not be cancelled by an adjustment of the outgoing wallpaper.
            let result = (|| {
                let options = self.animation_options.ok_or("this renderer has no playback")?;
                let i = self
                    .screens
                    .iter()
                    .position(|s| s.name == output.monitor)
                    .ok_or_else(|| format!("no monitor named {}", output.monitor))?;
                let screen = &self.screens[i];
                if screen.asked.is_some()
                    || screen
                        .incoming
                        .as_ref()
                        .is_some_and(|a| a.picture.intent != Intent::Adjust)
                {
                    return Err("wallpaper replacement is pending; retry the adjustment".into());
                }
                let active = screen.animation.as_ref().ok_or("no live animation to adjust")?;
                let source = self
                    .playback
                    .as_ref()
                    .and_then(|r| r.source(active.target.session))
                    .ok_or("no live animation to adjust")?;
                if source.key().identity != *identity {
                    return Err("adjustment source does not match the live animation".into());
                }
                Ok((i, active.target.session, source.key().application.clone(), options))
            })();
            let (i, generation, application, options) = match result {
                Ok(target) => target,
                Err(error) => {
                    let _ = reply.send(Err(error));
                    continue;
                }
            };
            picture.application = application;
            self.cancel_animation(i);
            let screen = &mut self.screens[i];
            screen.answer(Err("superseded by a newer adjustment".into()));
            screen.generation += 1;
            screen.waiting.push(reply);
            // Reuse both the source and texture. Keep the outgoing target running
            // until this immediate handoff succeeds, including for a solo output.
            if let Err(error) = self.stage_animation(i, generation, picture, options, None) {
                self.fail_animation(i, error);
            }
        }
        answers
    }

    pub(super) fn accept_animation(&mut self, loan: &mut Option<Handoff>) -> Result<Admission, String> {
        let options = self.animation_options.ok_or("this renderer has no playback")?;
        let handoff = loan.as_ref().ok_or("animation handoff already consumed")?;
        if self.live.is_none() {
            let gpu = self
                .gpu_device()
                .and_then(|device| Painter::new(device.clone()).map(|painter| (device, painter)));
            let (device, painter) = match gpu {
                Ok(gpu) => gpu,
                Err(reason) => {
                    return Ok(Admission::Fallback(super::fallback::Offer {
                        reason,
                        budget: self.fallback_budget.as_ref().unwrap().clone(),
                    }));
                }
            };
            self.live = Some(Live {
                painter,
                device,
                budget: Budget::new(options.texture_bytes),
                textures: BTreeMap::new(),
                next_output: 0,
                staging: false,
            });
        }
        if self.playback.is_none() {
            let mut runtime = Runtime::new(self.handle.clone(), options.capacity)?;
            runtime.set_manual_pause(self.paused);
            self.playback = Some(runtime);
        }
        if self.playback.as_ref().unwrap().available() < handoff.sessions.len() {
            return Err("playback admission exhausted (including retiring workers)".into());
        }
        // Allocate/upload every source before changing any screen. Rejection
        // leaves the loan with its caller for synchronous worker cleanup there.
        let live = self.live.as_ref().unwrap();
        let mut textures = Vec::new();
        for session in &handoff.sessions {
            let frame = session.first();
            let size = frame.frame().pixels.dimensions();
            let texture = Texture::new(live.device.clone(), &live.budget, size, frame.generation)?;
            texture.upload(frame.generation, frame.sequence, size, frame.frame().pixels.as_raw())?;
            textures.push(texture);
        }
        let handoff = loan.take().unwrap();
        let transition = Transition::from_setting(
            &handoff.batch.transition.kind,
            handoff.batch.transition.duration_ms.min(10_000),
        );
        self.live.as_mut().unwrap().staging = true;
        let mut generations = Vec::new();
        for (session, texture) in handoff.sessions.into_iter().zip(textures) {
            // Admission was checked for the entire batch on this same thread.
            let generation = self
                .playback
                .as_mut()
                .unwrap()
                .insert(session)
                .unwrap_or_else(|_| unreachable!("serial batch admission changed"));
            self.live.as_mut().unwrap().textures.insert(generation, texture);
            generations.push(generation);
        }
        let mut answers = Vec::new();
        for (index, output) in handoff.batch.outputs.iter().enumerate() {
            let (reply, answer) = mpsc::channel();
            answers.push((output.monitor.clone(), answer));
            let picture = handoff.batch.selection(output.picture.clone());
            let Some(group) = handoff.groups[index] else {
                self.handle(Msg::Show {
                    monitor: output.monitor.clone(),
                    picture,
                    transition,
                    reply,
                });
                continue;
            };
            let Some(i) = self.screens.iter().position(|s| s.name == output.monitor) else {
                let _ = reply.send(Err(format!("no monitor named {}", output.monitor)));
                continue;
            };
            self.cancel_animation(i);
            self.finish(i);
            let screen = &mut self.screens[i];
            screen.answer(Err("superseded by a newer wallpaper request".into()));
            screen.asked = None;
            screen.generation += 1; // Reject pending static worker results too.
            screen.waiting.push(reply);
            self.suspend_animation(i, true);
            if let Err(error) = self.stage_animation(i, generations[group], picture, options, transition) {
                self.fail_animation(i, error);
            }
        }
        self.live.as_mut().unwrap().staging = false;
        self.reconcile_animation();
        Ok(Admission::Accepted(answers))
    }

    fn stage_animation(
        &mut self,
        i: usize,
        generation: u64,
        picture: Picture,
        options: Options,
        transition: Option<Transition>,
    ) -> Result<(), String> {
        let transition = transition.filter(|_| self.screens[i].drawn.is_some());
        let timeout = options
            .configure_timeout
            .saturating_add(transition.map_or(Duration::ZERO, |t| t.duration));
        let live = self.live.as_mut().unwrap();
        let target = live.target(generation)?;
        let texture = live.textures[&generation].clone();
        let surface = self.compositor.create_surface(&self.qh);
        let viewport = self
            .viewporter
            .as_ref()
            .map(|v| v.get_viewport(&surface, &self.qh, Quiet));
        let layer = self.layer_shell.create_layer_surface(
            &self.qh,
            surface,
            Layer::Background,
            Some("wallpicker"),
            Some(&self.screens[i].output),
        );
        layer.set_anchor(Anchor::all());
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.commit();
        self.screens[i].incoming = Some(Incoming {
            active: None,
            target,
            texture,
            picture,
            layer,
            viewport,
            logical: (1, 1),
            timer: None,
            notified: false,
            reveal: transition.map(transition::Reveal::new),
        });
        let token = self
            .handle
            .insert_source(Timer::from_duration(timeout), move |_, _, state| {
                if let Some(i) = state
                    .screens
                    .iter()
                    .position(|s| s.incoming.as_ref().is_some_and(|a| a.target == target))
                {
                    state.screens[i].incoming.as_mut().unwrap().timer = None;
                    state.fail_animation(i, "animation handoff timed out".into());
                }
                TimeoutAction::Drop
            })
            .map_err(|e| e.error.to_string())?;
        self.screens[i].incoming.as_mut().unwrap().timer = Some(token);
        Ok(())
    }

    pub(super) fn configure_animation(&mut self, i: usize, configured: (u32, u32)) {
        let info = self.outputs.info(&self.screens[i].output);
        let fallback = info
            .as_ref()
            .and_then(|o| o.logical_size)
            .map(|(w, h)| (w.max(1) as u32, h.max(1) as u32));
        let logical = if configured.0 == 0 || configured.1 == 0 {
            fallback.unwrap_or((1, 1))
        } else {
            configured
        };
        let incoming = self.screens[i].incoming.as_mut().unwrap();
        let (size, place) = match (&incoming.viewport, info.as_ref().and_then(mode_pixels)) {
            (Some(_), Some(size)) => (size, Place { logical, scale: 1 }),
            _ => {
                let scale = info.as_ref().map_or(1, |o| o.scale_factor.max(1));
                (
                    (logical.0 * scale as u32, logical.1 * scale as u32),
                    Place { logical, scale },
                )
            }
        };
        let logical_changed = incoming.logical != logical;
        incoming.logical = logical;
        place.apply(incoming.layer.wl_surface(), incoming.viewport.as_ref());
        let result = (|| {
            if let Some(active) = &mut incoming.active {
                active.surface.resize(size)?;
                if (active.size != size || logical_changed)
                    && let Some(reveal) = &mut incoming.reveal
                {
                    reveal.endpoint_pending = false;
                }
                active.size = size;
            } else {
                let surface = Surface::new(
                    self.live.as_ref().unwrap().painter.clone(),
                    incoming.layer.wl_surface(),
                    size,
                )?;
                if incoming.reveal.is_none() {
                    self.playback.as_mut().unwrap().attach(
                        incoming.target.session,
                        incoming.target.output,
                        self.animation_options.unwrap().interval,
                    )?;
                }
                incoming.active = Some(Active {
                    surface,
                    target: incoming.target,
                    texture: incoming.texture.clone(),
                    geometry: incoming.picture.picture.geometry,
                    size,
                    presentations: 0,
                    error: None,
                });
            }
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            self.fail_animation(i, error);
        } else if self.screens[i].incoming.as_ref().is_some_and(Incoming::transitioning) {
            self.step_animation_transition(i);
        }
    }

    pub(super) fn refresh_animation_transition(&mut self, i: usize) {
        // A physical mode/rotation update need not produce a new layer configure.
        // Keep a configured incoming EGL target in sync before its endpoint.
        if let Some(incoming) = &self.screens[i].incoming
            && incoming.reveal.is_some()
            && incoming.active.is_some()
        {
            let logical = incoming.logical;
            self.configure_animation(i, logical);
        }
    }

    pub(super) fn animation_presented(&mut self, ticket: Ticket, result: Result<(), String>) {
        self.complete_animation(ticket.target(), result);
    }

    fn complete_animation(&mut self, target: Target, result: Result<(), String>) {
        let Some(i) = self
            .screens
            .iter()
            .position(|s| s.incoming.as_ref().is_some_and(|a| a.target == target))
        else {
            return;
        };
        if let Err(error) = result {
            self.fail_animation(i, error);
            return;
        }
        let mut incoming = self.screens[i].incoming.take().unwrap();
        if let Some(timer) = incoming.timer.take() {
            self.handle.remove(timer);
        }
        if let Some(timer) = incoming.reveal.as_mut().and_then(|r| r.timer.take()) {
            self.handle.remove(timer);
        }
        let screen = &mut self.screens[i];
        let old = screen.animation.take();
        let old_target = old.as_ref().map(|a| a.target);
        drop(old); // EGL before replacing/destroying the old layer role.
        if let Some(viewport) = screen.viewport.take() {
            viewport.destroy();
        }
        screen.layer = incoming.layer.clone();
        screen.viewport = incoming.viewport.take();
        screen.size = Some(incoming.logical);
        screen.animation = incoming.active.take();
        screen.drawn = screen.animation.as_ref().map(|a| a.size);
        screen.shown = Want::File(incoming.picture.clone());
        screen.render_error = None;
        screen.generation += 1; // Reject outgoing static resize work still in flight.
        screen.frame_pending = false;
        screen.answer(Ok(()));
        if let Some(target) = old_target {
            self.playback.as_mut().unwrap().unbind(target);
        }
        self.reconcile_animation();
    }

    pub(super) fn suspend_animation(&mut self, i: usize, suspended: bool) {
        if let Some(active) = &self.screens[i].animation {
            self.playback.as_mut().unwrap().suspend(active.target, suspended);
        }
    }

    pub(super) fn cancel_animation(&mut self, i: usize) {
        if let Some(mut incoming) = self.screens[i].incoming.take() {
            if let Some(timer) = incoming.timer.take() {
                self.handle.remove(timer);
            }
            if let Some(timer) = incoming.reveal.as_mut().and_then(|r| r.timer.take()) {
                self.handle.remove(timer);
            }
            if incoming.active.is_some() {
                self.playback.as_mut().unwrap().unbind(incoming.target);
            }
            drop(incoming);
        }
        self.suspend_animation(i, false);
        self.reconcile_animation();
    }

    pub(super) fn fail_animation(&mut self, i: usize, error: String) {
        self.cancel_animation(i);
        self.screens[i].answer(Err(error));
    }

    pub(super) fn remove_animation(&mut self, i: usize) {
        if let Some(active) = self.screens[i].animation.take() {
            self.playback.as_mut().unwrap().unbind(active.target);
            drop(active);
        }
        self.reconcile_animation();
    }

    pub(super) fn disconnect_animation(&mut self, i: usize) {
        self.cancel_animation(i);
        self.remove_animation(i);
        self.screens[i].answer(Err("monitor disconnected before wallpaper handoff".into()));
    }

    fn reconcile_animation(&mut self) {
        let Some(live) = &mut self.live else { return };
        if live.staging {
            return;
        }
        let runtime = self.playback.as_mut().unwrap();
        live.textures.retain(|generation, _| {
            let pending = self
                .screens
                .iter()
                .any(|s| s.incoming.as_ref().is_some_and(|a| a.target.session == *generation));
            let active = self
                .screens
                .iter()
                .any(|s| s.animation.as_ref().is_some_and(|a| a.target.session == *generation));
            if !pending && !active {
                runtime.retire(*generation);
                return false;
            }
            // Shared endpoint barrier: successful members start together
            // after every incoming layer has succeeded, failed or timed out.
            if !pending {
                runtime.set_paused(*generation, PauseReason::Transition, false);
            }
            true
        });
    }

    pub(super) fn resize_animation(&mut self, i: usize) {
        let Some((size, place)) = self.target(&self.screens[i]) else {
            return;
        };
        let screen = &mut self.screens[i];
        place.apply(screen.layer.wl_surface(), screen.viewport.as_ref());
        let replacing = screen.incoming.is_some();
        let active = screen.animation.as_mut().unwrap();
        if size == active.size {
            screen.layer.commit();
            return;
        }
        let result = (|| {
            active.surface.resize(size)?;
            let target = self.live.as_mut().unwrap().target(active.target.session)?;
            let runtime = self.playback.as_mut().unwrap();
            runtime.attach(target.session, target.output, self.animation_options.unwrap().interval)?;
            if replacing {
                runtime.suspend(target, true);
            }
            runtime.unbind(active.target); // Attach first so the shared clock survives.
            active.target = target;
            active.size = size;
            active.error = None;
            screen.drawn = Some(size);
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            active.error = Some(error);
            self.playback.as_mut().unwrap().suspend(active.target, true);
        }
    }

    pub(super) fn animation_status(&self, screen: &Screen) -> Value {
        if let Want::Still(picture, still) = &screen.shown {
            return json!({"state":"unavailable", "reason":still.reason(), "representative":true,
                "geometry":picture.picture.geometry, "error":screen.render_error});
        }
        let Some(active) = &screen.animation else {
            let animated = screen
                .shown
                .picture()
                .is_some_and(|p| matches!(p.picture.content, Content::Animated { .. }));
            return if screen.incoming.is_some() {
                json!({"state":"preparing"})
            } else if animated {
                json!({"state":"still"}) // Applied while playback was off.
            } else {
                Value::Null
            };
        };
        let runtime = self.playback.as_ref().unwrap();
        let source = runtime.source(active.target.session);
        let error = active
            .error
            .as_deref()
            .or_else(|| runtime.output_error(active.target))
            .or_else(|| source.and_then(|s| s.error()));
        let state = if error.is_some() {
            "frozen"
        } else if screen.incoming.is_some() {
            "replacing"
        } else if source.is_some_and(|s| s.running()) {
            "playing"
        } else if source.is_some_and(|s| s.paused_by(PauseReason::Hidden) && !s.paused_by(PauseReason::Manual)) {
            "hidden" // Locked, powered off or covered.
        } else {
            "paused"
        };
        json!({ "state": state,
            "generation": active.target.session, "output_generation": active.target.output,
            "geometry": active.geometry,
            "sequence": source.map(|s| s.current().sequence), "uploaded_sequence": active.texture.sequence(),
            "uploads": active.texture.uploads(), "presentations": active.presentations,
            "pending": screen.incoming.is_some(), "error": error })
    }
}
