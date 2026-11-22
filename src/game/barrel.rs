//! Rolling barrels: they roll downhill along a girder, sometimes climb down an intact ladder
//! (random, from the seeded `Rng`) and fall off the girder ends. Pure and deterministic.

use super::difficulty::Params;
use super::player::SNAP;
use super::rng::Rng;
use super::{FIELD_H, Geometry, Rect};

/// Rolling / descending speed in pixels per step (at obstacle speed 1.0).
pub const ROLL_SPEED: f32 = 0.8;
/// Horizontal speed while falling off a girder end.
pub const DROP_VX: f32 = 0.5;
const GRAVITY: f32 = 0.15;
const MAX_FALL_SPEED: f32 = 5.0;
/// Barrel size: a square of `2 * RADIUS`, `x` / `y` are the middle of the bottom edge.
pub const RADIUS: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BarrelMode {
    /// Rolling on a girder in direction `dir` (-1 / 1).
    Rolling { dir: i8 },
    /// Climbing down ladder number `ladder`; `dir` is kept for the roll afterwards.
    Descending { ladder: usize },
    /// Falling; `vx` is horizontal drift, `dir` the roll direction if the girder below is flat.
    Falling { vx: f32, vy: f32, dir: i8 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Barrel {
    pub x: f32,
    pub y: f32,
    pub mode: BarrelMode,
    /// The player has already been awarded the jump-over points for this barrel.
    pub jumped: bool,
}

impl Barrel {
    /// A barrel dropped at (`x`, `y`) that lands and then rolls in direction `dir`.
    pub fn thrown(x: f32, y: f32, dir: i8) -> Self {
        Self {
            x,
            y,
            mode: BarrelMode::Falling {
                vx: 0.0,
                vy: 0.0,
                dir,
            },
            jumped: false,
        }
    }

    /// Makes a rolling barrel go in direction `dir` (it is on a conveyor belt).
    pub fn ride(&mut self, dir: i8) {
        if let BarrelMode::Rolling { .. } = self.mode {
            self.mode = BarrelMode::Rolling { dir };
        }
    }

    pub fn rect(&self) -> Rect {
        Rect {
            x0: self.x - RADIUS,
            y0: self.y - 2.0 * RADIUS,
            x1: self.x + RADIUS,
            y1: self.y,
        }
    }

    /// Has the barrel left the playfield for good?
    pub fn is_gone(&self) -> bool {
        self.y > FIELD_H as f32 + 16.0 || self.x < -16.0 || self.x > super::FIELD_W as f32 + 16.0
    }

    /// One step; `p` scales the speed and sets the chance of climbing down a ladder.
    pub fn step(&mut self, geo: &Geometry, rng: &mut Rng, p: &Params) {
        match self.mode {
            BarrelMode::Rolling { dir } => self.roll(geo, rng, dir, p),
            BarrelMode::Descending { ladder } => {
                let l = geo.ladders[ladder];
                self.y += ROLL_SPEED * p.obstacle_speed;
                if self.y >= l.bottom {
                    self.y = l.bottom;
                    match geo.ground_near(self.x, self.y) {
                        Some(s) => {
                            self.y = s;
                            self.mode = BarrelMode::Rolling {
                                dir: downhill(geo, self.x, self.y, 1),
                            };
                        }
                        None => {
                            self.mode = BarrelMode::Falling {
                                vx: 0.0,
                                vy: 0.0,
                                dir: 1,
                            }
                        }
                    }
                }
            }
            BarrelMode::Falling { vx, vy, dir } => {
                let vy = (vy + GRAVITY).min(MAX_FALL_SPEED);
                let new_x = self.x + vx;
                let new_y = self.y + vy;
                if let Some(s) = geo.landing(new_x, self.y, new_y) {
                    self.x = new_x;
                    self.y = s;
                    self.mode = BarrelMode::Rolling {
                        dir: downhill(geo, self.x, s, dir),
                    };
                } else {
                    self.x = new_x;
                    self.y = new_y;
                    self.mode = BarrelMode::Falling { vx, vy, dir };
                }
            }
        }
    }

    fn roll(&mut self, geo: &Geometry, rng: &mut Rng, dir: i8, p: &Params) {
        let old_x = self.x;
        self.x += f32::from(dir) * ROLL_SPEED * p.obstacle_speed;
        let Some(s) = geo.ground_near(self.x, self.y) else {
            // Rolled off the end of the girder.
            self.mode = BarrelMode::Falling {
                vx: f32::from(dir) * DROP_VX * p.obstacle_speed,
                vy: 0.0,
                dir,
            };
            return;
        };
        self.y = s;
        // Rolling over the top of an intact ladder: sometimes take it.
        let crossed = geo.ladders.iter().position(|l| {
            !l.broken
                && (self.y - l.top).abs() <= SNAP
                && ((old_x < l.x && self.x >= l.x) || (old_x > l.x && self.x <= l.x))
        });
        if let Some(i) = crossed
            && rng.below(p.ladder_chance.1) < p.ladder_chance.0
        {
            self.x = geo.ladders[i].x;
            self.mode = BarrelMode::Descending { ladder: i };
        }
    }
}

/// The direction a barrel rolls on the girder at (`x`, `y`): downhill, or `fallback` on a flat one.
fn downhill(geo: &Geometry, x: f32, y: f32, fallback: i8) -> i8 {
    let slope = |dx: f32| {
        geo.ground_near(x + dx, y)
            .zip(geo.ground_near(x - dx, y))
            .map(|(a, b)| a - b)
    };
    match slope(2.0) {
        Some(d) if d > 0.01 => 1,
        Some(d) if d < -0.01 => -1,
        _ => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::super::difficulty::Difficulty;
    use super::super::player::{Girder, Ladder};
    use super::*;

    fn run(b: &mut Barrel, geo: &Geometry, rng: &mut Rng, steps: usize) {
        for _ in 0..steps {
            b.step(geo, rng, &Difficulty::Normal.params());
        }
    }

    fn slope_right() -> Geometry {
        Geometry {
            girders: vec![
                Girder::new(0.0, 100.0, 100.0, 110.0),
                Girder::new(0.0, 200.0, 200.0, 200.0),
            ],
            ladders: vec![],
        }
    }

    #[test]
    fn a_thrown_barrel_lands_and_rolls_downhill() {
        let geo = slope_right();
        let mut b = Barrel::thrown(50.0, 60.0, -1);
        let mut rng = Rng::new(1);
        run(&mut b, &geo, &mut rng, 40);
        // Landed on the sloped girder and rolls to the right (downhill), not to its throw side.
        assert!(
            matches!(b.mode, BarrelMode::Rolling { dir: 1 }),
            "{:?}",
            b.mode
        );
        let x = b.x;
        run(&mut b, &geo, &mut rng, 10);
        assert!(b.x > x);
        assert!((b.y - geo.girders[0].y_at(b.x).unwrap()).abs() < 1e-4);
    }

    #[test]
    fn a_barrel_falls_off_the_end_onto_the_girder_below() {
        let geo = slope_right();
        let mut b = Barrel::thrown(90.0, 90.0, 1);
        let mut rng = Rng::new(1);
        run(&mut b, &geo, &mut rng, 150);
        assert_eq!(b.y, 200.0);
        assert!(b.x > 100.0);
    }

    #[test]
    fn a_barrel_leaves_the_playfield_at_the_bottom() {
        let geo = Geometry {
            girders: vec![Girder::new(0.0, 100.0, 50.0, 110.0)],
            ladders: vec![],
        };
        let mut b = Barrel::thrown(40.0, 60.0, 1);
        let mut rng = Rng::new(1);
        run(&mut b, &geo, &mut rng, 600);
        assert!(b.is_gone());
    }

    #[test]
    fn barrels_take_ladders_sometimes_not_always_and_never_broken_ones() {
        let ladder = |broken| Geometry {
            girders: vec![
                Girder::new(0.0, 100.0, 100.0, 108.0),
                Girder::new(0.0, 160.0, 120.0, 160.0),
            ],
            ladders: vec![Ladder {
                x: 40.0,
                top: 103.2,
                bottom: 160.0,
                broken,
            }],
        };
        let (mut took, mut skipped) = (0, 0);
        for seed in 0..60 {
            let geo = ladder(false);
            let mut b = Barrel::thrown(10.0, 90.0, 1);
            let mut rng = Rng::new(seed);
            let mut climbed = false;
            for _ in 0..200 {
                b.step(&geo, &mut rng, &Difficulty::Normal.params());
                climbed |= matches!(b.mode, BarrelMode::Descending { .. });
            }
            if climbed {
                took += 1;
                assert_eq!(b.y, 160.0); // reached the lower girder
            } else {
                skipped += 1;
            }
        }
        assert!(took > 5 && skipped > 5, "took {took}, skipped {skipped}");

        let geo = ladder(true);
        for seed in 0..30 {
            let mut b = Barrel::thrown(10.0, 90.0, 1);
            let mut rng = Rng::new(seed);
            for _ in 0..200 {
                b.step(&geo, &mut rng, &Difficulty::Normal.params());
                assert!(!matches!(b.mode, BarrelMode::Descending { .. }));
            }
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_barrel_path() {
        let geo = slope_right();
        let path = |seed| {
            let mut b = Barrel::thrown(10.0, 60.0, 1);
            let mut rng = Rng::new(seed);
            (0..300)
                .map(|_| {
                    b.step(&geo, &mut rng, &Difficulty::Normal.params());
                    (b.x, b.y)
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(path(3), path(3));
    }

    #[test]
    fn the_obstacle_speed_scales_the_roll() {
        let geo = slope_right();
        let travelled = |d: Difficulty| {
            let mut b = Barrel::thrown(10.0, 60.0, 1);
            let mut rng = Rng::new(1);
            for _ in 0..40 {
                b.step(&geo, &mut rng, &Difficulty::Normal.params());
            }
            let x = b.x;
            for _ in 0..30 {
                b.step(&geo, &mut rng, &d.params());
            }
            b.x - x
        };
        let normal = travelled(Difficulty::Normal);
        assert!((normal - 30.0 * ROLL_SPEED).abs() < 1e-3, "{normal}");
        assert!(travelled(Difficulty::Relaxed) < normal);
        assert!(travelled(Difficulty::Insane) > normal);
    }

    #[test]
    fn rect_is_a_square_standing_on_y() {
        let b = Barrel::thrown(10.0, 50.0, 1);
        let r = b.rect();
        assert_eq!((r.x0, r.x1, r.y0, r.y1), (6.0, 14.0, 42.0, 50.0));
    }
}
