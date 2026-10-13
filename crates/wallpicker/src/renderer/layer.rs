//! Layer configuration and compositor callbacks for resize and animation.

use super::state::State;
use smithay_client_toolkit::compositor::{CompositorHandler, FrameCallbackData};
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_surface};
use smithay_client_toolkit::reexports::client::{Connection, QueueHandle};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure};
use std::time::Instant;

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        if let Some(i) = self
            .screens
            .iter()
            .position(|s| s.incoming.as_ref().is_some_and(|a| &a.layer == layer))
        {
            self.fail_animation(i, "incoming animation layer was closed".into());
            return;
        }
        if let Some(i) = self
            .screens
            .iter()
            .position(|s| s.overlay.as_ref().is_some_and(|o| &o.layer == layer))
        {
            self.finish(i);
            return;
        }
        if let Some(i) = self.screens.iter().position(|s| &s.layer == layer) {
            self.disconnect_animation(i);
            self.screens.remove(i);
        }
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if let Some(i) = self
            .screens
            .iter()
            .position(|s| s.incoming.as_ref().is_some_and(|a| &a.layer == layer))
        {
            self.configure_animation(i, configure.new_size);
            return;
        }
        if let Some(i) = self
            .screens
            .iter()
            .position(|s| s.overlay.as_ref().is_some_and(|o| &o.layer == layer))
        {
            let screen = &mut self.screens[i];
            let o = screen.overlay.as_mut().unwrap();
            o.place.apply(&o.surface, o.viewport.as_ref());
            let starting = o.started.is_none();
            if let Some(gpu) = &o.gpu {
                // The shader supplies premultiplied pixel alpha. No modifier fade.
                o.alpha.set_multiplier(u32::MAX);
                let t = o.started.map_or(0.0, |start| o.transition.progress(start.elapsed()));
                if let Err(e) = gpu.draw(t) {
                    eprintln!("wallpicker: shader transition failed to map: {e}");
                    self.finish(i);
                    return;
                }
            } else {
                if starting {
                    o.surface.attach(Some(&o.buffer), 0, 0);
                }
                o.surface.damage_buffer(0, 0, o.size.0 as i32, o.size.1 as i32);
                o.layer.commit();
            }
            if starting {
                o.started = Some(Instant::now());
            }
            if !screen.frame_pending {
                let parent = screen.layer.wl_surface();
                parent.frame(&self.qh, FrameCallbackData(parent.clone()));
                screen.layer.commit();
                screen.frame_pending = true;
            }
            return;
        }
        let Some(i) = self.screens.iter().position(|s| &s.layer == layer) else {
            return;
        };
        let (w, h) = configure.new_size;
        let screen = &mut self.screens[i];
        // Anchored to every edge, it's given the output's size; 0 only if
        // the compositor leaves it to us.
        let logical = self.outputs.info(&screen.output).and_then(|i| i.logical_size);
        let (w, h) = match (w, h, logical) {
            (0, _, Some((lw, lh))) | (_, 0, Some((lw, lh))) => (lw.max(1) as u32, lh.max(1) as u32),
            _ => (w.max(1), h.max(1)),
        };
        screen.size = Some((w, h));
        self.finish(i);
        let screen = &mut self.screens[i];
        // What it shows, stretched to the new size until it's drawn again.
        if let (Some(v), Some(_)) = (&screen.viewport, screen.drawn) {
            v.set_destination(w as i32, h as i32);
            screen.layer.commit();
        }
        self.redraw(i, false);
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: i32) {
        if let Some(i) = self.screens.iter().position(|s| s.layer.wl_surface() == surface) {
            self.redraw(i, false);
        }
    }
    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, surface: &wl_surface::WlSurface, _: u32) {
        if let Some(i) = self.screens.iter().position(|s| s.layer.wl_surface() == surface) {
            self.screens[i].frame_pending = false;
            self.step(i);
        }
    }
    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}
