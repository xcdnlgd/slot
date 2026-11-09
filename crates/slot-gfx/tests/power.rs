use slot_gfx::{
    screen_brightness, screen_rect_above, screen_scale, screen_width, PANEL_H, PANEL_W,
};

#[test]
fn the_picture_opens_from_a_line() {
    assert!(
        screen_scale(0.0, PANEL_H as f32) < 0.02,
        "it starts at full height"
    );
    assert!(screen_scale(0.5, PANEL_H as f32) > 0.2 && screen_scale(0.5, PANEL_H as f32) < 1.0);
    assert!(
        (screen_scale(1.0, PANEL_H as f32) - 1.0).abs() < 0.001,
        "it never reached full height"
    );
}

#[test]
fn brightness_overshoots_then_settles() {
    let steps: Vec<f32> = (0..=100)
        .map(|i| screen_brightness(i as f32 / 100.0))
        .collect();
    assert!(steps.windows(2).all(|w| w[0] >= w[1] - 0.0001));
    assert!(steps[0] > 1.3, "it does not flash bright enough");
    assert!(
        (screen_brightness(1.0) - 1.0).abs() < 0.01,
        "it does not settle at normal"
    );
}

#[test]
fn a_footer_keeps_its_strip_while_the_picture_scales_above_it() {
    let footer = 58.0;
    let (x, y, w, h) = screen_rect_above(PANEL_W as f32, PANEL_H as f32, 1.0, footer);
    assert!(x.abs() < 0.01 && y.abs() < 0.01);
    assert!((w - PANEL_W as f32).abs() < 0.01);
    assert!((h - (PANEL_H as f32 - footer)).abs() < 0.01);

    let (_, mid_y, _, mid_h) = screen_rect_above(PANEL_W as f32, PANEL_H as f32, 0.5, footer);
    assert!(mid_h < h, "the picture did not shrink above the footer");
    assert!((mid_y + mid_h / 2.0 - (PANEL_H as f32 - footer) / 2.0).abs() < 1.0);

    let (_, full_y, _, full_h) = screen_rect_above(PANEL_W as f32, PANEL_H as f32, 0.5, 0.0);
    assert!(full_h > mid_h, "the strip did not give the picture room");
    assert!((full_y + full_h / 2.0 - PANEL_H as f32 / 2.0).abs() < 1.0);
}

#[test]
fn the_width_closes_to_a_dot_then_opens() {
    assert!(
        screen_width(0.0, PANEL_W as f32) < 0.02,
        "the line never closes to a dot"
    );
    assert!(
        screen_width(0.5, PANEL_W as f32) > 0.99,
        "the line is still shorter than the screen at half power"
    );
    assert!((screen_width(1.0, PANEL_W as f32) - 1.0).abs() < 0.001);
}
