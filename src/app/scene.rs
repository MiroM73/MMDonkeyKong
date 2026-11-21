//! Draws the playfield: the static background (girders, ladders) rasterised once per level into
//! a texture, then the sprites of the hero, ape, barrels, hammers and effects on top.

use crate::app::sprites::{Id, Sprites};
use crate::app::*;
use crate::game::{BarrelMode, Geometry, Ladder, Mode, Player};
use crate::game::{FIELD_H, FIELD_W, GameState};

const CLEAR: egui::Color32 = egui::Color32::TRANSPARENT;
const GIRDER_TOP: egui::Color32 = egui::Color32::from_rgb(255, 130, 140);
const GIRDER_BODY: egui::Color32 = egui::Color32::from_rgb(200, 60, 80);
const GIRDER_LOW: egui::Color32 = egui::Color32::from_rgb(150, 40, 65);
const GIRDER_EDGE: egui::Color32 = egui::Color32::from_rgb(100, 25, 50);
const RIVET: egui::Color32 = egui::Color32::from_rgb(255, 215, 120);
const LADDER: egui::Color32 = egui::Color32::from_rgb(90, 200, 230);
/// Rows of a girder below its walking surface.
const GIRDER_THICK: i32 = 8;
/// Steps per frame of the animations.
const WALK_STEPS: u64 = 6;
const HAMMER_SWING_STEPS: u64 = 8;
const BEAT_STEPS: u64 = 20;
const FIRE_STEPS: u64 = 8;
/// How long a smash explosion lasts, in steps; it shows `Id::Explosion*` in thirds.
pub(in crate::app) const EXPLOSION_STEPS: u32 = 18;

/// A short effect on the playfield (a smashed barrel).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::app) struct Effect {
    pub x: f32,
    pub y: f32,
    pub age: u32,
}

/// The playfield pixels of the girders and ladders (row by row, `FIELD_W` x `FIELD_H`),
/// transparent elsewhere. Ladders are drawn first: the girder bodies cover their ends.
pub(in crate::app) fn render_background(geo: &Geometry) -> Vec<egui::Color32> {
    let (w, h) = (FIELD_W as usize, FIELD_H as usize);
    let mut px = vec![CLEAR; w * h];
    let mut put = |x: i32, y: i32, c: egui::Color32| {
        if (0..FIELD_W).contains(&x) && (0..FIELD_H).contains(&y) {
            px[y as usize * w + x as usize] = c;
        }
    };
    for l in &geo.ladders {
        draw_ladder(l, &mut put);
    }
    for g in &geo.girders {
        for x in g.x0.floor() as i32..g.x1.ceil() as i32 {
            let Some(surface) = g.y_at(x as f32 + 0.5) else {
                continue;
            };
            let top = surface.round() as i32;
            for r in 0..GIRDER_THICK {
                let c = match r {
                    0 => GIRDER_TOP,
                    3 if x.rem_euclid(8) == 4 => RIVET,
                    r if r == GIRDER_THICK - 1 => GIRDER_EDGE,
                    r if r >= GIRDER_THICK - 3 => GIRDER_LOW,
                    _ => GIRDER_BODY,
                };
                put(x, top + r, c);
            }
        }
    }
    px
}

fn draw_ladder(l: &Ladder, put: &mut impl FnMut(i32, i32, egui::Color32)) {
    let (top, bottom) = (l.top.round() as i32, l.bottom.round() as i32);
    let len = (bottom - top).max(1);
    let x = l.x.round() as i32;
    for y in top..bottom {
        // A broken ladder misses its middle third.
        let frac = (y - top) as f32 / len as f32;
        if l.broken && (0.35..0.65).contains(&frac) {
            continue;
        }
        put(x - 3, y, LADDER);
        put(x + 3, y, LADDER);
        if (y - top).rem_euclid(4) == 2 {
            for dx in -2..=2 {
                put(x + dx, y, LADDER);
            }
        }
    }
}

/// The sprite of the hero and whether it is mirrored. `moving`: the hero changed its position in
/// the last step (a standing hero shows no walk cycle).
pub(in crate::app) fn hero_sprite(p: &Player, tick: u64, moving: bool, hammer: bool) -> (Id, bool) {
    let flip = p.facing < 0;
    match p.mode {
        Mode::Dead { .. } => (Id::HeroDead, (tick / 8) % 2 == 1),
        Mode::Airborne { .. } => (Id::HeroJump, flip),
        // The climb mirrors the hero every few pixels (hands and feet swap).
        Mode::Climbing { .. } => (
            Id::HeroClimb,
            ((p.y / 4.0).floor() as i64).rem_euclid(2) == 1,
        ),
        Mode::Walking if hammer => (Id::HeroHammer, flip),
        Mode::Walking if !moving => (Id::HeroStand, flip),
        Mode::Walking => {
            if (tick / WALK_STEPS).is_multiple_of(2) {
                (Id::HeroWalk1, flip)
            } else {
                (Id::HeroWalk2, flip)
            }
        }
    }
}

/// The ape: throwing right after a barrel left, otherwise beating its chest.
pub(in crate::app) fn ape_sprite(throwing: bool, tick: u64) -> Id {
    if throwing {
        Id::ApeThrow
    } else if (tick / BEAT_STEPS).is_multiple_of(2) {
        Id::ApeIdle
    } else {
        Id::ApeBeat
    }
}

/// The sprite of a barrel: the roll follows its position, a fall turns with time.
pub(in crate::app) fn barrel_sprite(mode: BarrelMode, x: f32, tick: u64) -> Id {
    const ROLL: [Id; 4] = [Id::Barrel0, Id::Barrel1, Id::Barrel2, Id::Barrel3];
    match mode {
        BarrelMode::Rolling { .. } => ROLL[((x / 3.0).floor() as i64).rem_euclid(4) as usize],
        BarrelMode::Falling { .. } => ROLL[((tick / 4) % 4) as usize],
        BarrelMode::Descending { .. } => Id::BarrelFront,
    }
}

/// The frame of an explosion of this age.
pub(in crate::app) fn explosion_sprite(age: u32) -> Id {
    match age * 3 / EXPLOSION_STEPS {
        0 => Id::Explosion0,
        1 => Id::Explosion1,
        _ => Id::Explosion2,
    }
}

/// Maps playfield pixels to the screen.
pub(in crate::app) struct View<'a> {
    pub painter: &'a egui::Painter,
    pub origin: egui::Pos2,
    /// Screen points per playfield pixel.
    pub unit: f32,
}

impl View<'_> {
    /// Draws `tex` with its top-left corner at the playfield pixel (`x`, `y`).
    fn blit(&self, tex: &egui::TextureHandle, x: i32, y: i32, flip: bool) {
        let [w, h] = tex.size();
        let min = self.origin + egui::vec2(x as f32, y as f32) * self.unit;
        let rect = egui::Rect::from_min_size(min, egui::vec2(w as f32, h as f32) * self.unit);
        let (u0, u1) = if flip { (1.0, 0.0) } else { (0.0, 1.0) };
        let uv = egui::Rect::from_min_max(egui::pos2(u0, 0.0), egui::pos2(u1, 1.0));
        self.painter.image(tex.id(), rect, uv, egui::Color32::WHITE);
    }
}

/// Everything the scene needs besides the game state.
pub(in crate::app) struct Look<'a> {
    pub sprites: &'a Sprites,
    pub background: &'a egui::TextureHandle,
    pub effects: &'a [Effect],
    pub hero_moving: bool,
}

/// Draws the whole playfield (without the HUD).
pub(in crate::app) fn draw_scene(view: &View, st: &GameState, look: &Look) {
    let sp = look.sprites;
    let tick = st.tick;
    let blit = |id: Id, x: f32, y: f32, flip: bool| {
        view.blit(sp.get(id), x.round() as i32, y.round() as i32, flip);
    };

    view.blit(look.background, 0, 0, false);

    // The friend at the goal and the hammer items.
    let (gx, gy) = st.level.goal;
    blit(Id::Friend, gx - 6.0, gy - 16.0, false);
    for &(x, y) in &st.hammers {
        blit(Id::HammerUp, x - 4.0, y - 10.0, false);
    }
    let (ax, ay) = st.level.ape;
    blit(
        ape_sprite(st.ape_throwing(), tick),
        ax - 12.0,
        ay - 24.0,
        false,
    );
    if st.ape_throwing() {
        blit(Id::BarrelFront, ax + 4.0, ay - 12.0, false);
    }
    for b in &st.barrels {
        // The barrel position is its centre; the sprite is 8 x 8.
        blit(
            barrel_sprite(b.mode, b.x, tick),
            b.x - 4.0,
            b.y - 8.0,
            false,
        );
    }

    // The hero (a blinking one is not needed: death is its own sprite).
    let p = &st.player;
    let (id, flip) = hero_sprite(p, tick, look.hero_moving, st.hammer_left > 0);
    blit(id, p.x - 5.0, p.y - 12.0, flip);
    if st.hammer_left > 0 && !p.is_dead() {
        let dir = f32::from(p.facing.signum().max(-1));
        let (id, w, rise) = if (tick / HAMMER_SWING_STEPS).is_multiple_of(2) {
            (Id::HammerUp, 8.0, 17.0)
        } else {
            (Id::HammerSide, 10.0, 10.0)
        };
        // Right-facing: the handle starts 3 px (side) or 0 px (raised) in front of the centre.
        let from = if id == Id::HammerSide { 3.0 } else { 0.0 };
        let x = if dir > 0.0 {
            p.x + from
        } else {
            p.x - from - w
        };
        blit(id, x, p.y - rise, p.facing < 0);
    }

    for e in look.effects {
        blit(explosion_sprite(e.age), e.x - 5.0, e.y - 5.0, false);
    }
    // Fireballs have no source yet (#14); keep the frames reachable for the tests.
    let _ = (Id::Fire0, Id::Fire1, FIRE_STEPS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::{Girder, Ladder};

    fn geo() -> Geometry {
        Geometry {
            girders: vec![Girder::new(0.0, 100.0, 100.0, 100.0)],
            ladders: vec![
                Ladder {
                    x: 50.0,
                    top: 100.0,
                    bottom: 140.0,
                    broken: false,
                },
                Ladder {
                    x: 80.0,
                    top: 100.0,
                    bottom: 140.0,
                    broken: true,
                },
            ],
        }
    }

    fn at(px: &[egui::Color32], x: i32, y: i32) -> egui::Color32 {
        px[y as usize * FIELD_W as usize + x as usize]
    }

    #[test]
    fn the_background_has_girders_and_ladders() {
        let px = render_background(&geo());
        assert_eq!(px.len(), (FIELD_W * FIELD_H) as usize);
        assert_eq!(at(&px, 10, 100), GIRDER_TOP);
        assert_eq!(at(&px, 10, 99), CLEAR);
        assert_eq!(at(&px, 10, 100 + GIRDER_THICK), CLEAR);
        // A ladder rail below the girder, and nothing beside it.
        assert_eq!(at(&px, 47, 120), LADDER);
        assert_eq!(at(&px, 40, 120), CLEAR);
    }

    #[test]
    fn a_broken_ladder_misses_its_middle() {
        let px = render_background(&geo());
        assert_eq!(at(&px, 77, 112), LADDER);
        assert_eq!(at(&px, 77, 120), CLEAR);
        assert_eq!(at(&px, 47, 120), LADDER);
    }

    #[test]
    fn a_slope_follows_the_surface() {
        let g = Geometry {
            girders: vec![Girder::new(0.0, 60.0, 100.0, 80.0)],
            ladders: vec![],
        };
        let px = render_background(&g);
        assert_eq!(at(&px, 0, 60), GIRDER_TOP);
        assert_eq!(at(&px, 99, 80), GIRDER_TOP);
        assert_eq!(at(&px, 50, 70), GIRDER_TOP);
    }

    #[test]
    fn the_hero_animation_follows_the_mode() {
        let mut p = Player::new((20.0, 100.0));
        assert_eq!(hero_sprite(&p, 0, false, false).0, Id::HeroStand);
        assert_eq!(hero_sprite(&p, 0, true, false).0, Id::HeroWalk1);
        assert_eq!(hero_sprite(&p, WALK_STEPS, true, false).0, Id::HeroWalk2);
        assert_eq!(hero_sprite(&p, 0, true, true).0, Id::HeroHammer);
        p.facing = -1;
        assert!(hero_sprite(&p, 0, false, false).1);
        p.mode = Mode::Airborne {
            vx: 0.0,
            vy: 0.0,
            top: 0.0,
        };
        assert_eq!(hero_sprite(&p, 0, false, false).0, Id::HeroJump);
        p.mode = Mode::Climbing { ladder: 0 };
        let a = hero_sprite(&p, 0, false, false);
        p.y += 4.0;
        let b = hero_sprite(&p, 0, false, false);
        assert_eq!((a.0, b.0), (Id::HeroClimb, Id::HeroClimb));
        assert_ne!(a.1, b.1, "hands and feet swap while climbing");
        p.mode = Mode::Dead { left: 10 };
        assert_eq!(hero_sprite(&p, 0, false, false).0, Id::HeroDead);
    }

    #[test]
    fn barrels_and_explosions_pick_frames() {
        let roll = BarrelMode::Rolling { dir: 1 };
        assert_ne!(barrel_sprite(roll, 0.0, 0), barrel_sprite(roll, 3.0, 0));
        assert_eq!(barrel_sprite(roll, -1.0, 0), Id::Barrel3);
        assert_eq!(
            barrel_sprite(BarrelMode::Descending { ladder: 0 }, 5.0, 0),
            Id::BarrelFront
        );
        assert_eq!(explosion_sprite(0), Id::Explosion0);
        assert_eq!(explosion_sprite(EXPLOSION_STEPS - 1), Id::Explosion2);
    }

    #[test]
    fn the_ape_throws_and_beats() {
        assert_eq!(ape_sprite(true, 0), Id::ApeThrow);
        assert_eq!(ape_sprite(false, 0), Id::ApeIdle);
        assert_eq!(ape_sprite(false, BEAT_STEPS), Id::ApeBeat);
    }
}
