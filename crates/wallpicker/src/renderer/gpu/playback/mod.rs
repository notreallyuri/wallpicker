//! Persistent presentation primitives. Scheduling, output/session ownership and
//! transitions are supplied by the renderer; this layer never requests callbacks.
mod budget;
mod mapping;
mod painter;
mod surface;
#[cfg(test)]
mod tests;
mod texture;

pub(in crate::renderer) use budget::Budget;
pub(in crate::renderer) use painter::Painter;
pub(in crate::renderer) use surface::Surface;
pub(in crate::renderer) use texture::Texture;

use glow::HasContext;

fn dimensions((width, height): (u32, u32)) -> Result<u64, String> {
    if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
        return Err("invalid playback dimensions".into());
    }
    Ok(u64::from(width) * u64::from(height) * 4)
}

fn check(gl: &glow::Context, action: &str) -> Result<(), String> {
    // SAFETY: all callers have made the owning GLES context current.
    let error = unsafe { gl.get_error() };
    if error != glow::NO_ERROR {
        return Err(format!("GLES {action} failed: 0x{error:x}"));
    }
    Ok(())
}
