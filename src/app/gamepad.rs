//! Gamepad input: `gilrs` on the desktop, the browser Gamepad API (standard mapping) in the
//! browser. Both are poll-only, so `App` polls once per frame; the result is the same `Buttons`
//! the keyboard and the touch controls produce. Stick / d-pad move and climb, A jumps, Start
//! pauses (and restarts after the game ended).

use crate::app::controls::Buttons;

/// A stick must be pushed this far (of 1.0) before it counts as a direction.
const STICK_DEAD_ZONE: f32 = 0.5;

/// Raw state of one pad, already in game terms.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::app) struct RawPad {
    pub dpad: Buttons,
    /// Left stick, x to the right and y *down* (screen orientation).
    pub stick: (f32, f32),
    pub a: bool,
    pub start: bool,
}

/// What the pads did in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::app) struct PadFrame {
    pub held: Buttons,
    pub pressed: Buttons,
    /// Start went down in this frame.
    pub start_pressed: bool,
    /// A pad is connected (the app polls faster while one is, so Start works in a pause).
    pub connected: bool,
}

/// The game buttons of one pad: d-pad or stick, A = jump.
pub(in crate::app) fn buttons_of(p: &RawPad) -> Buttons {
    Buttons {
        left: p.dpad.left || p.stick.0 < -STICK_DEAD_ZONE,
        right: p.dpad.right || p.stick.0 > STICK_DEAD_ZONE,
        up: p.dpad.up || p.stick.1 < -STICK_DEAD_ZONE,
        down: p.dpad.down || p.stick.1 > STICK_DEAD_ZONE,
        jump: p.a,
    }
}

fn went_down(now: Buttons, before: Buttons) -> Buttons {
    Buttons {
        left: now.left && !before.left,
        right: now.right && !before.right,
        up: now.up && !before.up,
        down: now.down && !before.down,
        jump: now.jump && !before.jump,
    }
}

/// Combines the pads of this poll with the previous poll (edges).
fn frame_of(pads: &[RawPad], prev: &mut (Buttons, bool)) -> PadFrame {
    let held = pads
        .iter()
        .fold(Buttons::default(), |b, p| b.or(buttons_of(p)));
    let start = pads.iter().any(|p| p.start);
    let frame = PadFrame {
        held,
        pressed: went_down(held, prev.0),
        start_pressed: start && !prev.1,
        connected: !pads.is_empty(),
    };
    *prev = (held, start);
    frame
}

pub(in crate::app) struct Gamepad {
    prev: (Buttons, bool),
    #[cfg(not(target_arch = "wasm32"))]
    gilrs: Option<gilrs::Gilrs>,
}

impl Gamepad {
    pub(in crate::app) fn new() -> Self {
        Self {
            prev: (Buttons::default(), false),
            // No gamepad support on this system (or the backend failed): quietly no gamepad.
            #[cfg(not(target_arch = "wasm32"))]
            gilrs: gilrs::Gilrs::new().ok(),
        }
    }

    pub(in crate::app) fn poll(&mut self) -> PadFrame {
        let pads = self.read();
        frame_of(&pads, &mut self.prev)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn read(&mut self) -> Vec<RawPad> {
        use gilrs::{Axis, Button};
        let Some(g) = self.gilrs.as_mut() else {
            return Vec::new();
        };
        // Handle the pending events (connect / disconnect, and the state of every button).
        while g.next_event().is_some() {}
        g.gamepads()
            .filter(|(_, p)| p.is_connected())
            .map(|(_, p)| RawPad {
                dpad: Buttons {
                    left: p.is_pressed(Button::DPadLeft),
                    right: p.is_pressed(Button::DPadRight),
                    up: p.is_pressed(Button::DPadUp),
                    down: p.is_pressed(Button::DPadDown),
                    jump: false,
                },
                // gilrs: y is up-positive; the game wants screen orientation.
                stick: (p.value(Axis::LeftStickX), -p.value(Axis::LeftStickY)),
                a: p.is_pressed(Button::South),
                start: p.is_pressed(Button::Start),
            })
            .collect()
    }

    /// Standard mapping: buttons 0 = A, 9 = Start, 12 - 15 = d-pad up / down / left / right,
    /// axes 0 / 1 = the left stick.
    #[cfg(target_arch = "wasm32")]
    fn read(&mut self) -> Vec<RawPad> {
        use web_sys::wasm_bindgen::JsCast;
        let Some(nav) = web_sys::window().map(|w| w.navigator()) else {
            return Vec::new();
        };
        let Ok(list) = nav.get_gamepads() else {
            return Vec::new();
        };
        let mut pads = Vec::new();
        for item in list.iter() {
            let Ok(p) = item.dyn_into::<web_sys::Gamepad>() else {
                continue; // an empty slot (null)
            };
            if !p.connected() {
                continue;
            }
            let button = |i: u32| {
                p.buttons()
                    .get(i)
                    .dyn_into::<web_sys::GamepadButton>()
                    .map(|b| b.pressed())
                    .unwrap_or(false)
            };
            let axis = |i: u32| p.axes().get(i).as_f64().unwrap_or(0.0) as f32;
            pads.push(RawPad {
                dpad: Buttons {
                    up: button(12),
                    down: button(13),
                    left: button(14),
                    right: button(15),
                    jump: false,
                },
                stick: (axis(0), axis(1)),
                a: button(0),
                start: button(9),
            });
        }
        pads
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad() -> RawPad {
        RawPad::default()
    }

    #[test]
    fn the_stick_has_a_dead_zone() {
        let b = buttons_of(&RawPad {
            stick: (0.3, -0.4),
            ..pad()
        });
        assert_eq!(b, Buttons::default());
        let b = buttons_of(&RawPad {
            stick: (-0.9, -0.8),
            ..pad()
        });
        assert!(b.left && b.up && !b.right && !b.down);
    }

    #[test]
    fn dpad_and_a_work_and_combine_with_the_stick() {
        let b = buttons_of(&RawPad {
            dpad: Buttons {
                down: true,
                ..Buttons::default()
            },
            stick: (0.8, 0.0),
            a: true,
            ..pad()
        });
        assert!(b.down && b.right && b.jump && !b.up);
    }

    #[test]
    fn edges_fire_once_and_start_is_an_edge() {
        let mut prev = (Buttons::default(), false);
        let pressed = RawPad {
            a: true,
            start: true,
            ..pad()
        };
        let f = frame_of(&[pressed], &mut prev);
        assert!(f.pressed.jump && f.start_pressed && f.connected);
        let f = frame_of(&[pressed], &mut prev);
        assert!(f.held.jump && !f.pressed.jump && !f.start_pressed);
        let f = frame_of(&[pad()], &mut prev);
        assert!(!f.held.jump);
    }

    #[test]
    fn no_pad_means_not_connected_and_no_input() {
        let mut prev = (Buttons::default(), false);
        let f = frame_of(&[], &mut prev);
        assert_eq!(f, PadFrame::default());
    }

    #[test]
    fn two_pads_are_combined() {
        let mut prev = (Buttons::default(), false);
        let a = RawPad {
            stick: (1.0, 0.0),
            ..pad()
        };
        let b = RawPad { a: true, ..pad() };
        let f = frame_of(&[a, b], &mut prev);
        assert!(f.held.right && f.held.jump);
    }
}
