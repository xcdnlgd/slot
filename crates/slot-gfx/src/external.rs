use std::ffi::c_void;
use std::path::Path;

use crate::shaders;
use crate::surface::GfxError;

#[derive(Clone, Debug)]
pub struct ShaderSource {
    pub text: String,
    pub linear: bool,
}

/// Load a RetroArch single-pass `.glsl` or `.glslp`. Multi-pass presets are
/// rejected until the renderer can chain passes.
pub fn load_shader(path: &Path) -> Result<ShaderSource, String> {
    let is_preset = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("glslp"));
    if !is_preset {
        let text = read_source(path)?;
        return Ok(ShaderSource { text, linear: true });
    }

    let preset = read_text(path)?;
    let mut passes = 1usize;
    let mut shader = None;
    let mut linear = false;
    for line in preset.lines() {
        let line = line.split(['#', ';']).next().unwrap_or("").trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "shaders" => {
                passes = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("{}: bad shader count", path.display()))?;
            }
            "shader0" => shader = Some(value.trim().to_string()),
            "filter_linear0" => linear = matches!(value.trim(), "1" | "true" | "yes"),
            key if key.starts_with("shader") || key.starts_with("filter_") => {}
            _ => {}
        }
    }
    if passes != 1 {
        return Err(format!(
            "{}: {passes} passes are not supported yet",
            path.display()
        ));
    }
    let shader = shader.ok_or_else(|| format!("{}: no shader0", path.display()))?;
    let source = path.parent().unwrap_or_else(|| Path::new(".")).join(shader);
    let text = read_source(&source)?;
    Ok(ShaderSource { text, linear })
}

fn read_source(path: &Path) -> Result<String, String> {
    let text = read_text(path)?;
    let text = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("#pragma parameter"))
        .collect::<Vec<_>>()
        .join("\n");
    if !text.contains("#if defined(VERTEX)") || !text.contains("#elif defined(FRAGMENT)") {
        return Err(format!(
            "{}: not a RetroArch vertex/fragment GLSL file",
            path.display()
        ));
    }
    Ok(text)
}

fn read_text(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn rename_fragment_main(text: &str) -> String {
    let Some(at) = text.find("#elif defined(FRAGMENT)") else {
        return text.to_string();
    };
    let (head, tail) = text.split_at(at);
    let mut tail = tail.replacen("void main()", "void slot_external_main()", 1);
    if !tail.contains("slot_external_main") {
        tail = tail.replacen("void main(void)", "void slot_external_main(void)", 1);
    }
    format!("{head}{tail}")
}

#[derive(Clone, Copy)]
pub struct ShaderFrame {
    pub texture: (u32, u32),
    pub output: (u32, u32),
    pub src: [f32; 4],
    pub rect: [f32; 4],
    pub paper: gl::types::GLuint,
    pub paper_size: f32,
    pub origin: [f32; 2],
    pub pic: [f32; 4],
}

pub struct ExternalProgram {
    prog: gl::types::GLuint,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
    a_vertex: gl::types::GLint,
    a_tex: gl::types::GLint,
    a_colour: gl::types::GLint,
    u_mvp: gl::types::GLint,
    u_out: gl::types::GLint,
    u_tex_size: gl::types::GLint,
    u_in_size: gl::types::GLint,
    u_frame_dir: gl::types::GLint,
    u_frame_count: gl::types::GLint,
    u_texture: gl::types::GLint,
    u_src: gl::types::GLint,
    u_uv: gl::types::GLint,
    u_rect: gl::types::GLint,
    u_paper: gl::types::GLint,
    u_paper_size: gl::types::GLint,
    u_origin: gl::types::GLint,
    u_pic: gl::types::GLint,
}

fn fragment_source(text: &str) -> String {
    let frag_text = rename_fragment_main(text);
    format!(
        "#define FRAGMENT 1\n\
         #ifdef GL_ES\nprecision mediump float;\n#endif\n\
         {frag_text}\n\
         void main() {{ slot_external_main(); }}\n"
    )
}

impl ExternalProgram {
    pub fn new(source: &ShaderSource) -> Result<Self, GfxError> {
        let vert = format!("#define VERTEX 1\n{}", source.text);
        let frag = fragment_source(&source.text);
        let prog = shaders::program(&vert, &frag)?;
        let mut vao = 0;
        let mut vbo = 0;
        let mut a_vertex = -1;
        let mut a_tex = -1;
        let mut a_colour = -1;
        unsafe {
            if !crate::gl::es() {
                gl::GenVertexArrays(1, &mut vao);
                gl::BindVertexArray(vao);
            }
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            let vertices: [f32; 48] = [
                -1.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, // top left
                -1.0, -1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, // bottom left
                1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, // top right
                1.0, -1.0, 0.0, 1.0, 1.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, // bottom right
            ];
            gl::BufferData(
                gl::ARRAY_BUFFER,
                std::mem::size_of_val(&vertices) as isize,
                vertices.as_ptr() as *const c_void,
                gl::STATIC_DRAW,
            );
            for (name, slot) in [
                ("VertexCoord", &mut a_vertex),
                ("TexCoord", &mut a_tex),
                ("COLOR", &mut a_colour),
            ] {
                let Ok(name) = std::ffi::CString::new(name) else {
                    continue;
                };
                *slot = gl::GetAttribLocation(prog, name.as_ptr());
            }
            point_attributes(a_vertex, a_tex, a_colour);
            if !crate::gl::es() {
                gl::BindVertexArray(0);
            }
        }
        Ok(ExternalProgram {
            prog,
            vao,
            vbo,
            a_vertex,
            a_tex,
            a_colour,
            u_mvp: uniform(prog, "MVPMatrix"),
            u_out: uniform(prog, "OutputSize"),
            u_tex_size: uniform(prog, "TextureSize"),
            u_in_size: uniform(prog, "InputSize"),
            u_frame_dir: uniform(prog, "FrameDirection"),
            u_frame_count: uniform(prog, "FrameCount"),
            u_texture: uniform(prog, "Texture"),
            u_src: uniform(prog, "u_src"),
            u_uv: uniform(prog, "u_uv"),
            u_rect: uniform(prog, "u_rect"),
            u_paper: uniform(prog, "u_paper"),
            u_paper_size: uniform(prog, "u_paper_size"),
            u_origin: uniform(prog, "u_origin"),
            u_pic: uniform(prog, "u_pic"),
        })
    }

    pub fn draw(&self, texture: gl::types::GLuint, frame: &ShaderFrame, count: u64) {
        unsafe {
            gl::UseProgram(self.prog);
            if crate::gl::es() {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);
                point_attributes(self.a_vertex, self.a_tex, self.a_colour);
            } else {
                gl::BindVertexArray(self.vao);
            }
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, frame.paper);
            gl::ActiveTexture(gl::TEXTURE0);
            let mvp: [f32; 16] = [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ];
            if self.u_mvp >= 0 {
                gl::UniformMatrix4fv(self.u_mvp, 1, gl::FALSE, mvp.as_ptr());
            }
            if self.u_out >= 0 {
                gl::Uniform2f(self.u_out, frame.output.0 as f32, frame.output.1 as f32);
            }
            if self.u_tex_size >= 0 {
                gl::Uniform2f(
                    self.u_tex_size,
                    frame.texture.0 as f32,
                    frame.texture.1 as f32,
                );
            }
            if self.u_in_size >= 0 {
                gl::Uniform2f(
                    self.u_in_size,
                    frame.texture.0 as f32,
                    frame.texture.1 as f32,
                );
            }
            if self.u_frame_dir >= 0 {
                gl::Uniform1i(self.u_frame_dir, 1);
            }
            if self.u_frame_count >= 0 {
                gl::Uniform1i(self.u_frame_count, count as i32);
            }
            if self.u_texture >= 0 {
                gl::Uniform1i(self.u_texture, 0);
            }
            if self.u_src >= 0 {
                gl::Uniform2f(self.u_src, frame.texture.0 as f32, frame.texture.1 as f32);
            }
            if self.u_uv >= 0 {
                gl::Uniform4f(
                    self.u_uv,
                    frame.src[0],
                    frame.src[1],
                    frame.src[2],
                    frame.src[3],
                );
            }
            if self.u_rect >= 0 {
                gl::Uniform4f(
                    self.u_rect,
                    frame.rect[0],
                    frame.rect[1],
                    frame.rect[2],
                    frame.rect[3],
                );
            }
            if self.u_paper >= 0 {
                gl::Uniform1i(self.u_paper, 1);
            }
            if self.u_paper_size >= 0 {
                gl::Uniform1f(self.u_paper_size, frame.paper_size);
            }
            if self.u_origin >= 0 {
                gl::Uniform2f(self.u_origin, frame.origin[0], frame.origin[1]);
            }
            if self.u_pic >= 0 {
                gl::Uniform4f(
                    self.u_pic,
                    frame.pic[0],
                    frame.pic[1],
                    frame.pic[2],
                    frame.pic[3],
                );
            }
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
            if !crate::gl::es() {
                gl::BindVertexArray(0);
            }
        }
    }
}

fn point_attributes(vertex: gl::types::GLint, tex: gl::types::GLint, colour: gl::types::GLint) {
    let stride = 12 * std::mem::size_of::<f32>() as i32;
    for (loc, offset) in [(vertex, 0), (tex, 4), (colour, 8)] {
        if loc < 0 {
            continue;
        }
        unsafe {
            gl::EnableVertexAttribArray(loc as u32);
            gl::VertexAttribPointer(
                loc as u32,
                4,
                gl::FLOAT,
                gl::FALSE,
                stride,
                (offset * std::mem::size_of::<f32>() as i32) as *const c_void,
            );
        }
    }
}

fn uniform(prog: gl::types::GLuint, name: &str) -> gl::types::GLint {
    crate::gl::uniform_location(prog, name)
}

impl Drop for ExternalProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.prog);
            if !crate::gl::es() {
                gl::DeleteVertexArrays(1, &self.vao);
            }
            gl::DeleteBuffers(1, &self.vbo);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str =
        "#if defined(VERTEX)\nvoid main() {}\n#elif defined(FRAGMENT)\nvoid main() {}\n#endif\n";

    #[test]
    fn a_single_pass_preset_resolves_its_shader() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("shaders")).unwrap();
        std::fs::write(dir.path().join("shaders/foo.glsl"), SOURCE).unwrap();
        std::fs::write(
            dir.path().join("foo.glslp"),
            "shaders = 1\nshader0 = shaders/foo.glsl\nfilter_linear0 = true\n",
        )
        .unwrap();
        let shader = load_shader(&dir.path().join("foo.glslp")).unwrap();
        assert!(shader.linear);
        assert!(shader.text.contains("void main()"));
    }

    #[test]
    fn a_multi_pass_preset_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("foo.glslp"),
            "shaders = 2\nshader0 = shaders/a.glsl\nshader1 = shaders/b.glsl\n",
        )
        .unwrap();
        assert!(load_shader(&dir.path().join("foo.glslp"))
            .unwrap_err()
            .contains("2 passes"));
    }

    #[test]
    fn the_fragment_main_is_wrapped_for_brightness() {
        let renamed = rename_fragment_main(SOURCE);
        assert!(renamed.contains("void slot_external_main()"));
        assert_eq!(renamed.matches("void main()").count(), 1);
    }

    #[test]
    fn the_wrapper_declares_fragment_float_precision_on_gles() {
        let wrapped = fragment_source(SOURCE);
        assert!(wrapped.contains("#ifdef GL_ES\nprecision mediump float;\n#endif"));
    }
}
