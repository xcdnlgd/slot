use crate::external::ShaderFrame;
use crate::power::{screen_brightness, screen_rect_above};
use crate::quad::Quad;
use crate::shaders::{OFF_FRAG, RECT_VERT};
use crate::surface::GfxError;

pub const SRC_W: u32 = 240;
pub const SRC_H: u32 = 160;

pub const WHOLE_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

pub struct GamePass {
    prog: gl::types::GLuint,
    game: gl::types::GLuint,
    u_rect: gl::types::GLint,
    u_uv: gl::types::GLint,
    u_target: gl::types::GLint,
    origin: [f32; 2],
    pic: [f32; 4],
    paper: gl::types::GLuint,
    paper_size: f32,
    power: f32,
    fills: bool,
    /// Pixels kept for the carousel footer. The picture scales in the space above them.
    bottom: f32,
    src: [f32; 4],
    target: [f32; 2],
    frame: (u32, u32),
}

impl GamePass {
    pub fn new() -> Result<Self, GfxError> {
        let prog = crate::shaders::program(RECT_VERT, OFF_FRAG)?;
        let game = crate::gl::texture(SRC_W, SRC_H, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
        let paper = crate::gl::texture(
            1,
            1,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(&[255]),
        );
        let (u_rect, u_uv, u_target);
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_game"), 0);
            u_target = crate::gl::uniform_location(prog, "u_target");
            gl::Uniform2f(u_target, SRC_W as f32, SRC_H as f32);
            u_rect = crate::gl::uniform_location(prog, "u_rect");
            u_uv = crate::gl::uniform_location(prog, "u_uv");
        }
        Ok(GamePass {
            prog,
            game,
            u_rect,
            u_uv,
            u_target,
            origin: [0.0, 0.0],
            pic: WHOLE_TEXTURE,
            paper,
            paper_size: 1.0,
            power: 1.0,
            fills: false,
            bottom: 0.0,
            src: WHOLE_TEXTURE,
            target: [SRC_W as f32, SRC_H as f32],
            frame: (SRC_W, SRC_H),
        })
    }

    pub fn set_power(&mut self, t: f32) {
        self.power = t.clamp(0.0, 1.0);
    }

    pub fn set_bottom(&mut self, px: f32) {
        self.bottom = px.max(0.0);
    }

    pub fn frame(&self) -> (f32, f32, f32, f32) {
        let (width, height) = (self.target[0], self.target[1]);
        let foot = self.bottom;
        match self.fills {
            true => (0.0, 0.0, width, height - foot),
            false => screen_rect_above(width, height, self.power, foot),
        }
    }

    pub fn set_target(&mut self, width: f32, height: f32) {
        self.target = [width, height];
    }

    pub fn set_fills_frame(&mut self, on: bool) {
        self.fills = on;
    }

    pub fn set_picture(&mut self, rect: [f32; 4]) {
        self.pic = rect;
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin = [x, y];
    }

    pub fn set_paper(&mut self, size: u32, gray: &[u8]) {
        let tex = crate::gl::texture(
            size,
            size,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(gray),
        );
        unsafe { gl::DeleteTextures(1, &self.paper) };
        self.paper = tex;
        self.paper_size = size as f32;
    }

    pub fn set_source_rect(&mut self, rect: [f32; 4]) {
        self.src = rect;
    }

    pub fn texture(&self) -> gl::types::GLuint {
        self.game
    }

    pub fn brightness(&self) -> f32 {
        if self.fills {
            1.0
        } else {
            screen_brightness(self.power)
        }
    }

    pub fn set_linear(&self, linear: bool) {
        let filter = if linear { gl::LINEAR } else { gl::NEAREST };
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.game);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter as i32);
        }
    }

    pub fn upload(&mut self, xrgb8888: &[u8]) {
        self.upload_sized(xrgb8888, (SRC_W, SRC_H));
    }

    pub fn upload_sized(&mut self, xrgb8888: &[u8], size: (u32, u32)) {
        let (width, height) = size;
        if width == 0 || height == 0 || xrgb8888.len() < (width * height * 4) as usize {
            return;
        }
        if size != self.frame {
            let game = crate::gl::texture(
                width,
                height,
                gl::NEAREST,
                gl::CLAMP_TO_EDGE,
                gl::BGRA,
                None,
            );
            unsafe {
                gl::DeleteTextures(1, &self.game);
            }
            self.game = game;
            self.frame = size;
        }
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.game);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                width as i32,
                height as i32,
                gl::BGRA,
                gl::UNSIGNED_BYTE,
                xrgb8888.as_ptr() as *const std::ffi::c_void,
            );
        }
    }

    pub fn shader_frame(&self, output: (u32, u32)) -> ShaderFrame {
        let (x, y, w, h) = self.frame();
        ShaderFrame {
            texture: self.frame,
            output,
            src: self.trimmed_source(),
            rect: [x, y, w, h],
            paper: self.paper,
            paper_size: self.paper_size,
            origin: self.origin,
            pic: self.pic,
        }
    }

    pub fn draw(&self, quad: &Quad) {
        self.draw_source(self.game, quad);
    }

    pub fn draw_still(&self, tex: gl::types::GLuint, quad: &Quad) {
        self.draw_source(tex, quad);
    }

    fn trimmed_source(&self) -> [f32; 4] {
        let mut src = self.src;
        // The footer covers the bottom of a full-frame picture. Keep the part that sat above
        // it, so opening the scale does not squash the picture into the shorter rect.
        if self.bottom > 0.0 {
            src[3] *= (self.target[1] - self.bottom) / self.target[1];
        }
        src
    }

    fn draw_source(&self, tex: gl::types::GLuint, quad: &Quad) {
        let (x, y, w, h) = self.frame();
        let src = self.trimmed_source();
        unsafe {
            gl::UseProgram(self.prog);
            gl::Uniform2f(self.u_target, self.target[0], self.target[1]);
            gl::Uniform4f(self.u_rect, x, y, w, h);
            gl::Uniform4f(self.u_uv, src[0], src[1], src[2], src[3]);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
        }
        quad.draw();
    }
}

impl Drop for GamePass {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.game);
            gl::DeleteTextures(1, &self.paper);
            gl::DeleteProgram(self.prog);
        }
    }
}
