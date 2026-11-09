#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Canvas {
    pub w: f32,
    pub h: f32,
}

impl Canvas {
    /// Fallback canvas for tests and construction before a panel is known.
    pub const DEFAULT: Canvas = Canvas { w: 720.0, h: 480.0 };

    pub fn of(panel: (u32, u32)) -> Canvas {
        if panel.0 == 0 || panel.1 == 0 {
            return Canvas::DEFAULT;
        }
        Canvas {
            w: panel.0 as f32,
            h: panel.1 as f32,
        }
    }

    pub fn centred(self, w: f32) -> f32 {
        (self.w - w) / 2.0
    }

    pub fn window(self, tex_w: f32) -> [f32; 4] {
        if tex_w <= self.w {
            return [0.0, 0.0, 1.0, 1.0];
        }
        [(tex_w - self.w) / 2.0 / tex_w, 0.0, self.w / tex_w, 1.0]
    }

    pub fn cover(self, tex_w: f32) -> [f32; 4] {
        if tex_w >= self.w {
            return self.window(tex_w);
        }
        let shown = tex_w / self.w;
        [0.0, (1.0 - shown) / 2.0, 1.0, shown]
    }
}
