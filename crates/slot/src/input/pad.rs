use sdl3::gamepad::Button;
use slot_input::Btn;

pub fn pad_to_btn(button: Button) -> Option<Btn> {
    Some(match button {
        Button::DPadUp => Btn::Up,
        Button::DPadDown => Btn::Down,
        Button::DPadLeft => Btn::Left,
        Button::DPadRight => Btn::Right,
        Button::East => Btn::A,
        Button::South => Btn::B,
        Button::North => Btn::X,
        Button::West => Btn::Y,
        Button::LeftShoulder => Btn::L1,
        Button::RightShoulder => Btn::R1,
        Button::Start => Btn::Start,
        Button::Back => Btn::Select,
        Button::Guide => Btn::Menu,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdl_buttons_keep_the_handheld_layout() {
        assert_eq!(pad_to_btn(Button::South), Some(Btn::B));
        assert_eq!(pad_to_btn(Button::East), Some(Btn::A));
        assert_eq!(pad_to_btn(Button::North), Some(Btn::X));
        assert_eq!(pad_to_btn(Button::West), Some(Btn::Y));
        assert_eq!(pad_to_btn(Button::LeftShoulder), Some(Btn::L1));
        assert_eq!(pad_to_btn(Button::RightShoulder), Some(Btn::R1));
        assert_eq!(pad_to_btn(Button::Back), Some(Btn::Select));
        assert_eq!(pad_to_btn(Button::Start), Some(Btn::Start));
        assert_eq!(pad_to_btn(Button::Guide), Some(Btn::Menu));
    }

    #[test]
    fn sticks_and_paddles_are_left_for_the_future() {
        assert_eq!(pad_to_btn(Button::LeftStick), None);
        assert_eq!(pad_to_btn(Button::RightStick), None);
        assert_eq!(pad_to_btn(Button::Touchpad), None);
        assert_eq!(pad_to_btn(Button::Misc1), None);
    }
}
