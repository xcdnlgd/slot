use crate::draw::{Draw, Sprites, TexId};
use crate::external::{ExternalProgram, ShaderSource};
use crate::grade::{blue_light_gain, contrast_boost, dim_gain, saturation_boost};
use crate::pipeline::GamePass;
use crate::quad::Quad;
use crate::shaders::{BLIT_FRAG, BLIT_VERT, TURN_VERT};
use crate::surface::{
    blit_panel_rect, blit_rect, GfxError, Surface, DEFAULT_PANEL_H, DEFAULT_PANEL_W,
};
use crate::Canvas;

pub const BACKDROP: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

struct Turn {
    panel: (u32, u32),
    prog: gl::types::GLuint,
    fbo: gl::types::GLuint,
    tex: gl::types::GLuint,
    size: (u32, u32),
}

pub struct Compositor {
    fbo: gl::types::GLuint,
    tex: gl::types::GLuint,
    size: (u32, u32),
    blit: gl::types::GLuint,
    u_gain: gl::types::GLint,
    u_saturation: gl::types::GLint,
    u_contrast: gl::types::GLint,
    u_blit_uv: gl::types::GLint,
    gain: [f32; 3],
    dim: f32,
    saturation: f32,
    contrast: f32,
    shake: f32,
    window: (u32, u32),
    panel: Option<(u32, u32)>,
    screen: f32,
    quad: Quad,
    game: GamePass,
    sprites: Sprites,
    game_bezel: Option<TexId>,
    game_fbo: gl::types::GLuint,
    game_tex: gl::types::GLuint,
    game_tex_size: (u32, u32),
    game_size: (u32, u32),
    game_offset: (i32, i32),
    external: Option<ExternalProgram>,
    external_linear: bool,
    turn: Option<Turn>,
    captured: Option<(u32, u32, Vec<u8>)>,
}

impl Compositor {
    pub fn new(surface: &dyn Surface) -> Result<Self, GfxError> {
        crate::gl::load(surface);
        let blit = crate::shaders::program(BLIT_VERT, BLIT_FRAG)?;
        let tex = crate::gl::texture(
            DEFAULT_PANEL_W,
            DEFAULT_PANEL_H,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            None,
        );
        unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            if status != gl::FRAMEBUFFER_COMPLETE {
                gl::DeleteFramebuffers(1, &fbo);
                gl::DeleteTextures(1, &tex);
                gl::DeleteProgram(blit);
                return Err(GfxError::Framebuffer(status));
            }
            gl::UseProgram(blit);
            gl::Uniform1i(crate::gl::uniform_location(blit, "u_tex"), 0);
            let u_gain = crate::gl::uniform_location(blit, "u_gain");
            let u_saturation = crate::gl::uniform_location(blit, "u_saturation");
            let u_contrast = crate::gl::uniform_location(blit, "u_contrast");
            let u_blit_uv = crate::gl::uniform_location(blit, "u_uv");
            gl::Uniform4f(u_blit_uv, 0.0, 0.0, 1.0, 1.0);

            Ok(Compositor {
                fbo,
                tex,
                size: (DEFAULT_PANEL_W, DEFAULT_PANEL_H),
                blit,
                u_gain,
                u_saturation,
                u_contrast,
                u_blit_uv,
                gain: blue_light_gain(0),
                dim: dim_gain(0),
                saturation: saturation_boost(0),
                contrast: contrast_boost(0),
                shake: 0.0,
                window: (DEFAULT_PANEL_W, DEFAULT_PANEL_H),
                panel: None,
                screen: 0.0,
                quad: Quad::new(),
                game: GamePass::new()?,
                sprites: Sprites::new()?,
                game_bezel: None,
                game_fbo: 0,
                game_tex: 0,
                game_tex_size: (0, 0),
                game_size: (DEFAULT_PANEL_W, DEFAULT_PANEL_H),
                game_offset: (0, 0),
                external: None,
                external_linear: false,
                turn: None,
                captured: None,
            })
        }
    }

    pub fn set_panel_turn(&mut self, panel: (u32, u32), ccw: bool) -> Result<(), GfxError> {
        let prog = crate::shaders::program(TURN_VERT, BLIT_FRAG)?;
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_tex"), 0);
            gl::Uniform3f(crate::gl::uniform_location(prog, "u_gain"), 1.0, 1.0, 1.0);
            gl::Uniform1f(
                crate::gl::uniform_location(prog, "u_ccw"),
                if ccw { 1.0 } else { 0.0 },
            );
        }
        self.turn = Some(Turn {
            panel,
            prog,
            fbo: 0,
            tex: 0,
            size: (0, 0),
        });
        Ok(())
    }

    pub fn upload_game(&mut self, xrgb8888: &[u8]) {
        self.game.upload(xrgb8888);
    }

    pub fn upload_game_sized(&mut self, xrgb8888: &[u8], size: (u32, u32)) {
        self.game.upload_sized(xrgb8888, size);
    }

    pub fn draw_game(&mut self) {
        self.with_game_rect(|c| c.draw_game_source());
    }

    pub fn draw_list(&mut self, items: &[Draw]) {
        let mut from = 0;
        for (i, item) in items.iter().enumerate() {
            match *item {
                Draw::Game => {
                    self.sprites.draw(&items[from..i], &self.quad);
                    self.with_game_rect(|c| c.draw_game_source());
                    self.draw_game_bezel();
                }
                Draw::Shot { tex } => {
                    self.sprites.draw(&items[from..i], &self.quad);
                    if let Some(tex) = self.sprites.source(tex) {
                        self.with_game_rect(move |c| c.draw_still_source(tex));
                    }
                }
                Draw::Capture => {
                    self.sprites.draw(&items[from..i], &self.quad);
                    let (w, h) = self.size;
                    self.captured = Some((w, h, self.read_frame()));
                }
                _ => continue,
            }
            from = i + 1;
        }
        self.sprites.draw(&items[from..], &self.quad);
    }

    pub fn create_texture(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        self.sprites.create_texture(w, h, rgba)
    }

    pub fn create_texture_nearest(&mut self, w: u32, h: u32, rgba: &[u8]) -> TexId {
        self.sprites.create_texture_nearest(w, h, rgba)
    }

    pub fn prepare_paper(&mut self) -> Result<(), GfxError> {
        self.sprites.prepare_paper()
    }

    pub fn update_texture(&mut self, id: TexId, w: u32, h: u32, rgba: &[u8]) {
        self.sprites.update_texture(id, w, h, rgba);
    }

    pub fn set_blue_light(&mut self, step: u8) {
        self.gain = blue_light_gain(step);
    }

    pub fn set_dim(&mut self, step: u8) {
        self.dim = dim_gain(step);
    }

    pub fn set_saturation(&mut self, step: u8) {
        self.saturation = saturation_boost(step);
    }

    pub fn set_contrast(&mut self, step: u8) {
        self.contrast = contrast_boost(step);
    }

    pub fn set_picture(&mut self, rect: [f32; 4]) {
        self.game.set_picture(rect);
    }

    pub fn set_game_bezel(&mut self, tex: Option<TexId>) {
        self.game_bezel = tex;
    }

    pub fn set_game_size(&mut self, size: crate::GameSize) {
        self.game_size = (size.w, size.h);
    }

    pub fn set_game_offset(&mut self, offset: (i32, i32)) {
        self.game_offset = offset;
    }

    pub fn set_shader_source(&mut self, source: Option<ShaderSource>) -> Result<(), GfxError> {
        self.external = match &source {
            Some(source) => Some(ExternalProgram::new(source)?),
            None => None,
        };
        self.external_linear = source.is_some_and(|s| s.linear);
        Ok(())
    }

    fn draw_game_bezel(&mut self) {
        let Some(tex) = self.game_bezel else {
            return;
        };
        let canvas = Canvas::of(self.panel_size(self.window));
        self.sprites.set_target(canvas);
        self.sprites.draw(
            &[Draw::Tex {
                x: 0.0,
                y: 0.0,
                w: canvas.w,
                h: canvas.h,
                tex,
                alpha: 1.0,
            }],
            &self.quad,
        );
    }

    pub fn set_paper(&mut self, size: u32, gray: &[u8]) {
        self.game.set_paper(size, gray);
    }

    pub fn fit(&mut self, window: (u32, u32)) {
        self.window = window;
        self.resize(self.panel.unwrap_or_else(|| canvas_size(window)));
    }

    pub fn set_panel(&mut self, panel: Option<(u32, u32)>) {
        self.panel = panel;
        if let Some(panel) = panel {
            self.resize(panel);
        }
    }

    pub fn panel_size(&self, window: (u32, u32)) -> (u32, u32) {
        self.panel.unwrap_or(window)
    }

    fn native(&self) -> bool {
        crate::bezel::framed(self.panel_size(self.window))
    }

    fn resize(&mut self, size: (u32, u32)) {
        if size == self.size {
            return;
        }
        let tex = crate::gl::texture(
            size.0,
            size.1,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            None,
        );
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::DeleteTextures(1, &self.tex);
        }
        self.tex = tex;
        self.size = size;
    }

    pub fn set_screen_power(&mut self, t: f32) {
        self.screen = t;
        self.game.set_power(t);
    }

    pub fn set_screen_footer(&mut self, px: f32) {
        self.game.set_bottom(px);
    }

    pub fn set_game_fills_frame(&mut self, on: bool) {
        self.game.set_fills_frame(on);
    }

    pub fn set_game_source_rect(&mut self, rect: [f32; 4]) {
        self.game.set_source_rect(rect);
    }

    pub fn set_shake(&mut self, dx: f32) {
        self.shake = dx;
    }

    pub fn take_capture(&mut self) -> Option<(u32, u32, Vec<u8>)> {
        self.captured.take()
    }

    pub fn frame_size(&self) -> (u32, u32) {
        self.size
    }

    pub fn read_frame(&self) -> Vec<u8> {
        let (w, h) = self.size;
        let stride = w as usize * 4;
        let mut buf = vec![0u8; stride * h as usize];
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::ReadPixels(
                0,
                0,
                w as i32,
                h as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
            );
        }
        let mut top_down = Vec::with_capacity(buf.len());
        for row in buf.chunks_exact(stride).rev() {
            top_down.extend_from_slice(row);
        }
        top_down
    }

    pub fn read_window(&mut self, window: (u32, u32)) -> Vec<u8> {
        let (w, h) = window;
        let tex = crate::gl::texture(w, h, gl::NEAREST, gl::CLAMP_TO_EDGE, gl::RGBA, None);
        let mut fbo = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );
        }
        self.output(window, fbo);
        let stride = w as usize * 4;
        let mut buf = vec![0u8; stride * h as usize];
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::PixelStorei(gl::PACK_ALIGNMENT, 1);
            gl::ReadPixels(
                0,
                0,
                w as i32,
                h as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
            );
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::DeleteFramebuffers(1, &fbo);
            gl::DeleteTextures(1, &tex);
        }
        buf.chunks_exact(stride).rev().flatten().copied().collect()
    }

    pub fn begin_frame(&mut self) {
        let size = self.panel.unwrap_or_else(|| {
            if self.native() {
                self.window
            } else {
                canvas_size(self.window)
            }
        });
        self.resize(size);
        self.sprites
            .set_target(Canvas::of(self.panel_size(self.window)));
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::Viewport(0, 0, self.size.0 as i32, self.size.1 as i32);
            gl::ClearColor(BACKDROP[0], BACKDROP[1], BACKDROP[2], BACKDROP[3]);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
    }

    pub fn end_frame(&mut self, window: (u32, u32)) {
        let Some(mut turn) = self.turn.take() else {
            self.output(window, 0);
            return;
        };
        if turn.size != window {
            unsafe {
                if turn.fbo != 0 {
                    gl::DeleteFramebuffers(1, &turn.fbo);
                    gl::DeleteTextures(1, &turn.tex);
                }
                turn.tex = crate::gl::texture(
                    window.0,
                    window.1,
                    gl::NEAREST,
                    gl::CLAMP_TO_EDGE,
                    gl::RGBA,
                    None,
                );
                gl::GenFramebuffers(1, &mut turn.fbo);
                gl::BindFramebuffer(gl::FRAMEBUFFER, turn.fbo);
                gl::FramebufferTexture2D(
                    gl::FRAMEBUFFER,
                    gl::COLOR_ATTACHMENT0,
                    gl::TEXTURE_2D,
                    turn.tex,
                    0,
                );
            }
            turn.size = window;
        }
        self.output(window, turn.fbo);
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
            gl::Viewport(0, 0, turn.panel.0 as i32, turn.panel.1 as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(turn.prog);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, turn.tex);
        }
        self.quad.draw();
        self.turn = Some(turn);
    }

    fn output(&mut self, window: (u32, u32), target: gl::types::GLuint) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, target);
            gl::Viewport(0, 0, window.0 as i32, window.1 as i32);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::UseProgram(self.blit);
            gl::ActiveTexture(gl::TEXTURE0);
            let [r, g, b] = self.gain.map(|g| g * self.dim);
            gl::Uniform3f(self.u_gain, r, g, b);
            gl::Uniform1f(self.u_saturation, self.saturation);
            gl::Uniform1f(self.u_contrast, self.contrast);
            gl::BindTexture(gl::TEXTURE_2D, self.tex);
        }
        if let Some(panel) = self.panel {
            let (x, y, w, h) = blit_panel_rect(panel, window, self.shake);
            self.blit_part(x, y, w, h, [0.0, 0.0, 1.0, 1.0]);
            return;
        }
        if self.native() {
            let canvas = Canvas::of(window);
            let dx = (self.shake * window.0 as f32 / canvas.w).round() as i32;
            self.blit_part(
                dx,
                0,
                window.0 as i32,
                window.1 as i32,
                [0.0, 0.0, 1.0, 1.0],
            );
            return;
        }
        let (x, y, w, h) = blit_rect(window, self.shake);
        self.blit_part(x, y, w, h, [0.0, 0.0, 1.0, 1.0]);
    }
}

impl Compositor {
    fn game_rect_px(&self) -> (i32, i32, i32, i32) {
        let (w, h) = (self.game_size.0 as i32, self.game_size.1 as i32);
        let max_x = (self.size.0 as i32 - w).max(0);
        let max_y = (self.size.1 as i32 - h).max(0);
        (
            self.game_offset.0.clamp(0, max_x),
            self.game_offset.1.clamp(0, max_y),
            w,
            h,
        )
    }

    fn draw_game_source(&mut self) {
        if self.external.is_some() {
            self.draw_shader_with(self.game.texture());
        } else {
            self.game.draw(&self.quad);
        }
    }

    fn draw_still_source(&mut self, texture: gl::types::GLuint) {
        if self.external.is_some() {
            self.draw_shader_with(texture);
        } else {
            self.game.draw_still(texture, &self.quad);
        }
    }

    fn draw_shader_with(&mut self, texture: gl::types::GLuint) {
        let (_, _, w, h) = self.game_rect_px();
        let output = (w as u32, h as u32);
        self.game.set_linear(self.external_linear);
        if let Some(program) = &self.external {
            unsafe { gl::Viewport(0, 0, w, h) };
            let frame = self.game.shader_frame(output);
            program.draw(texture, &frame, 0);
        }
        self.game.set_linear(false);
    }

    fn ensure_game_rect(&mut self, size: (u32, u32)) {
        if self.game_tex_size == size {
            return;
        }
        let tex = crate::gl::texture(
            size.0,
            size.1,
            gl::NEAREST,
            gl::CLAMP_TO_EDGE,
            gl::RGBA,
            None,
        );
        unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(
                gl::FRAMEBUFFER,
                gl::COLOR_ATTACHMENT0,
                gl::TEXTURE_2D,
                tex,
                0,
            );
            let status = gl::CheckFramebufferStatus(gl::FRAMEBUFFER);
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            if self.game_fbo != 0 {
                gl::DeleteFramebuffers(1, &self.game_fbo);
                gl::DeleteTextures(1, &self.game_tex);
            }
            if status != gl::FRAMEBUFFER_COMPLETE {
                gl::DeleteFramebuffers(1, &fbo);
                gl::DeleteTextures(1, &tex);
                self.game_fbo = 0;
                self.game_tex = 0;
                self.game_tex_size = (0, 0);
                return;
            }
            self.game_fbo = fbo;
            self.game_tex = tex;
            self.game_tex_size = size;
        }
    }

    fn with_game_rect(&mut self, draw: impl FnOnce(&mut Self)) {
        let (x, top, w, h) = self.game_rect_px();
        if w <= 0 || h <= 0 {
            return;
        }
        let size = (w as u32, h as u32);
        self.ensure_game_rect(size);
        if self.game_fbo == 0 {
            return;
        }
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.game_fbo);
            gl::Viewport(0, 0, w, h);
            gl::Disable(gl::BLEND);
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }
        self.game.set_target(w as f32, h as f32);
        self.game.set_origin(0.0, 0.0);
        draw(self);
        let y = self.size.1 as i32 - top - h;
        let bright = self.game.brightness();
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::Viewport(x, y, w, h);
            gl::Disable(gl::BLEND);
            gl::UseProgram(self.blit);
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, self.game_tex);
            gl::Uniform3f(self.u_gain, bright, bright, bright);
            gl::Uniform1f(self.u_saturation, 0.0);
            gl::Uniform1f(self.u_contrast, 0.0);
            gl::Uniform4f(self.u_blit_uv, 0.0, 0.0, 1.0, 1.0);
        }
        self.quad.draw();
        unsafe { gl::Viewport(0, 0, self.size.0 as i32, self.size.1 as i32) };
    }

    fn blit_part(&self, x: i32, y: i32, w: i32, h: i32, uv: [f32; 4]) {
        unsafe {
            gl::Viewport(x, y, w, h);
            gl::Uniform4f(self.u_blit_uv, uv[0], uv[1], uv[2], uv[3]);
        }
        self.quad.draw();
    }
}

pub fn canvas_size(window: (u32, u32)) -> (u32, u32) {
    let (_, _, w, h) = blit_rect(window, 0.0);
    let (w, h) = (w.max(1) as u32, h.max(1) as u32);
    if w % DEFAULT_PANEL_W == 0
        && h % DEFAULT_PANEL_H == 0
        && w / DEFAULT_PANEL_W == h / DEFAULT_PANEL_H
    {
        (DEFAULT_PANEL_W, DEFAULT_PANEL_H)
    } else {
        (w, h)
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo);
            gl::DeleteTextures(1, &self.tex);
            if self.game_fbo != 0 {
                gl::DeleteFramebuffers(1, &self.game_fbo);
                gl::DeleteTextures(1, &self.game_tex);
            }
            gl::DeleteProgram(self.blit);
            if let Some(turn) = &self.turn {
                gl::DeleteProgram(turn.prog);
                if turn.fbo != 0 {
                    gl::DeleteFramebuffers(1, &turn.fbo);
                    gl::DeleteTextures(1, &turn.tex);
                }
            }
        }
    }
}
