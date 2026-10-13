use super::{Texture, check, dimensions, mapping::Mapping};
use crate::renderer::gpu::{Device, program};
use crate::renderer::transition::Effect;
use glow::HasContext;
use std::rc::Rc;
use wallpicker_core::render::Geometry;

const VERTEX: &str = "attribute vec2 position; varying vec2 texcoord; void main() { texcoord = position * 0.5 + 0.5; gl_Position = vec4(position, 0.0, 1.0); }";

pub(in crate::renderer) struct Painter {
    pub(super) device: Rc<Device>,
    program: glow::Program,
    vertices: glow::Buffer,
    uniforms: Uniforms,
}

struct Uniforms {
    image: glow::UniformLocation,
    resolution: glow::UniformLocation,
    source_size: glow::UniformLocation,
    source_rect: glow::UniformLocation,
    destination_rect: glow::UniformLocation,
    flip: glow::UniformLocation,
    tiled: glow::UniformLocation,
    progress: glow::UniformLocation,
    effect_kind: glow::UniformLocation,
    direction: glow::UniformLocation,
}

fn fragment() -> String {
    let mut source = include_str!("image.frag").to_owned();
    source.push_str("\nuniform float progress; uniform vec2 direction;\n");
    // Share the established effect masks, sampling the GPU-mapped source image
    // instead of allocating a second, output-sized transition texture.
    for (name, shader) in [
        ("draw_wipe", include_str!("../../transition/wipe.frag")),
        ("draw_slide", include_str!("../../transition/slide.frag")),
        ("draw_grow", include_str!("../../transition/grow.frag")),
        ("draw_shrink", include_str!("../../transition/shrink.frag")),
        ("draw_wave", include_str!("../../transition/wave.frag")),
    ] {
        source.push_str(&format!("#define effect {name}\n{shader}\n#undef effect\n"));
    }
    source.push_str(include_str!("effect.frag"));
    source
}

impl Painter {
    /// One shader/quad can draw every output on this EGL context.
    pub(in crate::renderer) fn new(device: Rc<Device>) -> Result<Rc<Self>, String> {
        device.context.current(None)?;
        let gl = &device.gl;
        let program = program::compile_sources(gl, VERTEX, &fragment())?;
        // SAFETY: scratch context current; clean up every partial allocation.
        let (vertices, uniforms) = unsafe {
            let vertices = match gl.create_buffer() {
                Ok(buffer) => buffer,
                Err(e) => {
                    gl.delete_program(program);
                    return Err(e);
                }
            };
            let setup = || -> Result<Uniforms, String> {
                let positions: [f32; 8] = [-1., -1., 1., -1., -1., 1., 1., 1.];
                let bytes = std::slice::from_raw_parts(positions.as_ptr().cast(), std::mem::size_of_val(&positions));
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(vertices));
                gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
                let uniform = |name: &str| {
                    gl.get_uniform_location(program, name)
                        .ok_or_else(|| format!("missing playback uniform {name}"))
                };
                let uniforms = Uniforms {
                    image: uniform("image")?,
                    resolution: uniform("resolution")?,
                    source_size: uniform("source_size")?,
                    source_rect: uniform("source_rect")?,
                    destination_rect: uniform("destination_rect")?,
                    flip: uniform("flip")?,
                    tiled: uniform("tiled")?,
                    progress: uniform("progress")?,
                    effect_kind: uniform("effect_kind")?,
                    direction: uniform("direction")?,
                };
                check(gl, "playback program setup")?;
                Ok(uniforms)
            };
            match setup() {
                Ok(uniforms) => (vertices, uniforms),
                Err(error) => {
                    gl.delete_buffer(vertices);
                    gl.delete_program(program);
                    return Err(error);
                }
            }
        };
        Ok(Rc::new(Self {
            device,
            program,
            vertices,
            uniforms,
        }))
    }

    pub(super) fn validate_output(&self, size: (u32, u32)) -> Result<(), String> {
        dimensions(size)?;
        self.device.context.current(None)?;
        let mut maximum = [0; 2];
        // SAFETY: scratch context current; GLES returns two viewport limits.
        unsafe {
            self.device
                .gl
                .get_parameter_i32_slice(glow::MAX_VIEWPORT_DIMS, &mut maximum);
        }
        check(&self.device.gl, "viewport limit query")?;
        if maximum.iter().any(|n| *n <= 0) || size.0 > maximum[0] as u32 || size.1 > maximum[1] as u32 {
            return Err("output exceeds GLES viewport dimensions".into());
        }
        Ok(())
    }

    /// Caller makes its window/pbuffer current. No swap, resize, upload, buffer
    /// creation, uniform lookup or CPU image conversion occurs in this draw.
    pub(super) fn draw(&self, image: &Texture, size: (u32, u32), geometry: Geometry) -> Result<(), String> {
        self.draw_effect(image, size, geometry, Effect::Fade, 1.0)
    }

    pub(super) fn draw_effect(
        &self,
        image: &Texture,
        size: (u32, u32),
        geometry: Geometry,
        effect: Effect,
        progress: f64,
    ) -> Result<(), String> {
        if !Rc::ptr_eq(&self.device, &image.device) {
            return Err("playback texture belongs to another device".into());
        }
        if image.sequence().is_none() {
            return Err("playback texture has no frame".into());
        }
        let mapping = Mapping::new(image.size, size, geometry)?;
        let gl = &self.device.gl;
        let u = &self.uniforms;
        let kind = match effect {
            Effect::Fade => 0,
            Effect::Wipe => 1,
            Effect::Slide(_) => 2,
            Effect::Circle { shrinking: false, .. } => 3,
            Effect::Circle { shrinking: true, .. } => 4,
            Effect::Wave => 5,
        };
        let (x, y) = effect.parameters();
        // SAFETY: caller made this device's context current; these objects and
        // uniform locations belong to it and validated dimensions fit i32.
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, size.0 as i32, size.1 as i32);
            for capability in [glow::BLEND, glow::SCISSOR_TEST, glow::DEPTH_TEST, glow::CULL_FACE] {
                gl.disable(capability);
            }
            gl.color_mask(true, true, true, true);
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(image.id));
            gl.uniform_1_i32(Some(&u.image), 0);
            gl.uniform_2_f32(Some(&u.resolution), size.0 as f32, size.1 as f32);
            gl.uniform_2_f32(Some(&u.source_size), image.size.0 as f32, image.size.1 as f32);
            gl.uniform_4_f32_slice(Some(&u.source_rect), &mapping.source);
            gl.uniform_4_f32_slice(Some(&u.destination_rect), &mapping.destination);
            gl.uniform_2_f32(Some(&u.flip), mapping.flip[0], mapping.flip[1]);
            gl.uniform_1_i32(Some(&u.tiled), i32::from(mapping.tiled));
            gl.uniform_1_f32(Some(&u.progress), progress.clamp(0.0, 1.0) as f32);
            gl.uniform_1_i32(Some(&u.effect_kind), kind);
            gl.uniform_2_f32(Some(&u.direction), x, y);
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vertices));
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 0, 0);
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
        }
        check(gl, "playback draw")
    }
}

impl Drop for Painter {
    fn drop(&mut self) {
        if self.device.context.current(None).is_ok() {
            // SAFETY: owning context current; these resources are uniquely owned.
            unsafe {
                self.device.gl.use_program(None);
                self.device.gl.delete_buffer(self.vertices);
                self.device.gl.delete_program(self.program);
            }
        }
    }
}
