//! The player: walking along sloped girders, ladders, a fixed-arc jump, falling, death and respawn.
//! Pure and deterministic (no egui, no I/O). Coordinates are playfield pixels, y grows downwards,
//! `Player::x` / `Player::y` are the middle of the feet.

use super::input::Input;
use super::{FIELD_W, Rect};

/// Walking speed in pixels per step.
pub const WALK_SPEED: f32 = 1.0;
/// Climbing speed in pixels per step.
pub const CLIMB_SPEED: f32 = 0.6;
/// Initial vertical speed of a jump (negative = up).
pub const JUMP_VY: f32 = -2.6;
/// Gravity in pixels per step squared.
pub const GRAVITY: f32 = 0.15;
/// A fall faster than this is capped (the landing check is swept, so it never tunnels anyway).
pub const MAX_FALL_SPEED: f32 = 6.0;
/// Falling (or landing from a jump) more than this many pixels below the highest point kills.
pub const FALL_LIMIT: f32 = 32.0;
/// How far below the feet a girder still counts as the ground (slope following, edge snapping).
pub const SNAP: f32 = 4.0;
/// A ladder is grabbed when the player is at most this far from it horizontally.
pub const LADDER_REACH: f32 = 4.0;
/// A ladder can be grabbed in the air only when rising slower than this (not at take-off).
pub const GRAB_MIN_VY: f32 = -2.0;
/// Steps the death animation lasts before the player respawns.
pub const DEATH_STEPS: u32 = 90;
/// Forgiving hitbox of the player: half width and height above the feet.
pub const HIT_HALF_W: f32 = 3.0;
pub const HIT_HEIGHT: f32 = 12.0;

/// A straight girder from (`x0`, `y0`) to (`x1`, `y1`), `x0 < x1`; the walking surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Girder {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Girder {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    /// Surface height at `x`, `None` outside the girder.
    pub fn y_at(&self, x: f32) -> Option<f32> {
        if x < self.x0 || x > self.x1 {
            return None;
        }
        let t = (x - self.x0) / (self.x1 - self.x0);
        Some(self.y0 + (self.y1 - self.y0) * t)
    }
}

/// A vertical ladder at `x` between the girder surfaces `top` and `bottom` (`top < bottom`).
/// A broken ladder cannot be climbed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ladder {
    pub x: f32,
    pub top: f32,
    pub bottom: f32,
    pub broken: bool,
}

/// What the player can stand on and climb.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Geometry {
    pub girders: Vec<Girder>,
    pub ladders: Vec<Ladder>,
}

impl Geometry {
    /// The girder under (`x`, `y`) whose surface is nearest to `y` while not more than `SNAP`
    /// above it and `SNAP` below it; returns the surface height.
    pub(super) fn ground_near(&self, x: f32, y: f32) -> Option<f32> {
        self.girders
            .iter()
            .filter_map(|g| g.y_at(x))
            .filter(|s| (*s - y).abs() <= SNAP)
            .min_by(|a, b| (a - y).abs().total_cmp(&(b - y).abs()))
    }

    /// First surface crossed when moving from height `from` to `to` (`from <= to`) at `x`.
    pub(super) fn landing(&self, x: f32, from: f32, to: f32) -> Option<f32> {
        self.girders
            .iter()
            .filter_map(|g| g.y_at(x))
            .filter(|s| *s >= from && *s <= to)
            .min_by(f32::total_cmp)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    Walking,
    /// On ladder number `ladder` of the geometry.
    Climbing {
        ladder: usize,
    },
    /// In the air; `vx` is fixed at take-off (no air control), `top` the highest `y` reached.
    Airborne {
        vx: f32,
        vy: f32,
        top: f32,
    },
    /// Dead; respawns when `left` reaches zero.
    Dead {
        left: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerEvent {
    Jumped,
    Landed,
    /// Died by a fall (landed too far below the highest point).
    FellToDeath,
    /// Killed from outside (`Player::kill`).
    Killed,
    Respawned,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    /// -1 left, 1 right; where the player looks.
    pub facing: i8,
    pub mode: Mode,
    spawn: (f32, f32),
}

impl Player {
    /// A player standing at `spawn` (middle of the feet); `step` snaps it onto a girder.
    pub fn new(spawn: (f32, f32)) -> Self {
        Self {
            x: spawn.0,
            y: spawn.1,
            facing: 1,
            mode: Mode::Walking,
            spawn,
        }
    }

    pub fn is_dead(&self) -> bool {
        matches!(self.mode, Mode::Dead { .. })
    }

    pub fn is_walking(&self) -> bool {
        matches!(self.mode, Mode::Walking)
    }

    pub fn is_airborne(&self) -> bool {
        matches!(self.mode, Mode::Airborne { .. })
    }

    /// Small, forgiving hitbox: narrower than the sprite, a bit lower than its head.
    pub fn hitbox(&self) -> Rect {
        Rect {
            x0: self.x - HIT_HALF_W,
            y0: self.y - HIT_HEIGHT,
            x1: self.x + HIT_HALF_W,
            y1: self.y,
        }
    }

    /// Does `hazard` (already shrunk by the caller if it wants to be forgiving) touch the player?
    pub fn hit_by(&self, hazard: Rect) -> bool {
        !self.is_dead() && self.hitbox().overlaps(hazard)
    }

    /// Kills the player (barrel, fireball, ...); no effect when already dead.
    pub fn kill(&mut self) -> Option<PlayerEvent> {
        if self.is_dead() {
            return None;
        }
        self.mode = Mode::Dead { left: DEATH_STEPS };
        Some(PlayerEvent::Killed)
    }

    /// One fixed step.
    pub fn step(&mut self, input: &Input, geo: &Geometry) -> Option<PlayerEvent> {
        match self.mode {
            Mode::Dead { left } => {
                if left <= 1 {
                    (self.x, self.y) = self.spawn;
                    self.facing = 1;
                    self.mode = Mode::Walking;
                    self.settle(geo);
                    Some(PlayerEvent::Respawned)
                } else {
                    self.mode = Mode::Dead { left: left - 1 };
                    None
                }
            }
            Mode::Walking => self.walk(input, geo),
            Mode::Climbing { ladder } => self.climb(input, geo, ladder),
            Mode::Airborne { vx, vy, top } => self.fly(input, geo, vx, vy, top),
        }
    }

    /// Puts the player onto the nearest girder below the spawn point (or lets it fall).
    fn settle(&mut self, geo: &Geometry) {
        if let Some(s) = geo.landing(self.x, self.y - SNAP, f32::MAX) {
            self.y = s;
        } else {
            self.mode = Mode::Airborne {
                vx: 0.0,
                vy: 0.0,
                top: self.y,
            };
        }
    }

    fn walk(&mut self, input: &Input, geo: &Geometry) -> Option<PlayerEvent> {
        if input.jump_pressed {
            let dir = i8::from(input.right) - i8::from(input.left);
            if dir != 0 {
                self.facing = dir;
            }
            self.mode = Mode::Airborne {
                vx: f32::from(dir) * WALK_SPEED,
                vy: JUMP_VY,
                top: self.y,
            };
            return Some(PlayerEvent::Jumped);
        }
        if input.up != input.down
            && let Some(i) = self.ladder_to_grab(input, geo)
        {
            self.x = geo.ladders[i].x;
            self.mode = Mode::Climbing { ladder: i };
            return None;
        }
        let dir = i8::from(input.right) - i8::from(input.left);
        if dir != 0 {
            self.facing = dir;
            self.x = (self.x + f32::from(dir) * WALK_SPEED).clamp(0.0, FIELD_W as f32);
        }
        match geo.ground_near(self.x, self.y) {
            Some(s) => self.y = s,
            None => {
                // Walked off the edge: fall straight down.
                self.mode = Mode::Airborne {
                    vx: 0.0,
                    vy: 0.0,
                    top: self.y,
                };
            }
        }
        None
    }

    /// The ladder the pressed up / down key grabs: up from below, down from the top.
    fn ladder_to_grab(&self, input: &Input, geo: &Geometry) -> Option<usize> {
        geo.ladders.iter().position(|l| {
            !l.broken
                && (l.x - self.x).abs() <= LADDER_REACH
                && if input.up {
                    self.y > l.top + SNAP && self.y <= l.bottom + SNAP
                } else {
                    self.y >= l.top - SNAP && self.y < l.bottom - SNAP
                }
        })
    }

    fn climb(&mut self, input: &Input, geo: &Geometry, ladder: usize) -> Option<PlayerEvent> {
        if input.jump_pressed {
            // Jump off the ladder (sideways when a direction is held).
            let dir = i8::from(input.right) - i8::from(input.left);
            if dir != 0 {
                self.facing = dir;
            }
            self.mode = Mode::Airborne {
                vx: f32::from(dir) * WALK_SPEED,
                vy: JUMP_VY,
                top: self.y,
            };
            return Some(PlayerEvent::Jumped);
        }
        let l = geo.ladders[ladder];
        let dir = f32::from(i8::from(input.down) - i8::from(input.up));
        self.y += dir * CLIMB_SPEED;
        // Leaving at either end: step onto the girder there.
        if self.y <= l.top || self.y >= l.bottom {
            let end = if self.y <= l.top { l.top } else { l.bottom };
            self.y = end;
            if let Some(s) = geo.ground_near(self.x, end) {
                self.y = s;
                self.mode = Mode::Walking;
            }
        }
        None
    }

    fn fly(
        &mut self,
        input: &Input,
        geo: &Geometry,
        vx: f32,
        vy: f32,
        top: f32,
    ) -> Option<PlayerEvent> {
        // Grab a ladder in reach with up / down held, except right after take-off.
        if input.up != input.down
            && vy > GRAB_MIN_VY
            && let Some(i) = geo.ladders.iter().position(|l| {
                !l.broken
                    && (l.x - self.x).abs() <= LADDER_REACH
                    && self.y > l.top
                    && self.y < l.bottom
            })
        {
            self.x = geo.ladders[i].x;
            self.mode = Mode::Climbing { ladder: i };
            return None;
        }
        let vy = (vy + GRAVITY).min(MAX_FALL_SPEED);
        let new_x = (self.x + vx).clamp(0.0, FIELD_W as f32);
        let new_y = self.y + vy;
        let top = top.min(self.y);
        if vy > 0.0
            && let Some(s) = geo.landing(new_x, self.y, new_y)
        {
            self.x = new_x;
            self.y = s;
            if s - top > FALL_LIMIT {
                self.mode = Mode::Dead { left: DEATH_STEPS };
                return Some(PlayerEvent::FellToDeath);
            }
            self.mode = Mode::Walking;
            return Some(PlayerEvent::Landed);
        }
        self.x = new_x;
        self.y = new_y;
        self.mode = Mode::Airborne { vx, vy, top };
        // Fell out of the playfield: nothing to land on.
        if self.y > super::FIELD_H as f32 + 64.0 {
            self.mode = Mode::Dead { left: DEATH_STEPS };
            return Some(PlayerEvent::FellToDeath);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat() -> Geometry {
        Geometry {
            girders: vec![Girder::new(0.0, 100.0, 200.0, 100.0)],
            ladders: vec![],
        }
    }

    fn held(left: bool, right: bool, up: bool, down: bool) -> Input {
        Input {
            left,
            right,
            up,
            down,
            ..Input::default()
        }
    }

    fn jump_input(right: bool) -> Input {
        Input {
            right,
            jump_pressed: true,
            ..Input::default()
        }
    }

    #[test]
    fn follows_a_slope() {
        let geo = Geometry {
            girders: vec![Girder::new(0.0, 100.0, 200.0, 120.0)],
            ladders: vec![],
        };
        let mut p = Player::new((10.0, 101.0));
        for _ in 0..100 {
            p.step(&held(false, true, false, false), &geo);
        }
        assert_eq!(p.mode, Mode::Walking);
        let surface = geo.girders[0].y_at(p.x).unwrap();
        assert!((p.y - surface).abs() < 1e-3, "{} vs {}", p.y, surface);
        assert!(p.y > 108.0);
    }

    #[test]
    fn walks_off_the_edge_and_falls_to_the_girder_below() {
        let geo = Geometry {
            girders: vec![
                Girder::new(0.0, 100.0, 50.0, 100.0),
                Girder::new(0.0, 120.0, 200.0, 120.0),
            ],
            ladders: vec![],
        };
        let mut p = Player::new((45.0, 100.0));
        let e = loop {
            if let Some(e) = p.step(&held(false, true, false, false), &geo) {
                break e;
            }
        };
        assert_eq!(e, PlayerEvent::Landed);
        assert_eq!(p.mode, Mode::Walking);
        assert_eq!(p.y, 120.0);
    }

    fn ladder_geo() -> Geometry {
        Geometry {
            girders: vec![
                Girder::new(0.0, 100.0, 200.0, 100.0),
                Girder::new(0.0, 60.0, 200.0, 60.0),
            ],
            ladders: vec![Ladder {
                x: 100.0,
                top: 60.0,
                bottom: 100.0,
                broken: false,
            }],
        }
    }

    #[test]
    fn ladder_is_entered_climbed_and_left_at_the_top() {
        let geo = ladder_geo();
        let mut p = Player::new((98.0, 100.0));
        p.step(&held(false, false, true, false), &geo);
        assert_eq!(p.mode, Mode::Climbing { ladder: 0 });
        assert_eq!(p.x, 100.0);
        for _ in 0..200 {
            p.step(&held(false, false, true, false), &geo);
        }
        assert_eq!(p.mode, Mode::Walking);
        assert_eq!(p.y, 60.0);
    }

    #[test]
    fn ladder_is_entered_from_the_top_and_left_at_the_bottom() {
        let geo = ladder_geo();
        let mut p = Player::new((100.0, 60.0));
        p.step(&held(false, false, false, true), &geo);
        assert!(matches!(p.mode, Mode::Climbing { .. }));
        for _ in 0..200 {
            p.step(&held(false, false, false, true), &geo);
        }
        assert_eq!((p.mode, p.y), (Mode::Walking, 100.0));
    }

    #[test]
    fn broken_ladder_and_far_ladder_are_not_grabbed() {
        let mut geo = ladder_geo();
        let mut p = Player::new((100.0, 100.0));
        p.step(&held(false, false, true, false), &geo);
        assert!(matches!(p.mode, Mode::Climbing { .. }));
        geo.ladders[0].broken = true;
        let mut p = Player::new((100.0, 100.0));
        p.step(&held(false, false, true, false), &geo);
        assert_eq!(p.mode, Mode::Walking);
        geo.ladders[0].broken = false;
        let mut p = Player::new((120.0, 100.0));
        p.step(&held(false, false, true, false), &geo);
        assert_eq!(p.mode, Mode::Walking);
    }

    #[test]
    fn jump_has_a_fixed_arc_without_air_control() {
        let geo = flat();
        let mut p = Player::new((20.0, 100.0));
        assert_eq!(p.step(&jump_input(true), &geo), Some(PlayerEvent::Jumped));
        let mut steps = 1;
        let mut apex = p.y;
        // Steering left in the air changes nothing.
        let steer = held(true, false, false, false);
        let landed = loop {
            let e = p.step(&steer, &geo);
            steps += 1;
            apex = apex.min(p.y);
            if let Some(e) = e {
                break e;
            }
            assert!(steps < 200);
        };
        assert_eq!(landed, PlayerEvent::Landed);
        assert!(
            (100.0 - apex - 22.5).abs() < 2.0,
            "apex height {}",
            100.0 - apex
        );
        let dist = p.x - 20.0;
        assert!((dist - 35.0).abs() < 3.0, "jump length {dist}");
        assert_eq!(p.y, 100.0);
        // A standing jump goes straight up and down.
        let mut q = Player::new((50.0, 100.0));
        q.step(&jump_input(false), &geo);
        while q.step(&Input::default(), &geo).is_none() {}
        assert_eq!(q.x, 50.0);
    }

    #[test]
    fn a_jump_fits_over_a_gap_it_is_designed_for() {
        let geo = Geometry {
            girders: vec![
                Girder::new(0.0, 100.0, 60.0, 100.0),
                Girder::new(90.0, 100.0, 200.0, 100.0),
            ],
            ladders: vec![],
        };
        let mut p = Player::new((58.0, 100.0));
        p.step(&jump_input(true), &geo);
        for _ in 0..100 {
            p.step(&Input::default(), &geo);
        }
        assert_eq!((p.mode, p.y), (Mode::Walking, 100.0));
        assert!(p.x > 90.0);
    }

    #[test]
    fn small_drop_is_fine_big_drop_kills() {
        let drop = |h: f32| {
            let geo = Geometry {
                girders: vec![
                    Girder::new(0.0, 100.0, 50.0, 100.0),
                    Girder::new(0.0, 100.0 + h, 200.0, 100.0 + h),
                ],
                ladders: vec![],
            };
            let mut p = Player::new((49.0, 100.0));
            loop {
                if let Some(e) = p.step(&held(false, true, false, false), &geo) {
                    return e;
                }
            }
        };
        assert_eq!(drop(FALL_LIMIT - 4.0), PlayerEvent::Landed);
        assert_eq!(drop(FALL_LIMIT + 4.0), PlayerEvent::FellToDeath);
    }

    #[test]
    fn never_tunnels_through_a_thin_girder_at_top_speed() {
        let geo = Geometry {
            girders: vec![Girder::new(0.0, 200.0, 200.0, 200.0)],
            ladders: vec![],
        };
        let mut p = Player::new((100.0, 0.0));
        p.mode = Mode::Airborne {
            vx: 0.0,
            vy: MAX_FALL_SPEED,
            top: 0.0,
        };
        let e = loop {
            if let Some(e) = p.step(&Input::default(), &geo) {
                break e;
            }
        };
        assert_eq!(e, PlayerEvent::FellToDeath); // 200 px is far above the limit, but on the girder
        assert_eq!(p.y, 200.0);
    }

    #[test]
    fn hit_detection_is_forgiving_and_death_respawns() {
        let geo = flat();
        let mut p = Player::new((50.0, 100.0));
        // A barrel 4 px to the side (inside its sprite width, outside the narrow hitbox).
        let near = Rect {
            x0: 54.0,
            y0: 92.0,
            x1: 60.0,
            y1: 100.0,
        };
        assert!(!p.hit_by(near));
        let on_top = Rect {
            x0: 48.0,
            y0: 95.0,
            x1: 52.0,
            y1: 99.0,
        };
        assert!(p.hit_by(on_top));
        assert_eq!(p.kill(), Some(PlayerEvent::Killed));
        assert_eq!(p.kill(), None);
        assert!(!p.hit_by(on_top));
        p.x = 150.0;
        let mut ev = None;
        for _ in 0..DEATH_STEPS {
            ev = p.step(&Input::default(), &geo).or(ev);
        }
        assert_eq!(ev, Some(PlayerEvent::Respawned));
        assert_eq!((p.x, p.y, p.mode), (50.0, 100.0, Mode::Walking));
    }

    fn with_ladder() -> Geometry {
        Geometry {
            girders: vec![
                Girder::new(0.0, 100.0, 200.0, 100.0),
                Girder::new(0.0, 60.0, 200.0, 60.0),
            ],
            ladders: vec![Ladder {
                x: 50.0,
                top: 60.0,
                bottom: 100.0,
                broken: false,
            }],
        }
    }

    #[test]
    fn a_ladder_is_grabbed_in_the_air_with_up_held() {
        let geo = with_ladder();
        let mut p = Player::new((40.0, 100.0));
        p.step(&held(false, false, false, false), &geo);
        p.step(&jump_input(true), &geo);
        let mut grabbed = false;
        for _ in 0..40 {
            p.step(&held(false, false, true, false), &geo);
            if matches!(p.mode, Mode::Climbing { .. }) {
                grabbed = true;
                break;
            }
        }
        assert!(grabbed, "{:?}", p.mode);
        assert_eq!(p.x, 50.0);
    }

    #[test]
    fn the_player_can_jump_off_a_ladder() {
        let geo = with_ladder();
        let mut p = Player::new((50.0, 100.0));
        p.step(&held(false, false, true, false), &geo);
        for _ in 0..10 {
            p.step(&held(false, false, true, false), &geo);
        }
        assert!(matches!(p.mode, Mode::Climbing { .. }));
        let y = p.y;
        let ev = p.step(&jump_input(true), &geo);
        assert_eq!(ev, Some(PlayerEvent::Jumped));
        assert!(p.is_airborne());
        // Not grabbed again at once although up is not held; it lands on the girder below.
        for _ in 0..60 {
            p.step(&Input::default(), &geo);
        }
        assert!(p.y >= y - 1.0 && p.is_walking(), "{:?} {}", p.mode, p.y);
    }
}
