//! Shader transitions and persistent playback drawing resources over EGL/GLES.
//! Transitions upload once; playback updates source textures without CPU fitting
//! or GPU readback. Wayland scheduling belongs to the renderer owner.

mod context;
mod native;
mod program;
mod target;
// Instantiated lazily by the animated Wayland host; static sessions need none.
pub(super) mod playback;

use context::Context;
use glow::HasContext;
use std::rc::Rc;
pub(super) use target::Target;
use wayland_client::Connection;

pub(super) struct Device {
    context: Context,
    gl: glow::Context,
}

impl Device {
    pub(super) fn new(connection: &Connection) -> Result<Rc<Self>, String> {
        let context = Context::new(connection)?;
        Self::from_context(context, false)
    }

    fn from_context(context: Context, allow_software: bool) -> Result<Rc<Self>, String> {
        // SAFETY: GLES context is current; EGL returns valid procedure addresses.
        let gl = unsafe {
            glow::Context::from_loader_function(|name| {
                context
                    .api
                    .get_proc_address(name)
                    .map_or(std::ptr::null(), |f| f as *const () as *const _)
            })
        };
        // Exclude Mesa's CPU rasterizers rather than silently doing pixel work
        // on the CPU. The caller falls back to the existing compositor fade.
        let renderer = unsafe { gl.get_parameter_string(glow::RENDERER) };
        let lower = renderer.to_lowercase();
        if !allow_software
            && ["llvmpipe", "softpipe", "swrast", "software"]
                .iter()
                .any(|s| lower.contains(s))
        {
            return Err(format!("software GLES renderer: {renderer}"));
        }
        eprintln!("wallpicker: shader transitions on {renderer}");
        Ok(Rc::new(Self { context, gl }))
    }

    #[cfg(test)]
    fn headless() -> Result<Rc<Self>, String> {
        Self::from_context(Context::headless()?, true)
    }
}
