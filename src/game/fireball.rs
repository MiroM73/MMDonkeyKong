//! Fireballs of the rivets level: they walk along a girder, turn at its ends, climb intact
//! ladders and prefer the direction of the player. Pure and deterministic (seeded `Rng`).

use super::difficulty::Params;
use super::player::SNAP;
use super::rng::Rng;
use super::{FIELD_H, Geometry, Rect};

/// Walking speed in pixels per step (at obstacle speed 1.0).
pub const FIRE_WALK_SPEED: f32 = 0.45;
/// Climbing speed in pixels per step.
pub const FIRE_CLIMB_SPEED: f32 = 0.5;
/// Size of the hit box: `x` is the middle, `y` the bottom.
const HALF_W: f32 = 3.5;
const HEIGHT: f32 = 9.0;
/// Every this many steps a walking fireball may turn towards the player.
const TURN_EVERY: u64 = 45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireMode {
    Walking {
        dir: i8,
    },
    /// On ladder number `ladder`, going up (`-1`) or down (`1`).
    Climbing {
        ladder: usize,
        dir: i8,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fireball {
    pub x: f32,
    pub y: f32,
    pub mode: FireMode,
}

impl Fireball {
    /// A fireball standing at (`x`, `y`) on a girder, walking in direction `dir`.
    pub fn new(x: f32, y: f32, dir: i8) -> Self {
        Self {
            x,
            y,
            mode: FireMode::Walking { dir },
        }
    }

    pub fn rect(&self) -> Rect {
        Rect {
            x0: self.x - HALF_W,
            y0: self.y - HEIGHT,
            x1: self.x + HALF_W,
            y1: self.y,
        }
    }

    /// The direction it looks: -1 left, 1 right (a climbing fireball looks right).
    pub fn mode_dir(&self) -> i8 {
        match self.mode {
            FireMode::Walking { dir } => dir,
            FireMode::Climbing { .. } => 1,
        }
    }

    pub fn is_gone(&self) -> bool {
        self.y > FIELD_H as f32 + 16.0
    }

    /// One step. `target` is where the player is (it attracts the fireball), `tick` the step
    /// counter of the game.
    pub fn step(
        &mut self,
        geo: &Geometry,
        rng: &mut Rng,
        p: &Params,
        target: (f32, f32),
        tick: u64,
    ) {
        match self.mode {
            FireMode::Walking { dir } => self.walk(geo, rng, p, dir, target, tick),
            FireMode::Climbing { ladder, dir } => {
                let l = geo.ladders[ladder];
                self.y += f32::from(dir) * FIRE_CLIMB_SPEED * p.obstacle_speed;
                let end = if dir < 0 { l.top } else { l.bottom };
                if (dir < 0 && self.y <= end) || (dir > 0 && self.y >= end) {
                    self.y = end;
                    let towards = if target.0 >= self.x { 1 } else { -1 };
                    self.mode = FireMode::Walking { dir: towards };
                    if let Some(s) = geo.ground_near(self.x, self.y) {
                        self.y = s;
                    }
                }
            }
        }
    }

    fn walk(
        &mut self,
        geo: &Geometry,
        rng: &mut Rng,
        p: &Params,
        dir: i8,
        target: (f32, f32),
        tick: u64,
    ) {
        let mut dir = dir;
        // Now and then turn towards the player (two out of three times), else keep going.
        if tick.is_multiple_of(TURN_EVERY) && rng.below(3) > 0 {
            dir = if target.0 >= self.x { 1 } else { -1 };
        }
        let old_x = self.x;
        let new_x = self.x + f32::from(dir) * FIRE_WALK_SPEED * p.obstacle_speed;
        match geo.ground_near(new_x, self.y) {
            Some(s) => {
                self.x = new_x;
                self.y = s;
            }
            // The end of the girder: turn round.
            None => dir = -dir,
        }
        self.mode = FireMode::Walking { dir };

        // Passing a ladder: take it when it leads towards the player's floor.
        let crossed = geo.ladders.iter().position(|l| {
            !l.broken
                && ((old_x < l.x && self.x >= l.x) || (old_x > l.x && self.x <= l.x))
                && ((self.y - l.top).abs() <= SNAP || (self.y - l.bottom).abs() <= SNAP)
        });
        if let Some(i) = crossed {
            let l = geo.ladders[i];
            let at_top = (self.y - l.top).abs() <= SNAP;
            let wants_down = target.1 > self.y + 8.0;
            let wants_up = target.1 < self.y - 8.0;
            if (at_top && wants_down || !at_top && wants_up) && rng.below(2) == 0 {
                self.x = l.x;
                self.mode = FireMode::Climbing {
                    ladder: i,
                    dir: if at_top { 1 } else { -1 },
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::difficulty::Difficulty;
    use super::super::player::{Girder, Ladder};
    use super::*;

    fn geo() -> Geometry {
        Geometry {
            girders: vec![
                Girder::new(0.0, 100.0, 100.0, 100.0),
                Girder::new(0.0, 60.0, 100.0, 60.0),
            ],
            ladders: vec![Ladder {
                x: 50.0,
                top: 60.0,
                bottom: 100.0,
                broken: false,
            }],
        }
    }

    fn run(f: &mut Fireball, g: &Geometry, target: (f32, f32), steps: u64) {
        let mut rng = Rng::new(1);
        let p = Difficulty::Normal.params();
        for t in 1..=steps {
            f.step(g, &mut rng, &p, target, t);
        }
    }

    #[test]
    fn it_turns_at_the_end_of_the_girder() {
        let g = geo();
        let mut f = Fireball::new(95.0, 100.0, 1);
        // The player stands on the floor to the right, so it never wants to climb.
        run(&mut f, &g, (400.0, 100.0), 60);
        assert!(f.x <= 100.0 && f.x >= 0.0);
        assert!(matches!(f.mode, FireMode::Walking { dir: -1 }));
    }

    #[test]
    fn it_climbs_towards_a_player_on_a_higher_floor() {
        let g = geo();
        let mut f = Fireball::new(30.0, 100.0, 1);
        let mut rng = Rng::new(2);
        let p = Difficulty::Normal.params();
        let mut climbed = false;
        for t in 1..=3000 {
            f.step(&g, &mut rng, &p, (50.0, 60.0), t);
            climbed |= f.y < 99.0;
        }
        assert!(climbed);
    }

    #[test]
    fn it_never_leaves_its_floor_when_the_player_is_on_it() {
        let g = geo();
        let mut f = Fireball::new(30.0, 100.0, 1);
        run(&mut f, &g, (10.0, 100.0), 2000);
        assert_eq!(f.y, 100.0);
    }

    #[test]
    fn it_walks_towards_the_player() {
        let g = geo();
        let mut f = Fireball::new(50.0, 100.0, -1);
        run(&mut f, &g, (90.0, 100.0), 300);
        assert!(f.x > 60.0, "{}", f.x);
    }
}
