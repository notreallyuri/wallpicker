//! Shader compilation and resource ownership, with cleanup on failed links.

use glow::HasContext;

const VERTEX: &str = "attribute vec2 position; varying vec2 texcoord; void main() { texcoord = position * 0.5 + 0.5; gl_Position = vec4(position, 0.0, 1.0); }";
const PREFIX: &str = "precision highp float; varying vec2 texcoord; uniform sampler2D image; uniform float progress; uniform vec2 resolution; uniform vec2 direction; vec4 picture(vec2 uv) { return texture2D(image, uv); }";
const MAIN: &str = "void main() { vec2 uv = vec2(texcoord.x, 1.0 - texcoord.y); if (progress <= 0.0) { gl_FragColor = vec4(0.0); return; } if (progress >= 1.0) { gl_FragColor = texture2D(image, uv); return; } gl_FragColor = effect(uv); }";

pub(super) fn compile(gl: &glow::Context, effect: &str) -> Result<glow::Program, String> {
    compile_sources(gl, VERTEX, &format!("{PREFIX}\n{effect}\n{MAIN}"))
}

pub(super) fn compile_sources(gl: &glow::Context, vertex: &str, fragment: &str) -> Result<glow::Program, String> {
    // SAFETY: the caller has made the owning GLES context current.
    unsafe {
        let program = gl.create_program()?;
        for (kind, source) in [(glow::VERTEX_SHADER, vertex), (glow::FRAGMENT_SHADER, fragment)] {
            let shader = match gl.create_shader(kind) {
                Ok(s) => s,
                Err(e) => {
                    gl.delete_program(program);
                    return Err(e);
                }
            };
            gl.shader_source(shader, source);
            gl.compile_shader(shader);
            if !gl.get_shader_compile_status(shader) {
                let error = gl.get_shader_info_log(shader);
                gl.delete_shader(shader);
                gl.delete_program(program);
                return Err(error);
            }
            gl.attach_shader(program, shader);
            // GL retains attached shaders through linking, then frees them.
            gl.delete_shader(shader);
        }
        gl.bind_attrib_location(program, 0, "position");
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            let error = gl.get_program_info_log(program);
            gl.delete_program(program);
            return Err(error);
        }
        Ok(program)
    }
}
