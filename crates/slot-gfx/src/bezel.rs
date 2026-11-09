pub fn framed(panel: (u32, u32)) -> bool {
    panel.0 > 0 && panel.0 * 3 == panel.1 * 4
}

/// The transparent opening of a panel-sized bezel, in panel pixels.
pub fn hole(rgba: &[u8], panel: (u32, u32)) -> Option<(i32, i32, u32, u32)> {
    let (w, h) = (panel.0 as usize, panel.1 as usize);
    if w == 0 || h == 0 || rgba.len() < w * h * 4 {
        return None;
    }
    let (mut left, mut top, mut right, mut bottom) = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if rgba[(y * w + x) * 4 + 3] < 128 {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x);
                bottom = bottom.max(y);
            }
        }
    }
    if left == usize::MAX {
        return None;
    }
    Some((
        left as i32,
        top as i32,
        (right - left + 1) as u32,
        (bottom - top + 1) as u32,
    ))
}
