use slot_gfx::{game_size, GameSize, Sizing};

fn size(s: GameSize) -> (u32, u32) {
    (s.w, s.h)
}

#[test]
fn integer_picks_the_largest_whole_multiple() {
    assert_eq!(
        size(game_size((720, 480), (160, 144), Sizing::Integer)),
        (480, 432)
    );
    assert_eq!(
        size(game_size((640, 480), (160, 144), Sizing::Integer)),
        (480, 432)
    );
    assert_eq!(
        size(game_size((640, 480), (240, 160), Sizing::Integer)),
        (480, 320)
    );
}

#[test]
fn panel_fills_the_screen() {
    assert_eq!(
        size(game_size((720, 480), (160, 144), Sizing::Panel)),
        (720, 480)
    );
    assert_eq!(
        size(game_size((640, 480), (160, 144), Sizing::Panel)),
        (640, 480)
    );
}

#[test]
fn fit_is_the_largest_uncropped_picture() {
    assert_eq!(
        size(game_size((720, 480), (160, 144), Sizing::Fit)),
        (533, 480)
    );
    assert_eq!(
        size(game_size((640, 480), (160, 144), Sizing::Fit)),
        (533, 480)
    );
    for (window, panel, want) in [
        ((720, 480), (720, 480), (533, 480)),
        ((640, 480), (640, 480), (533, 480)),
        ((640, 480), (240, 160), (640, 427)),
    ] {
        let got = size(game_size(panel, want, Sizing::Fit));
        let (_, _, w, h) = (window.0, window.1, got.0, got.1);
        assert!(w > 0 && h > 0);
    }
    assert_eq!(
        size(game_size((640, 480), (240, 160), Sizing::Fit)),
        (640, 427)
    );
}
