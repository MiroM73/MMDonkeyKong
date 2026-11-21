//! The game view: runs the fixed-step simulation from the frame time, collects the keyboard
//! input, draws the playfield scaled to the available area, handles pause.

use crate::app::scene::{self, Effect, Look, View};
use crate::app::*;
use crate::game::{
    self, Event, FIELD_H, FIELD_W, FixedStep, GameState, Input, Status, builtin_level,
};

const LETTERBOX: egui::Color32 = egui::Color32::from_rgb(8, 8, 14);
const FIELD_BG: egui::Color32 = egui::Color32::from_rgb(18, 20, 48);
const FIELD_BORDER: egui::Color32 = egui::Color32::from_rgb(200, 60, 60);
const HUD: egui::Color32 = egui::Color32::from_rgb(235, 235, 235);
/// A frame that took longer than this (hidden browser tab, computer sleep, window drag) pauses
/// the game instead of silently continuing.
const STALL_PAUSE_SECS: f32 = 0.5;

/// The running game and the state of its loop.
pub(in crate::app) struct GameSession {
    state: GameState,
    stepper: FixedStep,
    paused: bool,
    /// The jump key went down and no simulation step has consumed it yet (a frame without a
    /// step must not lose it; a frame with two steps delivers it once).
    jump_latch: bool,
    /// The first frame after a pause: its frame time contains the whole pause, so it is dropped.
    skip_dt: bool,
    /// The static background of the level (girders, ladders), built on first draw.
    background: Option<egui::TextureHandle>,
    /// Smash explosions on the field.
    effects: Vec<Effect>,
    /// The hero moved in the last step (walk animation).
    hero_moving: bool,
}

impl GameSession {
    pub(in crate::app) fn new(seed: u64) -> Self {
        Self {
            state: GameState::new(builtin_level(0), seed),
            stepper: FixedStep::default(),
            paused: false,
            jump_latch: false,
            skip_dt: false,
            background: None,
            effects: Vec::new(),
            hero_moving: false,
        }
    }

    fn set_paused(&mut self, paused: bool) {
        if self.paused != paused {
            self.paused = paused;
            self.stepper.reset();
            self.jump_latch = false;
            self.skip_dt = !paused;
        }
    }

    pub(in crate::app) fn is_paused(&self) -> bool {
        self.paused
    }
}

/// The scale of the playfield in physical pixels per logical pixel: the biggest whole number
/// that fits (crisp pixel art), or a fraction when the area is smaller than the field itself.
pub(in crate::app) fn field_scale(avail_w: f32, avail_h: f32) -> f32 {
    // `f32::min` ignores a NaN, so check the inputs first.
    if !(avail_w.is_finite() && avail_h.is_finite() && avail_w > 0.0 && avail_h > 0.0) {
        return 0.0;
    }
    let fit = (avail_w / FIELD_W as f32).min(avail_h / FIELD_H as f32);
    if fit >= 1.0 { fit.floor() } else { fit }
}

/// Keyboard state of this frame: held directions and the jump edge.
fn read_input(ctx: &egui::Context) -> (Input, bool) {
    ctx.input(|i| {
        let held = |keys: &[egui::Key]| keys.iter().any(|&k| i.key_down(k));
        let input = Input {
            left: held(&[egui::Key::ArrowLeft, egui::Key::A]),
            right: held(&[egui::Key::ArrowRight, egui::Key::D]),
            up: held(&[egui::Key::ArrowUp, egui::Key::W]),
            down: held(&[egui::Key::ArrowDown, egui::Key::S]),
            jump_held: i.key_down(egui::Key::Space),
            jump_pressed: false,
        };
        (input, i.key_pressed(egui::Key::Space))
    })
}

impl App {
    /// Pause keys and losing the window focus; returns after updating `game.paused`.
    fn handle_pause(&mut self, ctx: &egui::Context) {
        let (toggle, focused, dt) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::P),
                i.focused,
                i.unstable_dt,
            )
        });
        let paused = self.game.is_paused();
        let stalled = dt > STALL_PAUSE_SECS && !self.game.skip_dt;
        if toggle {
            self.game.set_paused(!paused);
        } else if (!focused || stalled) && !paused {
            self.game.set_paused(true);
        }
    }

    /// Runs the simulation steps due in this frame.
    fn run_steps(&mut self, ctx: &egui::Context) {
        let (mut input, jump_edge) = read_input(ctx);
        self.game.jump_latch |= jump_edge;
        let dt = if std::mem::take(&mut self.game.skip_dt) {
            0.0
        } else {
            ctx.input(|i| i.unstable_dt)
        };
        for _ in 0..self.game.stepper.advance(dt) {
            input.jump_pressed = std::mem::take(&mut self.game.jump_latch);
            let (px, py) = (self.game.state.player.x, self.game.state.player.y);
            let events = game::step(&mut self.game.state, &input);
            let p = &self.game.state.player;
            self.game.hero_moving = p.x != px || p.y != py;
            for e in &mut self.game.effects {
                e.age += 1;
            }
            self.game.effects.retain(|e| e.age < scene::EXPLOSION_STEPS);
            for event in events {
                // Sounds follow with issue #16.
                if event == Event::Smash {
                    let r = self.game.state.hammer_rect();
                    self.game.effects.push(Effect {
                        x: (r.x0 + r.x1) / 2.0,
                        y: (r.y0 + r.y1) / 2.0,
                        age: 0,
                    });
                }
            }
        }
    }

    pub(in crate::app) fn game_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.game.state.status != Status::Playing
            && ctx.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::R))
        {
            // Game over starts again with the score at zero; after a completed level there is
            // no next level yet (#12), so the same one is replayed.
            // The frame time makes every new game differ (a fixed seed would repeat the barrels).
            self.game = GameSession::new(ctx.input(|i| (i.time * 1000.0) as u64));
            // A finished game requests no repaints, so this frame's time is the whole idle wait
            // (it would look like a stall and pause the new game): drop it.
            self.game.skip_dt = true;
        }
        self.handle_pause(&ctx);
        if !self.game.is_paused() && self.game.state.status == Status::Playing {
            self.run_steps(&ctx);
            // The game animates every frame; a paused game requests nothing (idle CPU).
            ctx.request_repaint();
        }

        let (resp, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::hover());
        // A pointer click only: `Sense::click` would also react to Enter / Space on a focused view.
        if resp.hovered() && ctx.input(|i| i.pointer.primary_clicked()) {
            let paused = self.game.is_paused();
            self.game.set_paused(!paused);
            // This frame already passed the repaint request above: ask again after an unpause.
            ctx.request_repaint();
        }
        painter.rect_filled(resp.rect, 0.0, LETTERBOX);

        // Integer scale in physical pixels, then back to points; the field is centred and
        // snapped to whole physical pixels so the pixel art stays sharp at any DPI.
        let ppp = ctx.pixels_per_point();
        let scale = field_scale(resp.rect.width() * ppp, resp.rect.height() * ppp);
        if scale <= 0.0 {
            return;
        }
        let size = egui::vec2(FIELD_W as f32, FIELD_H as f32) * scale / ppp;
        let snap = |v: f32| (v * ppp).round() / ppp;
        let origin = egui::pos2(
            snap(resp.rect.center().x - size.x / 2.0),
            snap(resp.rect.center().y - size.y / 2.0),
        );
        let field = egui::Rect::from_min_size(origin, size);
        let to_screen = |x: f32, y: f32| origin + egui::vec2(x, y) * scale / ppp;

        painter.rect_filled(field, 0.0, FIELD_BG);
        painter.rect_stroke(
            field,
            0.0,
            egui::Stroke::new(scale / ppp, FIELD_BORDER),
            egui::StrokeKind::Inside,
        );
        let st = &self.game.state;
        let background = self.game.background.get_or_insert_with(|| {
            let image = egui::ColorImage::new(
                [FIELD_W as usize, FIELD_H as usize],
                scene::render_background(&st.level.geometry),
            );
            ctx.load_texture("background", image, egui::TextureOptions::NEAREST)
        });
        let view = View {
            painter: &painter,
            origin,
            unit: scale / ppp,
        };
        let look = Look {
            sprites: &self.sprites,
            background,
            effects: &self.game.effects,
            hero_moving: self.game.hero_moving,
        };
        scene::draw_scene(&view, st, &look);
        let hud = egui::FontId::monospace(8.0 * scale / ppp);
        painter.text(
            to_screen(4.0, 2.0),
            egui::Align2::LEFT_TOP,
            t!("game.score", score = st.score),
            hud.clone(),
            HUD,
        );
        painter.text(
            to_screen(FIELD_W as f32 / 2.0, 2.0),
            egui::Align2::CENTER_TOP,
            t!("game.bonus", bonus = st.bonus),
            hud.clone(),
            HUD,
        );
        painter.text(
            to_screen(FIELD_W as f32 - 4.0, 2.0),
            egui::Align2::RIGHT_TOP,
            t!("game.lives", lives = st.lives),
            hud,
            HUD,
        );
        let banner = match st.status {
            Status::Playing => None,
            Status::Complete => Some(t!("game.complete", score = st.score)),
            Status::GameOver => Some(t!("game.over")),
        };
        if let Some(text) = banner {
            painter.rect_filled(field, 0.0, egui::Color32::from_black_alpha(150));
            painter.text(
                field.center(),
                egui::Align2::CENTER_CENTER,
                format!(
                    "{text}
{}",
                    t!("game.restart")
                ),
                egui::FontId::proportional(16.0 * scale / ppp / 2.0),
                egui::Color32::WHITE,
            );
        }

        if self.game.is_paused() {
            painter.rect_filled(field, 0.0, egui::Color32::from_black_alpha(150));
            painter.text(
                field.center(),
                egui::Align2::CENTER_CENTER,
                t!("game.paused"),
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_is_whole_when_the_field_fits() {
        assert_eq!(field_scale(1000.0, 1000.0), 3.0); // 1000 / 256 = 3.9 -> 3
        assert_eq!(field_scale(224.0, 256.0), 1.0);
        assert_eq!(field_scale(2000.0, 700.0), 2.0); // limited by the height
    }

    #[test]
    fn scale_is_fractional_in_a_small_area() {
        let s = field_scale(112.0, 500.0);
        assert!((s - 0.5).abs() < 1e-6, "{s}");
    }

    #[test]
    fn empty_or_bad_area_gives_zero() {
        assert_eq!(field_scale(0.0, 100.0), 0.0);
        assert_eq!(field_scale(f32::NAN, 100.0), 0.0);
        assert_eq!(field_scale(-5.0, 100.0), 0.0);
    }
}
