use super::{Painter, Texture};
use crate::renderer::gpu::native::Window;
use crate::renderer::transition::Effect;
use khronos_egl as egl;
use std::rc::Rc;
use wallpicker_core::render::Geometry;
use wayland_client::protocol::wl_surface::WlSurface;

/// Output surface persists across source frames, geometry edits and resize.
/// The owner must drop this before destroying its Wayland surface/layer role.
pub(in crate::renderer) struct Surface {
    // Drop the native window before the last painter/device reference.
    window: Window,
    painter: Rc<Painter>,
    surface: egl::Surface,
    size: (u32, u32),
}

impl Surface {
    pub(in crate::renderer) fn new(
        painter: Rc<Painter>,
        surface: &WlSurface,
        size: (u32, u32),
    ) -> Result<Self, String> {
        painter.validate_output(size)?;
        let window = Window::new(surface, size)?;
        let context = &painter.device.context;
        // SAFETY: caller provides a configured surface on the device's Wayland
        // connection, and the native window stays alive until EGL is destroyed.
        let surface = unsafe {
            context
                .api
                .create_window_surface(context.display, context.config, window.ptr(), None)
        }
        .map_err(|e| e.to_string())?;
        Ok(Self {
            painter,
            surface,
            window,
            size,
        })
    }

    pub(in crate::renderer) fn resize(&mut self, size: (u32, u32)) -> Result<(), String> {
        self.painter.validate_output(size)?;
        if self.size != size {
            self.window.resize(size);
            self.size = size;
        }
        Ok(())
    }

    pub(in crate::renderer) fn draw(
        &self,
        texture: &Texture,
        geometry: Geometry,
        before_swap: impl FnOnce(),
    ) -> Result<(), String> {
        self.draw_effect(texture, geometry, Effect::Fade, 1.0, before_swap)
    }

    pub(in crate::renderer) fn draw_effect(
        &self,
        texture: &Texture,
        geometry: Geometry,
        effect: Effect,
        progress: f64,
        before_swap: impl FnOnce(),
    ) -> Result<(), String> {
        let context = &self.painter.device.context;
        context.current(Some(self.surface))?;
        // Frame callbacks/deadline timers will pace presentation. Do not add an
        // EGL-vsync wait that can block the renderer when an output is occluded.
        context
            .api
            .swap_interval(context.display, 0)
            .map_err(|e| e.to_string())?;
        if progress >= 1.0 {
            self.painter.draw(texture, self.size, geometry)?;
        } else {
            self.painter
                .draw_effect(texture, self.size, geometry, effect, progress)?;
        }
        before_swap();
        context
            .api
            .swap_buffers(context.display, self.surface)
            .map_err(|e| e.to_string())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        let context = &self.painter.device.context;
        let _ = context.current(None);
        let _ = context.api.destroy_surface(context.display, self.surface);
        // The native window goes next, while its Wayland proxy is still alive.
    }
}
