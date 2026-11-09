use std::ffi::c_void;
use std::fmt;

pub const DEFAULT_PANEL_W: u32 = 720;
pub const DEFAULT_PANEL_H: u32 = 480;
pub const DEFAULT_PANEL: (u32, u32) = (DEFAULT_PANEL_W, DEFAULT_PANEL_H);
pub const PANEL_W: u32 = DEFAULT_PANEL_W;
pub const PANEL_H: u32 = DEFAULT_PANEL_H;

#[derive(Debug)]
pub enum GfxError {
    Context(String),
    Shader(String),
    Framebuffer(u32),
}

impl fmt::Display for GfxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GfxError::Context(m) => write!(f, "gl context: {m}"),
            GfxError::Shader(m) => write!(f, "shader: {m}"),
            GfxError::Framebuffer(s) => write!(f, "framebuffer incomplete: 0x{s:x}"),
        }
    }
}

impl std::error::Error for GfxError {}

pub trait Surface {
    fn make_current(&mut self) -> Result<(), GfxError>;
    fn window_size(&self) -> (u32, u32);
    fn swap(&mut self) -> Result<(), GfxError>;
    fn proc_address(&self, name: &str) -> *const c_void;
}

pub fn fit_scale(win_w: u32, win_h: u32) -> u32 {
    (win_w / DEFAULT_PANEL_W)
        .min(win_h / DEFAULT_PANEL_H)
        .max(1)
}

pub fn fit_rect(win_w: u32, win_h: u32) -> (i32, i32, i32, i32) {
    let s = fit_scale(win_w, win_h) as i32;
    let (w, h) = (DEFAULT_PANEL_W as i32 * s, DEFAULT_PANEL_H as i32 * s);
    ((win_w as i32 - w) / 2, (win_h as i32 - h) / 2, w, h)
}

pub fn blit_rect_fit(panel: (u32, u32), shake: f32) -> (i32, i32, i32, i32) {
    let scale =
        (panel.0 as f32 / DEFAULT_PANEL_W as f32).min(panel.1 as f32 / DEFAULT_PANEL_H as f32);
    let w = (DEFAULT_PANEL_W as f32 * scale).round() as i32;
    let h = (DEFAULT_PANEL_H as f32 * scale).round() as i32;
    let dx = (shake * scale).round() as i32;
    (
        (panel.0 as i32 - w) / 2 + dx,
        (panel.1 as i32 - h) / 2,
        w,
        h,
    )
}

/// A simulated device panel contained in the window at its own aspect ratio.
/// Whole multiples stay pixel-exact; other sizes scale to fill as much of the
/// window as the aspect allows.
pub fn blit_panel_rect(panel: (u32, u32), window: (u32, u32), shake: f32) -> (i32, i32, i32, i32) {
    let scale = (window.0 as f32 / panel.0 as f32).min(window.1 as f32 / panel.1 as f32);
    let w = (panel.0 as f32 * scale).round().max(1.0) as i32;
    let h = (panel.1 as f32 * scale).round().max(1.0) as i32;
    let dx = (shake * scale * panel.0 as f32 / DEFAULT_PANEL_W as f32).round() as i32;
    (
        (window.0 as i32 - w) / 2 + dx,
        (window.1 as i32 - h) / 2,
        w,
        h,
    )
}

pub fn blit_rect(window: (u32, u32), shake: f32) -> (i32, i32, i32, i32) {
    if crate::bezel::framed(window) {
        let dx = shake * window.0 as f32 / DEFAULT_PANEL_W as f32;
        return (dx.round() as i32, 0, window.0 as i32, window.1 as i32);
    }
    if window.0 < DEFAULT_PANEL_W || window.1 < DEFAULT_PANEL_H {
        return blit_rect_fit(window, shake);
    }
    let (x, y, w, h) = fit_rect(window.0, window.1);
    let dx = shake * fit_scale(window.0, window.1) as f32;
    (x + dx.round() as i32, y, w, h)
}
