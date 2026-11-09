use std::path::Path;
use std::sync::OnceLock;

use slot_gfx::{Canvas, Draw, TexId};

use crate::art;

const DEFAULT_PNG: &[u8] = include_bytes!("../../../assets/background_default.png");
const BEZEL_GB_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb.png");
const BEZEL_GBC_PNG: &[u8] = include_bytes!("../../../assets/bezel_gbc.png");
const BEZEL_GB_FIT_3X2_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb_fit_3x2.png");
const BEZEL_GBC_FIT_3X2_PNG: &[u8] = include_bytes!("../../../assets/bezel_gbc_fit_3x2.png");
const BEZEL_GB_INTEGER_4X3_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb_integer_4x3.png");
const BEZEL_GBC_INTEGER_4X3_PNG: &[u8] =
    include_bytes!("../../../assets/bezel_gbc_integer_4x3.png");
const BEZEL_GB_FIT_4X3_PNG: &[u8] = include_bytes!("../../../assets/bezel_gb_fit_4x3.png");
const BEZEL_GBC_FIT_4X3_PNG: &[u8] = include_bytes!("../../../assets/bezel_gbc_fit_4x3.png");
const BLUR_PNG: &[u8] = include_bytes!("../../../assets/background_default_blur.png");

const SCRIM: f32 = 0.62;

pub fn wallpaper_face(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    art::cover(path, w, h)
}

pub fn bezel_face(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    art::cover(path, w, h)
}

pub fn game_bezel_face(path: &Path, w: u32, h: u32) -> Option<Vec<u8>> {
    let (src, sw, sh) = art::decode(path)?;
    art::cover_rgba(&src, sw, sh, w, h)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BezelArt {
    Default,
    Fit3x2,
    Integer4x3,
    Fit4x3,
}

pub fn default_game_bezel_face(colour: bool, kind: BezelArt, w: u32, h: u32) -> Option<Vec<u8>> {
    let png = match (kind, colour) {
        (BezelArt::Default, false) => BEZEL_GB_PNG,
        (BezelArt::Default, true) => BEZEL_GBC_PNG,
        (BezelArt::Fit3x2, false) => BEZEL_GB_FIT_3X2_PNG,
        (BezelArt::Fit3x2, true) => BEZEL_GBC_FIT_3X2_PNG,
        (BezelArt::Integer4x3, false) => BEZEL_GB_INTEGER_4X3_PNG,
        (BezelArt::Integer4x3, true) => BEZEL_GBC_INTEGER_4X3_PNG,
        (BezelArt::Fit4x3, false) => BEZEL_GB_FIT_4X3_PNG,
        (BezelArt::Fit4x3, true) => BEZEL_GBC_FIT_4X3_PNG,
    };
    let (src, sw, sh) = art::decode_png_bytes(png)?;
    let _ = kind;
    art::cover_rgba(&src, sw, sh, w, h)
}

/// The carousel ground, scaled to cover the whole screen, including behind the footer.
pub const BACKGROUND_W: u32 = 720;
pub const BACKGROUND_H: u32 = 480;

pub fn default_background() -> Option<&'static [u8]> {
    static P: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    P.get_or_init(|| art::cover_bytes(DEFAULT_PNG, BACKGROUND_W, BACKGROUND_H))
        .as_deref()
}

/// The blurred carousel ground, scaled to cover the whole screen the same way as the default.
pub fn blur_background() -> Option<&'static [u8]> {
    static P: OnceLock<Option<Vec<u8>>> = OnceLock::new();
    P.get_or_init(|| art::cover_bytes(BLUR_PNG, BACKGROUND_W, BACKGROUND_H))
        .as_deref()
}

pub fn draw_ground(canvas: Canvas, face: Option<(TexId, u32)>, out: &mut Vec<Draw>) {
    let Some(face) = face else {
        return;
    };
    out.push(ground(canvas, face));
}

pub fn draw_backdrop(canvas: Canvas, face: Option<(TexId, u32)>, out: &mut Vec<Draw>) {
    let Some(face) = face else {
        return;
    };
    out.push(ground(canvas, face));
    out.push(Draw::Rect {
        x: 0.0,
        y: 0.0,
        w: canvas.w,
        h: canvas.h,
        colour: [0.0, 0.0, 0.0, SCRIM],
    });
}

fn ground(canvas: Canvas, (tex, tex_w): (TexId, u32)) -> Draw {
    Draw::Part {
        x: 0.0,
        y: 0.0,
        w: canvas.w,
        h: canvas.h,
        tex,
        alpha: 1.0,
        uv: canvas.cover(tex_w as f32),
    }
}
