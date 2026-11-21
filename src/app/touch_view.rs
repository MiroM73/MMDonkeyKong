//! Drawing of the on-screen controls (d-pad, jump button) and the pause button.

use crate::app::controls::{Buttons, Layout};
use crate::app::egui::{self, Color32, Pos2, Stroke, Vec2};

const BASE: u8 = 38;
const PRESSED: u8 = 90;
const EDGE: Color32 = Color32::from_rgba_premultiplied(120, 120, 120, 120);
const MARK: Color32 = Color32::from_rgba_premultiplied(170, 170, 170, 170);

fn disc(painter: &egui::Painter, c: Pos2, r: f32, pressed: bool) {
    let a = if pressed { PRESSED } else { BASE };
    painter.circle_filled(c, r, Color32::from_white_alpha(a));
    painter.circle_stroke(c, r, Stroke::new(2.0, EDGE));
}

/// A triangle pointing in `dir` (a unit vector) at `c`.
fn arrow(painter: &egui::Painter, c: Pos2, dir: Vec2, size: f32) {
    let side = Vec2::new(-dir.y, dir.x);
    let pts = vec![
        c + dir * size,
        c - dir * size * 0.6 + side * size,
        c - dir * size * 0.6 - side * size,
    ];
    painter.add(egui::Shape::convex_polygon(pts, MARK, Stroke::NONE));
}

/// Draws the d-pad and the jump button; pressed parts are highlighted.
pub(in crate::app) fn draw_controls(painter: &egui::Painter, l: &Layout, held: Buttons) {
    if !l.controls {
        return;
    }
    let (c, r) = (l.dpad_center, l.dpad_radius);
    disc(painter, c, r, false);
    let s = r * 0.16;
    for (dir, on) in [
        (Vec2::new(-1.0, 0.0), held.left),
        (Vec2::new(1.0, 0.0), held.right),
        (Vec2::new(0.0, -1.0), held.up),
        (Vec2::new(0.0, 1.0), held.down),
    ] {
        let at = c + dir * r * 0.62;
        if on {
            painter.circle_filled(at, r * 0.3, Color32::from_white_alpha(PRESSED));
        }
        arrow(painter, at, dir, s);
    }
    disc(painter, l.jump_center, l.jump_radius, held.jump);
    arrow(
        painter,
        l.jump_center,
        Vec2::new(0.0, -1.0),
        l.jump_radius * 0.32,
    );
}

/// Draws the pause button: two bars while playing, a triangle while paused.
pub(in crate::app) fn draw_pause_button(
    painter: &egui::Painter,
    rect: egui::Rect,
    paused: bool,
    hovered: bool,
) {
    let a = if hovered { PRESSED } else { BASE };
    painter.rect_filled(rect, 6.0, Color32::from_white_alpha(a));
    painter.rect_stroke(rect, 6.0, Stroke::new(1.5, EDGE), egui::StrokeKind::Inside);
    let c = rect.center();
    let u = rect.width() * 0.2;
    if paused {
        arrow(painter, c, Vec2::new(1.0, 0.0), u * 0.9);
    } else {
        for dx in [-0.45, 0.45] {
            painter.rect_filled(
                egui::Rect::from_center_size(
                    c + Vec2::new(dx * u * 2.0, 0.0),
                    Vec2::new(u * 0.7, u * 2.0),
                ),
                1.0,
                MARK,
            );
        }
    }
}
