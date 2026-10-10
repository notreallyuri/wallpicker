//! Per-output surfaces, transition resources, placement, and cleanup.

use super::gpu::Target;
use super::image::Want;
use super::{Picture, Reply, Transition};
use smithay_client_toolkit::reexports::client::protocol::{wl_buffer, wl_output, wl_surface};
use smithay_client_toolkit::reexports::protocols::wp::alpha_modifier::v1::client::wp_alpha_modifier_surface_v1::WpAlphaModifierSurfaceV1;
use smithay_client_toolkit::reexports::protocols::wp::viewporter::client::wp_viewport::WpViewport;
use smithay_client_toolkit::shell::wlr_layer::LayerSurface;
use smithay_client_toolkit::shm::raw::RawPool;
use std::time::Instant;

/// One output's background surface.
pub(super) struct Screen {
    /// The output's global name.
    pub(super) id: u32,
    pub(super) name: String,
    pub(super) output: wl_output::WlOutput,
    pub(super) layer: LayerSurface,
    pub(super) viewport: Option<WpViewport>,
    /// Its size on the layout (logical pixels), once configured.
    pub(super) size: Option<(u32, u32)>,
    /// What it shows, drawn again at a new size or scale.
    pub(super) shown: Want,
    /// A picture on its way, and how it comes in: it's what's shown once
    /// drawn (a file that can't be read changes nothing).
    pub(super) asked: Option<(Picture, Option<Transition>)>,
    /// A transition on its way.
    pub(super) overlay: Option<Overlay>,
    /// At most one animation callback on the opaque parent surface.
    pub(super) frame_pending: bool,
    /// Bumped by each frame asked for: one made for an older is dropped.
    pub(super) generation: u64,
    /// The size in pixels of what it shows (None: nothing yet).
    pub(super) drawn: Option<(u32, u32)>,
    /// Shows waiting for the picture to be up.
    pub(super) waiting: Vec<Reply>,
}

impl Screen {
    pub(super) fn answer(&mut self, result: Result<(), String>) {
        for reply in self.waiting.drain(..) {
            let _ = reply.send(result.clone());
        }
    }
}

/// A transition on its way: the new picture on a temporary background layer above the old.
pub(super) struct Overlay {
    pub(super) transition: Transition,
    pub(super) gpu: Option<Target>,
    /// Starts after the layer-shell configure handshake.
    pub(super) started: Option<Instant>,
    /// The exact GPU endpoint was submitted; retire after its frame callback.
    pub(super) endpoint_pending: bool,
    pub(super) layer: LayerSurface,
    pub(super) surface: wl_surface::WlSurface,
    pub(super) alpha: WpAlphaModifierSurfaceV1,
    pub(super) viewport: Option<WpViewport>,
    pub(super) buffer: wl_buffer::WlBuffer,
    /// The picture's memory, for the layer surface's buffer at the end.
    pub(super) pool: Option<RawPool>,
    /// Its size in pixels, and how it's laid on the surface.
    pub(super) size: (u32, u32),
    pub(super) place: Place,
}

impl Drop for Overlay {
    fn drop(&mut self) {
        // EGL resources and native window go before their Wayland surface.
        self.gpu.take();
        self.alpha.destroy();
        if let Some(v) = &self.viewport {
            v.destroy();
        }
        // LayerSurface drops its role and wl_surface after this cleanup.
        self.buffer.destroy();
    }
}

/// How a buffer is laid on a surface: scaled to its logical size by the
/// viewport, else at an integer buffer scale.
#[derive(Clone, Copy)]
pub(super) struct Place {
    pub(super) logical: (u32, u32),
    pub(super) scale: i32,
}

impl Place {
    pub(super) fn apply(self, surface: &wl_surface::WlSurface, viewport: Option<&WpViewport>) {
        match viewport {
            Some(v) => v.set_destination(self.logical.0 as i32, self.logical.1 as i32),
            None => surface.set_buffer_scale(self.scale),
        }
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        self.answer(Err(format!("{} was disconnected", self.name)));
        // Release the temporary layer and its protocol objects as well.
        self.overlay.take();
        if let Some(v) = &self.viewport {
            v.destroy();
        }
    }
}
