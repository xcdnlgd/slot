const LINE_PX: f32 = 2.0;

const DOT_T: f32 = 0.12;

const OVERSHOOT: f32 = 0.6;

pub fn screen_scale(t: f32, height: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 - (1.0 - LINE_PX / height) * left * left
}

pub fn screen_width(t: f32, width: f32) -> f32 {
    let left = 1.0 - (t.clamp(0.0, 1.0) / DOT_T).min(1.0);
    1.0 - (1.0 - LINE_PX / width) * left
}

pub fn screen_brightness(t: f32) -> f32 {
    let left = 1.0 - t.clamp(0.0, 1.0);
    1.0 + OVERSHOOT * left * left
}

/// The picture's place while a footer of `footer` px stays put along the bottom.
/// The scale is centred in the space above that strip, so the strip is not part of it.
pub fn screen_rect_above(width: f32, height: f32, t: f32, footer: f32) -> (f32, f32, f32, f32) {
    let view = height - footer.max(0.0);
    let w = width * screen_width(t, width);
    let h = view * screen_scale(t, height);
    ((width - w) / 2.0, (view - h) / 2.0, w, h)
}

/// The panel height the tests and the old callers use when they do not have a
/// concrete drawing target.
pub const DEFAULT_HEIGHT: f32 = 480.0;
