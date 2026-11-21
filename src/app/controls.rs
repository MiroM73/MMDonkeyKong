//! Input sources shared by keyboard, touch and gamepad: the button set, the press latch that keeps
//! a tap shorter than one frame, the touch tracker and the (pure, tested) layout of the touch
//! controls. Drawing lives in `touch_view.rs`.

use std::collections::HashMap;

use crate::app::egui::{self, Pos2, Rect, Vec2};
use crate::game::Input;

/// The five game buttons, whatever device they come from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::app) struct Buttons {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
}

impl Buttons {
    pub(in crate::app) fn or(self, o: Buttons) -> Buttons {
        Buttons {
            left: self.left | o.left,
            right: self.right | o.right,
            up: self.up | o.up,
            down: self.down | o.down,
            jump: self.jump | o.jump,
        }
    }
}

/// Remembers the buttons that went down since the last simulation step. The input is sampled once
/// per frame, so a tap shorter than a frame (down and up inside the same frame) would never be
/// seen as "held"; the latch makes every press count for at least one step. A frame without a
/// step keeps the latch, a frame with two steps delivers it once.
#[derive(Debug, Default)]
pub(in crate::app) struct PressLatch {
    pending: Buttons,
}

impl PressLatch {
    /// Registers the buttons that went down in this frame.
    pub(in crate::app) fn press(&mut self, pressed: Buttons) {
        self.pending = self.pending.or(pressed);
    }

    /// The input of the next step: held buttons plus the latched presses (consumed here).
    pub(in crate::app) fn take_step(&mut self, held: Buttons) -> Input {
        let p = std::mem::take(&mut self.pending);
        let m = held.or(p);
        Input {
            left: m.left,
            right: m.right,
            up: m.up,
            down: m.down,
            jump_held: m.jump,
            jump_pressed: p.jump,
            auto_grab: false,
        }
    }

    pub(in crate::app) fn clear(&mut self) {
        self.pending = Buttons::default();
    }
}

/// The live touches (multi-touch), fed from `egui::Event::Touch`. egui's pointer follows only one
/// finger, so the d-pad and the jump button (two thumbs) need the raw touches.
#[derive(Debug, Default)]
pub(in crate::app) struct Touches {
    active: HashMap<u64, Pos2>,
    /// A touch event has ever arrived: the device has a touch screen.
    pub seen: bool,
}

impl Touches {
    /// Applies the events of one frame; returns the positions where a touch started.
    pub(in crate::app) fn update(&mut self, events: &[egui::Event]) -> Vec<Pos2> {
        let mut started = Vec::new();
        for e in events {
            if let egui::Event::Touch { id, phase, pos, .. } = e {
                self.seen = true;
                match phase {
                    egui::TouchPhase::Start => {
                        self.active.insert(id.0, *pos);
                        started.push(*pos);
                    }
                    egui::TouchPhase::Move => {
                        self.active.insert(id.0, *pos);
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel => {
                        self.active.remove(&id.0);
                    }
                }
            }
        }
        started
    }

    pub(in crate::app) fn positions(&self) -> impl Iterator<Item = Pos2> + '_ {
        self.active.values().copied()
    }
}

/// How the on-screen controls are shown: "auto" (after the first touch), "on", "off".
pub(in crate::app) fn controls_visible(mode: &str, touch_seen: bool) -> bool {
    match mode {
        "on" => true,
        "off" => false,
        _ => touch_seen,
    }
}

/// Positions of the on-screen controls and the area left for the playfield.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct Layout {
    pub field_area: Rect,
    pub dpad_center: Pos2,
    pub dpad_radius: f32,
    pub jump_center: Pos2,
    pub jump_radius: f32,
    pub pause: Rect,
    pub controls: bool,
}

/// Size of the pause button in points (comfortably over 9 mm on a phone).
pub(in crate::app) const PAUSE_SIZE: f32 = 44.0;
const MARGIN: f32 = 12.0;
const DPAD_RADIUS: f32 = 66.0;
const JUMP_RADIUS: f32 = 44.0;
/// In portrait the controls take at most this share of the height.
const MAX_BAND: f32 = 0.42;

/// Portrait (taller than wide): the playfield goes to the top and the controls below it.
/// Landscape: the controls overlay the left / right edges of the area. `size` scales the
/// controls, `left_handed` swaps the sides.
pub(in crate::app) fn layout(area: Rect, controls: bool, size: f32, left_handed: bool) -> Layout {
    let pause = Rect::from_min_size(
        Pos2::new(area.right() - PAUSE_SIZE - 4.0, area.top() + 4.0),
        Vec2::splat(PAUSE_SIZE),
    );
    let mut l = Layout {
        field_area: area,
        dpad_center: area.center(),
        dpad_radius: 0.0,
        jump_center: area.center(),
        jump_radius: 0.0,
        pause,
        controls,
    };
    if !controls {
        return l;
    }
    let size = size.clamp(0.5, 2.0);
    let mut dr = DPAD_RADIUS * size;
    let mut jr = JUMP_RADIUS * size;
    let portrait = area.height() > area.width();
    let (dy, jy);
    if portrait {
        let mut band = 2.0 * dr + 2.0 * MARGIN;
        let max = area.height() * MAX_BAND;
        if band > max {
            let k = max / band;
            dr *= k;
            jr *= k;
            band = max;
        }
        l.field_area = Rect::from_min_max(area.min, Pos2::new(area.right(), area.bottom() - band));
        dy = area.bottom() - band / 2.0;
        jy = dy;
    } else {
        dy = area.bottom() - MARGIN - dr;
        jy = area.bottom() - MARGIN - jr;
    }
    let left_x = |r: f32| area.left() + MARGIN + r;
    let right_x = |r: f32| area.right() - MARGIN - r;
    let (dx, jx) = if left_handed {
        (right_x(dr), left_x(jr))
    } else {
        (left_x(dr), right_x(jr))
    };
    l.dpad_center = Pos2::new(dx, dy);
    l.dpad_radius = dr;
    l.jump_center = Pos2::new(jx, jy);
    l.jump_radius = jr;
    l
}

/// The d-pad buttons held by a finger at `pos`: a 3 x 3 grid around the centre (diagonals
/// included, the centre is a dead zone); the touch may slide a bit outside the circle.
pub(in crate::app) fn dpad_buttons(center: Pos2, radius: f32, pos: Pos2) -> Buttons {
    let d = pos - center;
    if radius <= 0.0 || d.length() > radius * 1.5 {
        return Buttons::default();
    }
    let dead = radius * 0.3;
    Buttons {
        left: d.x < -dead,
        right: d.x > dead,
        up: d.y < -dead,
        down: d.y > dead,
        jump: false,
    }
}

/// The buttons held by the given touch positions (all fingers together).
pub(in crate::app) fn touch_buttons(l: &Layout, touches: impl Iterator<Item = Pos2>) -> Buttons {
    let mut b = Buttons::default();
    if !l.controls {
        return b;
    }
    for p in touches {
        b = b.or(dpad_buttons(l.dpad_center, l.dpad_radius, p));
        if (p - l.jump_center).length() <= l.jump_radius * 1.3 {
            b.jump = true;
        }
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn left() -> Buttons {
        Buttons {
            left: true,
            ..Buttons::default()
        }
    }

    #[test]
    fn a_tap_shorter_than_a_frame_still_moves_one_step() {
        let mut latch = PressLatch::default();
        latch.press(left()); // down and up in the same frame: not held, but pressed
        let first = latch.take_step(Buttons::default());
        assert!(first.left);
        let second = latch.take_step(Buttons::default());
        assert!(!second.left, "the latch is consumed by the first step");
    }

    #[test]
    fn a_frame_without_a_step_keeps_the_press() {
        let mut latch = PressLatch::default();
        latch.press(left());
        latch.press(Buttons::default()); // next frame, still no step
        assert!(latch.take_step(Buttons::default()).left);
    }

    #[test]
    fn held_buttons_stay_and_jump_is_an_edge() {
        let mut latch = PressLatch::default();
        let held = Buttons {
            right: true,
            jump: true,
            ..Buttons::default()
        };
        latch.press(Buttons {
            jump: true,
            ..Buttons::default()
        });
        let a = latch.take_step(held);
        assert!(a.right && a.jump_held && a.jump_pressed);
        let b = latch.take_step(held);
        assert!(b.right && b.jump_held && !b.jump_pressed);
    }

    #[test]
    fn clear_drops_the_latched_press() {
        let mut latch = PressLatch::default();
        latch.press(left());
        latch.clear();
        assert!(!latch.take_step(Buttons::default()).left);
    }

    #[test]
    fn multi_touch_tracks_each_finger() {
        let touch = |id, phase, x| egui::Event::Touch {
            device_id: egui::TouchDeviceId(0),
            id: egui::TouchId(id),
            phase,
            pos: Pos2::new(x, 0.0),
            force: None,
        };
        let mut t = Touches::default();
        let started = t.update(&[
            touch(1, egui::TouchPhase::Start, 10.0),
            touch(2, egui::TouchPhase::Start, 20.0),
        ]);
        assert_eq!(started.len(), 2);
        assert!(t.seen);
        assert_eq!(t.positions().count(), 2);
        t.update(&[touch(1, egui::TouchPhase::End, 10.0)]);
        assert_eq!(
            t.positions().collect::<Vec<_>>(),
            vec![Pos2::new(20.0, 0.0)]
        );
        t.update(&[touch(2, egui::TouchPhase::Cancel, 20.0)]);
        assert_eq!(t.positions().count(), 0);
    }

    #[test]
    fn visibility_modes() {
        assert!(controls_visible("on", false));
        assert!(!controls_visible("off", true));
        assert!(!controls_visible("auto", false));
        assert!(controls_visible("auto", true));
        assert!(!controls_visible("anything", false));
    }

    fn area(w: f32, h: f32) -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h))
    }

    #[test]
    fn portrait_puts_the_controls_below_the_field() {
        let l = layout(area(400.0, 800.0), true, 1.0, false);
        assert!(l.field_area.bottom() < 800.0);
        assert_eq!(l.field_area.top(), 0.0);
        assert!(l.dpad_center.y > l.field_area.bottom());
        assert!(l.jump_center.y > l.field_area.bottom());
        assert!(l.dpad_center.x < l.jump_center.x);
    }

    #[test]
    fn landscape_overlays_the_edges_and_keeps_the_whole_area() {
        let a = area(900.0, 400.0);
        let l = layout(a, true, 1.0, false);
        assert_eq!(l.field_area, a);
        assert!(l.dpad_center.x < 450.0 && l.jump_center.x > 450.0);
    }

    #[test]
    fn left_handed_swaps_the_sides() {
        let l = layout(area(900.0, 400.0), true, 1.0, true);
        assert!(l.dpad_center.x > l.jump_center.x);
    }

    #[test]
    fn controls_never_take_more_than_their_share_of_a_portrait_area() {
        let l = layout(area(300.0, 400.0), true, 2.0, false);
        assert!(l.field_area.height() >= 400.0 * (1.0 - MAX_BAND) - 0.01);
        assert!(l.dpad_center.y + l.dpad_radius <= 400.0);
    }

    #[test]
    fn no_controls_leaves_the_area_alone() {
        let a = area(400.0, 800.0);
        let l = layout(a, false, 1.0, false);
        assert_eq!(l.field_area, a);
        assert_eq!(
            touch_buttons(&l, [a.center()].into_iter()),
            Buttons::default()
        );
    }

    #[test]
    fn the_hit_areas_are_big_enough_for_a_thumb() {
        // About 9 mm is 34 css pixels at 96 dpi; the smallest size in the menu is 0.6.
        let l = layout(area(900.0, 400.0), true, 0.6, false);
        assert!(l.jump_radius * 2.0 >= 34.0 && l.dpad_radius * 2.0 >= 34.0 * 2.0);
        const { assert!(PAUSE_SIZE >= 34.0) };
    }

    #[test]
    fn the_dpad_reads_directions_and_diagonals() {
        let c = Pos2::new(100.0, 100.0);
        let at = |dx: f32, dy: f32| dpad_buttons(c, 60.0, Pos2::new(100.0 + dx, 100.0 + dy));
        assert_eq!(at(0.0, 0.0), Buttons::default()); // dead zone
        assert!(at(-40.0, 0.0).left && !at(-40.0, 0.0).up);
        assert!(at(40.0, 0.0).right);
        assert!(at(0.0, -40.0).up && !at(0.0, -40.0).right);
        assert!(at(0.0, 40.0).down);
        let d = at(35.0, -35.0);
        assert!(d.right && d.up);
        assert_eq!(at(300.0, 0.0), Buttons::default()); // far away
    }

    #[test]
    fn two_thumbs_move_and_jump_together() {
        let l = layout(area(900.0, 400.0), true, 1.0, false);
        let b = touch_buttons(
            &l,
            [
                l.dpad_center + Vec2::new(40.0, 0.0),
                l.jump_center + Vec2::new(5.0, 5.0),
            ]
            .into_iter(),
        );
        assert!(b.right && b.jump && !b.left);
    }

    #[test]
    fn the_pause_button_does_not_overlap_the_controls() {
        for (w, h) in [(900.0, 400.0), (400.0, 800.0)] {
            let l = layout(area(w, h), true, 1.0, false);
            for (c, r) in [
                (l.dpad_center, l.dpad_radius),
                (l.jump_center, l.jump_radius),
            ] {
                let closest = c.clamp(l.pause.min, l.pause.max);
                assert!((closest - c).length() > r * 1.3, "{w}x{h}");
            }
        }
    }
}
