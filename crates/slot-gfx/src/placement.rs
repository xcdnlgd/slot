#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sizing {
    Integer,
    Fit,
    Panel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameSize {
    pub w: u32,
    pub h: u32,
}

/// The picture's size in panel pixels. Position is decided separately by the
/// panel/bezel offset.
pub fn game_size(panel: (u32, u32), picture: (u32, u32), sizing: Sizing) -> GameSize {
    let (pw, ph) = (picture.0.max(1), picture.1.max(1));
    let (w, h) = match sizing {
        Sizing::Panel => (panel.0, panel.1),
        Sizing::Integer => {
            let k = (panel.0 / pw).min(panel.1 / ph).max(1);
            (pw * k, ph * k)
        }
        Sizing::Fit => {
            let scale = (panel.0 as f32 / pw as f32).min(panel.1 as f32 / ph as f32);
            (
                (pw as f32 * scale).round() as u32,
                (ph as f32 * scale).round() as u32,
            )
        }
    };
    GameSize { w, h }
}
