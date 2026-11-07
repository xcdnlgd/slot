use slot_gfx::{Draw, TexId};
use slot_power::{Battery, Charge};

use crate::footer::Printed;
use crate::plate::HINT_H;

pub const BOLT_PX: f32 = 18.0;

pub const GAUGE_W: f32 = 26.4;
pub const GAUGE_H: f32 = 13.2;
const _: () = assert!(GAUGE_H < GAUGE_W);
const NUB_W: f32 = 3.0;
const NUB_H: f32 = 4.8;
pub const WALL: f32 = 1.8;
const GAP: f32 = 7.0;

const BOLT_W: f32 = 14.0;
const BOLT_H: f32 = 14.0;
const BOLT_GAP: f32 = 5.0;
const _: () = assert!(BOLT_GAP > 0.0);
pub const GAUGE_ROW_W: f32 = BOLT_W + BOLT_GAP + GAUGE_W + NUB_W + GAP;

pub fn draw_gauge(
    x: f32,
    y: f32,
    battery: Option<Battery>,
    percent: Printed,
    bolt: Option<TexId>,
    ink: [u8; 3],
    out: &mut Vec<Draw>,
) {
    let Some(b) = battery else {
        return;
    };
    let colour = [
        ink[0] as f32 / 255.0,
        ink[1] as f32 / 255.0,
        ink[2] as f32 / 255.0,
        1.0,
    ];

    let rect = |x: f32, y: f32, w: f32, h: f32, out: &mut Vec<Draw>| {
        out.push(Draw::Rect { x, y, w, h, colour });
    };

    // Snap the capsule to whole pixels. The 1.2x design leaves the walls on
    // fractional coordinates, which makes one edge land a pixel thinner than
    // the rest depending on where the gauge sits.
    let bolt_x = x.round();
    let left = bolt_x + BOLT_W + BOLT_GAP;
    let top = y.round();
    let wall = WALL.round().max(1.0);
    let width = GAUGE_W.round().max(2.0 * wall + 1.0);
    let height = GAUGE_H.round().max(2.0 * wall + 1.0);
    let right = left + width;
    let bottom = top + height;

    rect(left, top, width, wall, out);
    rect(left, bottom - wall, width, wall, out);
    rect(left, top, wall, height, out);
    rect(right - wall, top, wall, height, out);

    let nub_w = NUB_W.round().max(1.0);
    let nub_h = NUB_H.round().max(1.0);
    let nub_y = (top + (height - nub_h) / 2.0).round();
    rect(right, nub_y, nub_w, nub_h, out);

    let inner = width - 4.0 * wall;
    let fill = inner * f32::from(b.percent.min(100)) / 100.0;
    if fill > 0.0 {
        rect(
            left + 2.0 * wall,
            top + 2.0 * wall,
            fill,
            height - 4.0 * wall,
            out,
        );
    }

    if let (Charge::Charging, Some(tex)) = (b.charge, bolt) {
        out.push(Draw::Tex {
            x: bolt_x,
            y: (top + (height - BOLT_H) / 2.0).round(),
            w: BOLT_W,
            h: BOLT_H,
            tex,
            alpha: 1.0,
        });
    }

    if percent.w > 0 {
        let px = right + nub_w + GAP;
        let py = (top + (height - HINT_H as f32) / 2.0).round();
        out.push(match percent.face {
            Some(tex) => Draw::Tex {
                x: px,
                y: py,
                w: percent.w as f32,
                h: HINT_H as f32,
                tex,
                alpha: 1.0,
            },
            None => Draw::Rect {
                x: px,
                y: py,
                w: percent.w as f32,
                h: HINT_H as f32,
                colour: [1.0, 1.0, 1.0, 0.08],
            },
        });
    }
}
