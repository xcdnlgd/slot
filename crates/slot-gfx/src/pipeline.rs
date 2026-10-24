use crate::lcd3x::ScreenEffect;
use crate::power::{screen_brightness, screen_rect_above};
use crate::quad::Quad;
use crate::shaders::{GAME_FRAG, RECT_VERT};
use crate::surface::{GfxError, OUT_H, OUT_W};

pub const SCALE: u32 = 3;
pub const SRC_W: u32 = OUT_W / SCALE;
pub const SRC_H: u32 = OUT_H / SCALE;

pub const WHOLE_TEXTURE: [f32; 4] = [0.0, 0.0, 1.0, 1.0];

pub struct GamePass {
    prog: gl::types::GLuint,
    game: gl::types::GLuint,
    u_rect: gl::types::GLint,
    u_bright: gl::types::GLint,
    u_uv: gl::types::GLint,
    u_src: gl::types::GLint,
    u_mode: gl::types::GLint,
    u_paper_size: gl::types::GLint,
    u_fbo: gl::types::GLint,
    fbo_scale: [f32; 2],
    u_origin: gl::types::GLint,
    origin: [f32; 2],
    u_pic: gl::types::GLint,
    pic: [f32; 4],
    paper: gl::types::GLuint,
    paper_size: f32,
    effect: ScreenEffect,
    power: f32,
    fills: bool,
    /// Pixels kept for the carousel footer. The picture scales in the space above them.
    bottom: f32,
    src: [f32; 4],
    frame: (u32, u32),
}

impl GamePass {
    pub fn new() -> Result<Self, GfxError> {
        let prog = crate::shaders::program(RECT_VERT, GAME_FRAG)?;
        let game = crate::gl::texture(SRC_W, SRC_H, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::BGRA, None);
        let paper = crate::gl::texture(
            1,
            1,
            gl::NEAREST,
            gl::REPEAT,
            crate::gl::gray_format(),
            Some(&[255]),
        );
        let (u_rect, u_bright, u_uv, u_src, u_mode, u_paper_size, u_pic, u_fbo, u_origin);
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_game"), 0);
            u_src = crate::gl::uniform_location(prog, "u_src");
            gl::Uniform2f(u_src, SRC_W as f32, SRC_H as f32);
            gl::Uniform2f(
                crate::gl::uniform_location(prog, "u_target"),
                OUT_W as f32,
                OUT_H as f32,
            );
            u_rect = crate::gl::uniform_location(prog, "u_rect");
            u_bright = crate::gl::uniform_location(prog, "u_bright");
            u_uv = crate::gl::uniform_location(prog, "u_uv");
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_paper"), 1);
            u_mode = crate::gl::uniform_location(prog, "u_mode");
            u_paper_size = crate::gl::uniform_location(prog, "u_paper_size");
            u_fbo = crate::gl::uniform_location(prog, "u_fbo");
            u_pic = crate::gl::uniform_location(prog, "u_pic");
            u_origin = crate::gl::uniform_location(prog, "u_origin");
        }
        Ok(GamePass {
            prog,
            game,
            u_rect,
            u_bright,
            u_uv,
            u_src,
            u_mode,
            u_paper_size,
            u_fbo,
            fbo_scale: [1.0, 1.0],
            u_origin,
            origin: [0.0, 0.0],
            u_pic,
            pic: WHOLE_TEXTURE,
            paper,
            paper_size: 1.0,
            effect: ScreenEffect::Lcd3x,
            power: 1.0,
            fills: false,
            bottom: 0.0,
            src: WHOLE_TEXTURE,
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
        self.placed()
    }

    pub fn frame_size(&self) -> (u32, u32) {
        self.frame
    }

    fn placed(&self) -> (f32, f32, f32, f32) {
        let foot = self.bottom;
        let base = match self.fills {
            true => (0.0, 0.0, OUT_W as f32, OUT_H as f32 - foot),
            false => screen_rect_above(self.power, foot),
        };
        if self.frame == (SRC_W, SRC_H) {
            return base;
        }
        content_rect(base, self.frame, self.src != WHOLE_TEXTURE)
    }

    pub fn set_fills_frame(&mut self, on: bool) {
        self.fills = on;
    }

    pub fn set_picture(&mut self, rect: [f32; 4]) {
        self.pic = rect;
    }

    pub fn set_fbo_scale(&mut self, x: f32, y: f32) {
        self.fbo_scale = [x, y];
    }

    pub fn set_origin(&mut self, x: f32, y: f32) {
        self.origin = [x, y];
    }

    pub fn set_effect(&mut self, effect: ScreenEffect) {
        self.effect = effect;
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
        if width == 0 || height == 0 {
            return;
        }
        let len = (width * height * 4) as usize;
        if xrgb8888.len() < len {
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
                gl::UseProgram(self.prog);
                gl::Uniform2f(self.u_src, width as f32, height as f32);
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

    pub fn draw(&self, quad: &Quad) {
        let src = match self.frame != (SRC_W, SRC_H) {
            true => WHOLE_TEXTURE,
            false => match self.effect {
                ScreenEffect::Grid => whole_scale_source(self.src, self.fbo_scale),
                _ => self.src,
            },
        };
        self.draw_source(self.game, quad, src);
    }

    pub fn draw_still(&self, tex: gl::types::GLuint, quad: &Quad) {
        self.draw_source(tex, quad, WHOLE_TEXTURE);
    }

    fn draw_source(&self, tex: gl::types::GLuint, quad: &Quad, src: [f32; 4]) {
        let (x, y, w, h) = self.placed();
        let bright = if self.fills {
            1.0
        } else {
            screen_brightness(self.power)
        };
        // The footer covers the bottom of a full-frame picture. Keep the part that sat above
        // it, so opening the scale does not squash the picture into the shorter rect.
        let mut src = src;
        if self.bottom > 0.0 {
            src[3] *= (OUT_H as f32 - self.bottom) / OUT_H as f32;
        }
        unsafe {
            gl::UseProgram(self.prog);
            gl::Uniform4f(self.u_rect, x, y, w, h);
            gl::Uniform4f(self.u_uv, src[0], src[1], src[2], src[3]);
            gl::Uniform1f(self.u_bright, bright);
            gl::Uniform1f(self.u_mode, self.effect.mode());
            gl::Uniform1f(self.u_paper_size, self.paper_size);
            gl::Uniform2f(self.u_fbo, self.fbo_scale[0], self.fbo_scale[1]);
            gl::Uniform2f(self.u_origin, self.origin[0], self.origin[1]);
            gl::Uniform4f(
                self.u_pic,
                self.pic[0],
                self.pic[1],
                self.pic[2],
                self.pic[3],
            );
            gl::ActiveTexture(gl::TEXTURE1);
            gl::BindTexture(gl::TEXTURE_2D, self.paper);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, tex);
        }
        quad.draw();
    }
}

fn content_rect(
    base: (f32, f32, f32, f32),
    frame: (u32, u32),
    stretch: bool,
) -> (f32, f32, f32, f32) {
    if frame == (SRC_W, SRC_H) {
        return base;
    }
    let (x, y, w, h) = base;
    let scale = (w / SRC_W as f32).min(h / SRC_H as f32);
    let shown_w = match stretch {
        true => SRC_W as f32,
        false => frame.0 as f32,
    } * scale;
    let shown_h = frame.1 as f32 * scale;
    (
        x + (w - shown_w) / 2.0,
        y + (h - shown_h) / 2.0,
        shown_w,
        shown_h,
    )
}

fn whole_scale_source(src: [f32; 4], fbo: [f32; 2]) -> [f32; 4] {
    let (x, w) = whole_scale(src[0], src[2], SRC_W as f32, OUT_W as f32 * fbo[0]);
    let (y, h) = whole_scale(src[1], src[3], SRC_H as f32, OUT_H as f32 * fbo[1]);
    [x, y, w, h]
}

fn whole_scale(at: f32, len: f32, texels: f32, out: f32) -> (f32, f32) {
    let shown = len * texels;
    let scale = (out / shown).ceil();
    let trim = scale * shown - out;
    if trim >= 0.1 * shown {
        return (at, len);
    }
    let top = (trim / 2.0).floor();
    (at + top / scale / texels, out / scale / texels)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_game_boy_frame_is_three_times_its_size_on_a_720_panel() {
        let rect = content_rect((0.0, 0.0, 720.0, 480.0), (160, 144), false);
        assert_eq!(rect, (120.0, 24.0, 480.0, 432.0));
    }

    #[test]
    fn stretching_a_game_boy_frame_uses_the_gba_width() {
        let rect = content_rect((0.0, 0.0, 720.0, 480.0), (160, 144), true);
        assert_eq!(rect, (0.0, 24.0, 720.0, 432.0));
    }

    #[test]
    fn a_game_boy_frame_keeps_the_old_size_on_a_640_panel() {
        let rect = content_rect((0.0, 27.0, 640.0, 427.0), (160, 144), false);
        assert!((rect.0 - 106.666_67).abs() < 0.01, "{rect:?}");
        assert!((rect.1 - 48.5).abs() < 0.01, "{rect:?}");
        assert!((rect.2 - 426.666_66).abs() < 0.01, "{rect:?}");
        assert!((rect.3 - 384.0).abs() < 0.01, "{rect:?}");
    }

    #[test]
    fn a_gba_frame_uses_the_whole_game_rect() {
        let base = (0.0, 27.0, 640.0, 427.0);
        assert_eq!(content_rect(base, (SRC_W, SRC_H), false), base);
    }
}
