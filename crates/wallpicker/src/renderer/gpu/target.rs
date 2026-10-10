//! One transition's EGL window, incoming texture, and shader draw resources.

use super::{Device, native::Window, program};
use crate::renderer::transition::Effect;
use glow::HasContext;
use khronos_egl as egl;
use std::rc::Rc;
use wayland_client::protocol::wl_surface::WlSurface;

pub(in crate::renderer) struct Target {
    device: Rc<Device>,
    surface: egl::Surface,
    _window: Window,
    program: glow::Program,
    texture: glow::Texture,
    vertices: glow::Buffer,
    size: (u32, u32),
    effect: Effect,
}

impl Target {
    pub(in crate::renderer) fn new(
        device: Rc<Device>,
        surface: &WlSurface,
        size: (u32, u32),
        pixels: &[u8],
        effect: Effect,
    ) -> Result<Self, String> {
        let window = Window::new(surface, size)?;
        let context = &device.context;
        // SAFETY: wl_egl_window and EGL display belong to the same connection.
        let egl_surface = unsafe {
            context
                .api
                .create_window_surface(context.display, context.config, window.ptr(), None)
        }
        .map_err(|e| e.to_string())?;
        let result = Self::resources(&device, effect, size, pixels);
        match result {
            Ok((program, texture, vertices)) => Ok(Self {
                device,
                surface: egl_surface,
                _window: window,
                program,
                texture,
                vertices,
                size,
                effect,
            }),
            Err(e) => {
                let _ = context.api.destroy_surface(context.display, egl_surface);
                Err(e)
            }
        }
    }

    fn resources(
        device: &Device,
        effect: Effect,
        size: (u32, u32),
        pixels: &[u8],
    ) -> Result<(glow::Program, glow::Texture, glow::Buffer), String> {
        device.context.current(None)?;
        let gl = &device.gl;
        let program = program::compile(gl, effect.shader().ok_or("effect has no shader")?)?;
        // SAFETY: scratch context is current; owned objects are deleted on error.
        unsafe {
            let texture = match gl.create_texture() {
                Ok(t) => t,
                Err(e) => {
                    gl.delete_program(program);
                    return Err(e);
                }
            };
            let vertices = match gl.create_buffer() {
                Ok(b) => b,
                Err(e) => {
                    gl.delete_texture(texture);
                    gl.delete_program(program);
                    return Err(e);
                }
            };
            gl.bind_texture(glow::TEXTURE_2D, Some(texture));
            for parameter in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::LINEAR as i32);
            }
            for parameter in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, glow::CLAMP_TO_EDGE as i32);
            }
            // The shm format is BGRX; GLES2 has portable RGBA uploads. Swizzle
            // once during preparation, never in the animation's frame loop.
            let mut rgba = pixels.to_vec();
            for pixel in rgba.as_chunks_mut::<4>().0 {
                pixel.swap(0, 2);
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
                glow::PixelUnpackData::Slice(Some(&rgba)),
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
            let positions: [f32; 8] = [-1., -1., 1., -1., -1., 1., 1., 1.];
            let bytes = std::slice::from_raw_parts(
                positions.as_ptr().cast(),
                std::mem::size_of_val(&positions),
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                gl.delete_buffer(vertices);
                gl.delete_texture(texture);
                gl.delete_program(program);
                return Err(format!("GLES upload failed: 0x{error:x}"));
            }
            Ok((program, texture, vertices))
        }
    }

    pub(in crate::renderer) fn draw(&self, progress: f64) -> Result<(), String> {
        let context = &self.device.context;
        context.current(Some(self.surface))?;
        // Frame callbacks already pace the animation. Avoid EGL waiting on a
        // second callback, especially when the transparent layer is occluded.
        context
            .api
            .swap_interval(context.display, 0)
            .map_err(|e| e.to_string())?;
        let gl = &self.device.gl;
        // SAFETY: window context is current and all resources belong to it.
        unsafe {
            gl.viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
            gl.disable(glow::BLEND);
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.uniform_1_i32(gl.get_uniform_location(self.program, "image").as_ref(), 0);
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "progress").as_ref(),
                progress as f32,
            );
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "resolution").as_ref(),
                self.size.0 as f32,
                self.size.1 as f32,
            );
            let (x, y) = self.effect.parameters();
            gl.uniform_2_f32(
                gl.get_uniform_location(self.program, "direction").as_ref(),
                x,
                y,
            );
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            let error = gl.get_error();
            if error != glow::NO_ERROR {
                return Err(format!("GLES draw failed: 0x{error:x}"));
            }
        }
        // EGL attaches, damages, and commits the wl_surface itself.
        context
            .api
            .swap_buffers(context.display, self.surface)
            .map_err(|e| e.to_string())
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        let context = &self.device.context;
        if context.current(None).is_ok() {
            // SAFETY: owning context is current; each object is owned once.
            unsafe {
                let gl = &self.device.gl;
                gl.delete_buffer(self.vertices);
                gl.delete_texture(self.texture);
                gl.delete_program(self.program);
            }
        }
        let _ = context.api.destroy_surface(context.display, self.surface);
        // Native window is dropped next, before the Wayland layer is destroyed.
    }
}
