use std::collections::HashMap;

use sdl3::event::Event;
use sdl3::gamepad::{Axis, Button, Gamepad};
use sdl3::joystick::JoystickId;
use sdl3::{EventPump, GamepadSubsystem, Sdl};
use slot_input::{Btn, InputSource, Millis, RawEvent};
use winit::event::WindowEvent;
use winit::keyboard::{KeyCode, PhysicalKey};

use super::keys::key_to_btn;
use super::pad::pad_to_btn;

const TRIGGER_DOWN: i16 = 24_575;
const TRIGGER_UP: i16 = 21_299;

struct PadState {
    _gamepad: Gamepad,
    held: Vec<Btn>,
    left_trigger: bool,
    right_trigger: bool,
    guide: bool,
}

#[derive(Default)]
struct Held {
    counts: Vec<(Btn, usize)>,
}

impl Held {
    fn press(&mut self, btn: Btn, out: &mut Vec<RawEvent>) {
        if let Some((_, count)) = self.counts.iter_mut().find(|(b, _)| *b == btn) {
            *count += 1;
            return;
        }
        self.counts.push((btn, 1));
        down(out, btn);
    }

    fn release(&mut self, btn: Btn, out: &mut Vec<RawEvent>) {
        let Some(at) = self.counts.iter().position(|(b, _)| *b == btn) else {
            return;
        };
        if self.counts[at].1 > 1 {
            self.counts[at].1 -= 1;
            return;
        }
        self.counts.remove(at);
        out.push(RawEvent::Up(btn));
    }
}

struct Pads {
    subsystem: GamepadSubsystem,
    events: EventPump,
    open: HashMap<JoystickId, PadState>,
    held: Held,
    _sdl: Sdl,
}

impl Pads {
    fn open() -> Result<Self, String> {
        let sdl = sdl3::init().map_err(|e| e.to_string())?;
        let subsystem = sdl.gamepad().map_err(|e| e.to_string())?;
        let events = sdl.event_pump().map_err(|e| e.to_string())?;
        let mut pads = Pads {
            subsystem,
            events,
            open: HashMap::new(),
            held: Held::default(),
            _sdl: sdl,
        };
        let attached = pads.subsystem.gamepads().unwrap_or_default();
        for id in attached {
            pads.attach(id);
        }
        Ok(pads)
    }

    fn attach(&mut self, id: JoystickId) {
        if self.open.contains_key(&id) {
            return;
        }
        match self.subsystem.open(id) {
            Ok(gamepad) => {
                self.open.insert(
                    id,
                    PadState {
                        _gamepad: gamepad,
                        held: Vec::new(),
                        left_trigger: false,
                        right_trigger: false,
                        guide: false,
                    },
                );
            }
            Err(e) => eprintln!("slot: gamepad {id:?}: {e}"),
        }
    }

    fn detach(&mut self, id: JoystickId, out: &mut Vec<RawEvent>) {
        let Some(pad) = self.open.remove(&id) else {
            return;
        };
        for btn in pad.held {
            self.held.release(btn, out);
        }
    }

    fn press(&mut self, id: JoystickId, btn: Btn, out: &mut Vec<RawEvent>) {
        let Some(pad) = self.open.get_mut(&id) else {
            return;
        };
        if pad.held.contains(&btn) {
            return;
        }
        pad.held.push(btn);
        self.held.press(btn, out);
    }

    fn release(&mut self, id: JoystickId, btn: Btn, out: &mut Vec<RawEvent>) {
        let Some(pad) = self.open.get_mut(&id) else {
            return;
        };
        let Some(at) = pad.held.iter().position(|b| *b == btn) else {
            return;
        };
        pad.held.remove(at);
        self.held.release(btn, out);
    }

    fn trigger(&mut self, id: JoystickId, left: bool, value: i16, out: &mut Vec<RawEvent>) {
        let Some(pad) = self.open.get_mut(&id) else {
            return;
        };
        let state = if left {
            &mut pad.left_trigger
        } else {
            &mut pad.right_trigger
        };
        let Some(pressed) = trigger_edge(state, value) else {
            return;
        };
        let btn = if left { Btn::L2 } else { Btn::R2 };
        match pressed {
            true => self.press(id, btn, out),
            false => self.release(id, btn, out),
        }
    }

    /// SDL delays the Guide button's release until it has been down for 250ms,
    /// which is exactly slot's menu tap window. Ask the hardware every frame
    /// instead so a quick tap still produces down and up in time.
    fn poll_guide(&mut self, out: &mut Vec<RawEvent>) {
        let changed: Vec<(JoystickId, bool)> = self
            .open
            .iter_mut()
            .filter_map(|(id, pad)| {
                let down = pad._gamepad.button(Button::Guide);
                if down == pad.guide {
                    return None;
                }
                pad.guide = down;
                Some((*id, down))
            })
            .collect();
        for (id, down) in changed {
            match down {
                true => self.press(id, Btn::Menu, out),
                false => self.release(id, Btn::Menu, out),
            }
        }
    }

    fn poll(&mut self) -> Vec<RawEvent> {
        let mut out = Vec::new();
        while let Some(event) = self.events.poll_event() {
            match event {
                Event::GamepadAdded { which, .. } => self.attach(which),
                Event::GamepadRemoved { which, .. } => self.detach(which, &mut out),
                Event::GamepadButtonDown { which, button, .. }
                    if self.open.contains_key(&which) && button != Button::Guide =>
                {
                    if let Some(btn) = pad_to_btn(button) {
                        self.press(which, btn, &mut out);
                    }
                }
                Event::GamepadButtonUp { which, button, .. }
                    if self.open.contains_key(&which) && button != Button::Guide =>
                {
                    if let Some(btn) = pad_to_btn(button) {
                        self.release(which, btn, &mut out);
                    }
                }
                Event::GamepadAxisMotion {
                    which, axis, value, ..
                } if self.open.contains_key(&which) => match axis {
                    Axis::TriggerLeft => self.trigger(which, true, value, &mut out),
                    Axis::TriggerRight => self.trigger(which, false, value, &mut out),
                    _ => {}
                },
                _ => {}
            }
        }
        self.poll_guide(&mut out);
        out
    }
}

fn down(out: &mut Vec<RawEvent>, btn: Btn) {
    crate::latency::read();
    out.push(RawEvent::Down(btn));
}

fn trigger_edge(state: &mut bool, value: i16) -> Option<bool> {
    match (*state, value) {
        (false, v) if v >= TRIGGER_DOWN => {
            *state = true;
            Some(true)
        }
        (true, v) if v <= TRIGGER_UP => {
            *state = false;
            Some(false)
        }
        _ => None,
    }
}

pub struct HostInput {
    pending: Vec<RawEvent>,
    pads: Option<Pads>,
    lid_closed: bool,
    held_keys: Vec<Btn>,
}

impl HostInput {
    pub fn new() -> Self {
        let pads = match Pads::open() {
            Ok(pads) => Some(pads),
            Err(e) => {
                eprintln!("slot: gamepad: {e}");
                None
            }
        };
        Self::with_pads(pads)
    }

    /// Keyboard only, for tests and for hosts without a usable SDL gamepad stack.
    pub fn keyboard_only() -> Self {
        Self::with_pads(None)
    }

    fn with_pads(pads: Option<Pads>) -> Self {
        HostInput {
            pending: Vec::new(),
            pads,
            lid_closed: false,
            held_keys: Vec::new(),
        }
    }

    pub fn on_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Focused(false) => self.release_keys(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else {
                    return;
                };
                self.key(code, event.state.is_pressed(), event.repeat);
            }
            _ => {}
        }
    }

    fn release_keys(&mut self) {
        for btn in std::mem::take(&mut self.held_keys) {
            self.pending.push(RawEvent::Up(btn));
        }
    }

    pub fn key(&mut self, code: KeyCode, pressed: bool, repeat: bool) {
        if repeat {
            return;
        }
        let Some(btn) = key_to_btn(code) else {
            return;
        };
        if btn == Btn::Lid {
            if pressed {
                self.lid_closed = !self.lid_closed;
                self.pending.push(if self.lid_closed {
                    RawEvent::Down(Btn::Lid)
                } else {
                    RawEvent::Up(Btn::Lid)
                });
            }
            return;
        }
        match pressed {
            true if !self.held_keys.contains(&btn) => self.held_keys.push(btn),
            true => {}
            false => self.held_keys.retain(|b| *b != btn),
        }
        self.push(btn, pressed);
    }

    fn push(&mut self, btn: Btn, pressed: bool) {
        if pressed {
            crate::latency::read();
        }
        self.pending.push(if pressed {
            RawEvent::Down(btn)
        } else {
            RawEvent::Up(btn)
        });
    }

    fn drain_pads(&mut self) {
        let Some(pads) = self.pads.as_mut() else {
            return;
        };
        self.pending.extend(pads.poll());
    }
}

impl Default for HostInput {
    fn default() -> Self {
        HostInput::new()
    }
}

impl InputSource for HostInput {
    fn poll(&mut self, _now: Millis) -> Vec<RawEvent> {
        self.drain_pads();
        std::mem::take(&mut self.pending)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_use_hysteresis_instead_of_chattering() {
        let mut held = false;
        assert_eq!(trigger_edge(&mut held, TRIGGER_DOWN - 1), None);
        assert_eq!(trigger_edge(&mut held, TRIGGER_DOWN), Some(true));
        assert_eq!(trigger_edge(&mut held, TRIGGER_UP + 1), None);
        assert_eq!(trigger_edge(&mut held, TRIGGER_UP), Some(false));
    }

    #[test]
    fn a_button_held_by_two_pads_releases_only_once() {
        let mut held = Held::default();
        let mut out = Vec::new();
        held.press(Btn::A, &mut out);
        held.press(Btn::A, &mut out);
        assert_eq!(out, vec![RawEvent::Down(Btn::A)]);
        out.clear();
        held.release(Btn::A, &mut out);
        assert!(out.is_empty());
        held.release(Btn::A, &mut out);
        assert_eq!(out, vec![RawEvent::Up(Btn::A)]);
    }
}
