//! Buffer submission, compositor-driven fading, and the final surface handoff.

use super::Transition;
use super::gpu::{Device, Target};
use super::image::Made;
use super::output::mode_pixels;
use super::protocol::{BufferData, Quiet};
use super::screen::{Overlay, Place, Screen};
use super::state::State;
use super::transition::Effect;
use smithay_client_toolkit::compositor::FrameCallbackData;
use smithay_client_toolkit::reexports::client::protocol::{wl_buffer, wl_shm};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{Anchor, KeyboardInteractivity, Layer};
use smithay_client_toolkit::shm::raw::RawPool;

impl State {
    pub(super) fn gpu_device(&mut self) -> Result<std::rc::Rc<Device>, String> {
        if let Some(error) = &self.gpu_unavailable {
            return Err(error.clone());
        }
        if let Some(gpu) = &self.gpu {
            return Ok(gpu.clone());
        }
        match Device::new(&self.connection) {
            Ok(gpu) => {
                self.gpu = Some(gpu.clone());
                Ok(gpu)
            }
            Err(e) => {
                self.gpu_unavailable = Some(e.clone());
                Err(e)
            }
        }
    }

    /// The pixels to draw for `screen`, and how they're laid on it.
    pub(super) fn target(&self, screen: &Screen) -> Option<((u32, u32), Place)> {
        let (w, h) = screen.size?;
        let info = self.outputs.info(&screen.output);
        match (&screen.viewport, info.as_ref().and_then(mode_pixels)) {
            // The output's own pixels, scaled to the surface by the compositor.
            (Some(_), Some(px)) => Some((
                px,
                Place {
                    logical: (w, h),
                    scale: 1,
                },
            )),
            _ => {
                let scale = info.map_or(1, |i| i.scale_factor.max(1));
                Some(((w * scale as u32, h * scale as u32), Place { logical: (w, h), scale }))
            }
        }
    }

    /// Put `made` on screen `i`, from a pool of its own (freed on release),
    /// coming in with `transition` over what it shows. False when it was
    /// made for another size.
    pub(super) fn draw(&mut self, i: usize, made: &Made, transition: Option<Transition>) -> Result<bool, String> {
        let ((w, h), place) = match self.target(&self.screens[i]) {
            Some((px, place)) if px == made.size => (px, place),
            _ => return Ok(false),
        };
        // One still on its way ends now.
        self.finish(i);
        let stride = w as usize * 4;
        let mut pool = RawPool::new(stride * h as usize, &self.shm).map_err(|e| e.to_string())?;
        pool.mmap().copy_from_slice(&made.pixels);
        let buffer = pool.create_buffer(
            0,
            w as i32,
            h as i32,
            stride as i32,
            wl_shm::Format::Argb8888,
            BufferData,
            &self.qh,
        );
        let screen = &self.screens[i];
        let parent = screen.layer.wl_surface();
        // Nothing shown yet: nothing to come in over.
        let transition = transition.filter(|_| screen.drawn.is_some());
        let (Some(mut transition), Some(alpha_modifier)) = (transition, &self.alpha_modifier) else {
            place.apply(parent, screen.viewport.as_ref());
            parent.attach(Some(&buffer), 0, 0);
            parent.damage_buffer(0, 0, w as i32, h as i32);
            screen.layer.commit();
            self.in_flight.push((buffer, pool));
            return Ok(true);
        };
        // Hyprland does not apply alpha modifiers to layer-shell subsurfaces.
        // A second background layer has its own desktop surface and opacity.
        // It is created after the persistent layer, so Hyprland stacks it above.
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
            Some(&screen.output),
        );
        layer.set_anchor(Anchor::all());
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        let surface = layer.wl_surface().clone();
        let alpha = alpha_modifier.get_surface(&surface, &self.qh, Quiet);
        let gpu = if transition.effect.shader().is_some() {
            let result = self
                .gpu_device()
                .and_then(|device| Target::new(device, &surface, (w, h), &made.pixels, transition.effect));
            match result {
                Ok(target) => Some(target),
                Err(e) => {
                    eprintln!("wallpicker: shader transition unavailable, using Fade: {e}");
                    transition.effect = Effect::Fade;
                    None
                }
            }
        } else {
            None
        };
        alpha.set_multiplier(0);
        // No buffer until configure has been acknowledged by SCTK.
        layer.commit();
        let overlay = Overlay {
            transition,
            gpu,
            started: None,
            endpoint_pending: false,
            layer,
            surface,
            alpha,
            viewport,
            buffer,
            pool: Some(pool),
            size: (w, h),
            place,
        };
        self.screens[i].overlay = Some(overlay);
        Ok(true)
    }

    /// Screen `i`'s transition one frame on (it ends when it's over).
    pub(super) fn step(&mut self, i: usize) {
        let Some(o) = &mut self.screens[i].overlay else {
            return;
        };
        if o.endpoint_pending {
            return self.finish(i);
        }
        let Some(started) = o.started else { return };
        let t = o.transition.progress(started.elapsed());
        if t >= 1.0 && o.gpu.is_none() {
            return self.finish(i);
        }
        if let Some(gpu) = &o.gpu {
            if let Err(e) = gpu.draw(t) {
                eprintln!("wallpicker: shader transition stopped: {e}");
                return self.finish(i);
            }
            // Submit the unshifted final image before destroying this layer.
            // Compositors may animate layer removal using its last buffer;
            // an incomplete slide would otherwise leave a frozen ghost over
            // the static wallpaper. The next callback retires the endpoint.
            o.endpoint_pending = t >= 1.0;
        } else {
            o.alpha.set_multiplier((t * f64::from(u32::MAX)) as u32);
            o.surface.damage_buffer(0, 0, o.size.0 as i32, o.size.1 as i32);
            o.surface.commit();
        }
        let parent = self.screens[i].layer.wl_surface();
        parent.frame(&self.qh, FrameCallbackData(parent.clone()));
        self.screens[i].layer.commit();
        self.screens[i].frame_pending = true;
    }

    /// Screen `i`'s transition over: the new picture on the layer surface
    /// (a new buffer from the same memory), the temporary layer gone.
    pub(super) fn finish(&mut self, i: usize) {
        let Some(mut o) = self.screens[i].overlay.take() else {
            return;
        };
        let Some(mut pool) = o.pool.take() else {
            return;
        };
        let (w, h) = o.size;
        let buffer = pool.create_buffer(
            0,
            w as i32,
            h as i32,
            w as i32 * 4,
            wl_shm::Format::Argb8888,
            BufferData,
            &self.qh,
        );
        let screen = &self.screens[i];
        let surface = screen.layer.wl_surface();
        o.place.apply(surface, screen.viewport.as_ref());
        surface.attach(Some(&buffer), 0, 0);
        surface.damage_buffer(0, 0, w as i32, h as i32);
        screen.layer.commit();
        self.in_flight.push((buffer, pool));
        // Dropped after the layer surface has the picture: no gap between.
        drop(o);
    }

    pub(super) fn released(&mut self, buffer: &wl_buffer::WlBuffer) {
        if let Some(i) = self.in_flight.iter().position(|(b, _)| b == buffer) {
            let (buffer, _pool) = self.in_flight.swap_remove(i);
            buffer.destroy();
        }
    }
}
