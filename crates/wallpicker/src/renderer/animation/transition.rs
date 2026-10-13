//! Frozen incoming frames, paced by tagged callbacks, then promoted in place.
use super::{Incoming, State, Target, Transition};
use serde_json::{Value, json};
use smithay_client_toolkit::dispatch2::Dispatch2;
use smithay_client_toolkit::reexports::calloop::{
    RegistrationToken,
    timer::{TimeoutAction, Timer},
};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle, protocol::wl_callback};
use smithay_client_toolkit::shell::WaylandSurface;
use std::time::Instant;

pub(super) struct Reveal {
    transition: Transition,
    started: Option<Instant>,
    last_swap: Option<Instant>,
    serial: u64,
    pending: Option<u64>,
    pub(super) endpoint_pending: bool,
    pub(super) timer: Option<RegistrationToken>,
}

impl Reveal {
    pub(super) fn new(transition: Transition) -> Self {
        Self {
            transition,
            started: None,
            last_swap: None,
            serial: 0,
            pending: None,
            endpoint_pending: false,
            timer: None,
        }
    }

    fn callback(&mut self, serial: u64) -> bool {
        if self.pending != Some(serial) {
            return false;
        }
        self.pending = None;
        true
    }
}

struct Callback {
    target: Target,
    serial: u64,
}
impl Dispatch2<wl_callback::WlCallback, State> for Callback {
    fn event(
        &self,
        state: &mut State,
        _: &wl_callback::WlCallback,
        event: wl_callback::Event,
        _: &Connection,
        _: &QueueHandle<State>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.animation_transition_callback(self.target, self.serial);
        }
    }
}

impl State {
    pub(super) fn step_animation_transition(&mut self, i: usize) {
        let Some(incoming) = self.screens[i].incoming.as_mut() else {
            return;
        };
        let Some(reveal) = incoming.reveal.as_mut() else { return };
        if reveal.pending.is_some() || reveal.timer.is_some() {
            return;
        }
        let Some(active) = incoming.active.as_mut() else { return };
        let now = Instant::now();
        let progress = reveal.started.map_or(0.0, |started| {
            reveal.transition.progress(now.saturating_duration_since(started))
        });
        let result = (|| {
            let serial = reveal
                .serial
                .checked_add(1)
                .ok_or("transition callback serial exhausted")?;
            active.surface.draw_effect(
                &active.texture,
                active.geometry,
                reveal.transition.effect,
                progress,
                || {
                    incoming.layer.wl_surface().frame(
                        &self.qh,
                        Callback {
                            target: incoming.target,
                            serial,
                        },
                    );
                },
            )?;
            reveal.started.get_or_insert(now);
            reveal.last_swap = Some(Instant::now());
            reveal.serial = serial;
            reveal.pending = Some(serial);
            reveal.endpoint_pending = progress >= 1.0;
            active.presentations = active.presentations.saturating_add(1);
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            self.fail_animation(i, error);
        }
    }

    fn animation_transition_callback(&mut self, target: Target, serial: u64) {
        let Some(i) = self
            .screens
            .iter()
            .position(|s| s.incoming.as_ref().is_some_and(|a| a.target == target))
        else {
            return;
        };
        let Some(reveal) = self.screens[i].incoming.as_mut().and_then(|a| a.reveal.as_mut()) else {
            return;
        };
        if !reveal.callback(serial) {
            return;
        }
        if reveal.endpoint_pending {
            // The exact opaque endpoint was swapped AND its callback arrived.
            // Attach before promotion; the group barrier still holds decoding.
            let result = self.playback.as_mut().unwrap().attach(
                target.session,
                target.output,
                self.animation_options.unwrap().interval,
            );
            self.complete_animation(target, result.map(|_| ()));
            return;
        }
        let interval = self.animation_options.unwrap().interval;
        let remaining = interval.saturating_sub(reveal.last_swap.unwrap().elapsed());
        if remaining.is_zero() {
            self.step_animation_transition(i);
            return;
        }
        // At most one deadline and one callback per incoming output. A timer is
        // armed only after callback completion, never to bypass backpressure.
        let timer = self
            .handle
            .insert_source(Timer::from_duration(remaining), move |_, _, state| {
                if let Some(i) = state
                    .screens
                    .iter()
                    .position(|s| s.incoming.as_ref().is_some_and(|a| a.target == target))
                {
                    if let Some(reveal) = state.screens[i].incoming.as_mut().and_then(|a| a.reveal.as_mut()) {
                        reveal.timer = None;
                    }
                    state.step_animation_transition(i);
                }
                TimeoutAction::Drop
            });
        match timer {
            Ok(token) => {
                self.screens[i]
                    .incoming
                    .as_mut()
                    .unwrap()
                    .reveal
                    .as_mut()
                    .unwrap()
                    .timer = Some(token)
            }
            Err(error) => self.fail_animation(i, error.error.to_string()),
        }
    }

    pub(in crate::renderer) fn animation_transition_status(&self, incoming: Option<&Incoming>) -> Value {
        let Some(incoming) = incoming else { return Value::Null };
        let Some(reveal) = &incoming.reveal else {
            return Value::Null;
        };
        let source = self.playback.as_ref().and_then(|r| r.source(incoming.target.session));
        json!({
            "effect": format!("{:?}", reveal.transition.effect),
            "generation": incoming.target.session,
            "sequence": source.map(|s| s.current().sequence),
            "playing": source.is_some_and(|s| s.running()),
            "uploads": incoming.texture.uploads(),
            "presentations": incoming.active.as_ref().map_or(0, |a| a.presentations),
            "progress": reveal.started.map_or(0.0, |t| reveal.transition.progress(t.elapsed())),
            "endpoint_pending": reveal.endpoint_pending,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callback_serials_reject_duplicates_and_obsolete_swaps() {
        let mut reveal = Reveal::new(Transition::from_setting("fade", 100).unwrap());
        assert!(!reveal.callback(1));
        reveal.pending = Some(2);
        assert!(!reveal.callback(1));
        assert!(reveal.callback(2));
        assert!(!reveal.callback(2));
        reveal.pending = Some(3);
        assert!(!reveal.callback(2));
        assert!(reveal.callback(3));
    }
}
