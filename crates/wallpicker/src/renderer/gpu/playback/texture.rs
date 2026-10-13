use super::{Budget, budget::Reservation, check, dimensions};
use crate::renderer::gpu::Device;
use glow::HasContext;
use std::{cell::Cell, rc::Rc};

/// One source-resolution allocation shared by all outputs in a session.
pub(in crate::renderer) struct Texture {
    pub(super) device: Rc<Device>,
    pub(super) id: glow::Texture,
    pub(super) size: (u32, u32),
    generation: u64,
    sequence: Cell<Option<u64>>,
    uploads: Cell<u64>,
    _reservation: Reservation,
}

impl Texture {
    pub(in crate::renderer) fn new(
        device: Rc<Device>,
        budget: &Budget,
        size: (u32, u32),
        generation: u64,
    ) -> Result<Rc<Self>, String> {
        let bytes = dimensions(size)?;
        let reservation = budget.reserve(bytes)?;
        device.context.current(None)?;
        let gl = &device.gl;
        // SAFETY: scratch context is current; object is deleted if setup fails.
        let id = unsafe {
            let maximum = gl.get_parameter_i32(glow::MAX_TEXTURE_SIZE);
            if maximum <= 0 || size.0 > maximum as u32 || size.1 > maximum as u32 {
                return Err("source exceeds GLES texture dimensions".into());
            }
            let texture = gl.create_texture()?;
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                // The shader interpolates premultiplied samples before flattening
                // onto black, avoiding fringes from straight-alpha filtering.
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::NEAREST as i32);
            }
            for parameter in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::CLAMP_TO_EDGE as i32);
            }
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA as i32,
                size.0 as i32,
                size.1 as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            if let Err(error) = check(gl, "texture allocation") {
                gl.delete_texture(texture);
                return Err(error);
            }
            texture
        };
        Ok(Rc::new(Self {
            device,
            id,
            size,
            generation,
            sequence: Cell::new(None),
            uploads: Cell::new(0),
            _reservation: reservation,
        }))
    }

    /// Raw source RGBA, uploaded directly without CPU conversion or resizing.
    /// Shared outputs requesting the same sequence do not cause another upload.
    pub(in crate::renderer) fn upload(
        &self,
        generation: u64,
        sequence: u64,
        size: (u32, u32),
        rgba: &[u8],
    ) -> Result<bool, String> {
        if generation != self.generation {
            return Err("obsolete playback generation".into());
        }
        if size != self.size || rgba.len() as u64 != dimensions(size)? {
            return Err("invalid playback frame dimensions/bytes".into());
        }
        if let Some(previous) = self.sequence.get() {
            if sequence < previous {
                return Err("obsolete playback frame".into());
            }
            if sequence == previous {
                return Ok(false);
            }
        }
        self.device.context.current(None)?;
        let gl = &self.device.gl;
        // SAFETY: exact RGBA byte length matches the fixed source allocation.
        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.id));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                size.0 as i32,
                size.1 as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(Some(rgba)),
            );
        }
        check(gl, "frame upload")?;
        self.sequence.set(Some(sequence));
        self.uploads.set(self.uploads.get().saturating_add(1));
        Ok(true)
    }

    pub(in crate::renderer) fn sequence(&self) -> Option<u64> {
        self.sequence.get()
    }
    pub(in crate::renderer) fn uploads(&self) -> u64 {
        self.uploads.get()
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.device.context.current(None).is_ok() {
            // SAFETY: owning context current; final Rc owns this texture once.
            unsafe {
                self.device.gl.delete_texture(self.id);
            }
        }
    }
}
