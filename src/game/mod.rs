//! The platform-independent game: no egui, no I/O, no wall-clock time. The front end (`app`)
//! feeds it an `Input` once per fixed step (`STEP_HZ`) and plays the returned `Event`s.
//! With a seed and an input sequence the result is always the same (deterministic, testable).
//!
//! The levels are text files (`levels/*.txt`, parsed by `level.rs`) embedded by `levels.rs`.

mod barrel;
mod difficulty;
mod fireball;
mod input;
mod level;
// No front end generates levels yet (the editor, #25, and the tests do).
#[allow(dead_code)]
mod levelgen;
mod levels;
mod player;
mod rng;
mod timestep;
mod validate;

pub use barrel::{Barrel, BarrelMode, SPRING_VX};
pub use difficulty::{Difficulty, Params};
pub use fireball::Fireball;
pub use input::Input;
pub use level::{LIFT_WIDTH, Level, LevelKind};
pub use levels::{builtin as builtin_level, count as level_count};
pub use player::{Geometry, Girder, Ladder, Mode, Player, PlayerEvent};
pub use rng::Rng;
pub use timestep::{FixedStep, STEP_HZ};

/// Logical playfield size in pixels; the front end scales it to the screen.
pub const FIELD_W: i32 = 224;
pub const FIELD_H: i32 = 256;

/// Axis-aligned rectangle in playfield pixels (`x0 < x1`, `y0 < y1`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl Rect {
    pub fn overlaps(&self, o: Rect) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
}

/// Something that happened during a step; the front end turns it into sound and effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Jump,
    Land,
    Died,
    Respawn,
    HammerPickup,
    /// A barrel was smashed with the hammer.
    Smash,
    /// The player jumped over a barrel.
    JumpedBarrel,
    ExtraLife,
    /// A rivet was removed.
    Rivet,
    LevelComplete,
    GameOver,
}

/// Where the game stands; `step` does nothing unless it is `Playing`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Playing,
    /// The goal was reached; the time bonus is already added to the score.
    Complete,
    /// All lives are lost.
    GameOver,
}

/// Lives of a new game on `Difficulty::Normal`.
#[cfg(test)]
pub const START_LIVES: u32 = 3;
/// Steps the hammer stays in the hand.
pub const HAMMER_STEPS: u32 = 10 * STEP_HZ;
/// Steps before the ape throws the first barrel (and after a death).
const FIRST_BARREL_STEPS: u32 = STEP_HZ;
pub const SCORE_JUMP: u32 = 100;
pub const SCORE_SMASH: u32 = 300;
pub const SCORE_RIVET: u32 = 100;
/// Score of a fireball smashed with the hammer, and how many fireballs the ape keeps alive.
pub const SCORE_FIRE: u32 = 500;
const MAX_FIREBALLS: usize = 4;
/// How close (horizontally) the hero must walk over a rivet to remove it.
const RIVET_REACH: f32 = 4.0;
/// Half size of a hammer item and the reach of the swung hammer in front of the player.
const HAMMER_ITEM_R: f32 = 5.0;
const HAMMER_REACH: f32 = 12.0;
/// How far above a barrel the feet must pass (and how close horizontally) to count as a jump over.
const JUMP_OVER_HEIGHT: f32 = 14.0;
const JUMP_OVER_HALF_W: f32 = 6.0;
/// Steps the ape shows the throwing pose after a barrel left.
const APE_THROW_STEPS: u32 = 15;
const GOAL_HALF_W: f32 = 8.0;
const GOAL_HEIGHT: f32 = 16.0;
/// Pixels per step a conveyor belt carries what stands on it (at obstacle speed 1.0).
pub const BELT_SPEED: f32 = 0.5;
/// Pixels per step a lift moves (at obstacle speed 1.0).
pub const LIFT_SPEED: f32 = 0.4;

/// The running state of one `Belt` of the level (same index).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BeltState {
    pub dir: i8,
    /// Steps until the belt reverses (counts down only when the belt reverses at all).
    pub left: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    /// Simulation steps done since the start.
    pub tick: u64,
    pub level: Level,
    pub player: Player,
    pub barrels: Vec<Barrel>,
    pub fireballs: Vec<Fireball>,
    /// Rivets still in place.
    pub rivets: Vec<(f32, f32)>,
    /// Hammer items still lying around.
    pub hammers: Vec<(f32, f32)>,
    /// Steps left of the hammer in the player's hand (0 = none).
    pub hammer_left: u32,
    /// The conveyor belts: the direction each runs now and the steps until it reverses.
    pub belts: Vec<BeltState>,
    /// Current surface height of every lift of the level (same index). Each lift is also a short
    /// girder at the end of `level.geometry.girders`, so the player and the barrels treat it
    /// as ground; `scenery()` is the geometry without them.
    pub lifts: Vec<f32>,
    pub score: u32,
    pub lives: u32,
    pub bonus: u32,
    pub status: Status,
    pub difficulty: Difficulty,
    params: Params,
    extra_life_given: bool,
    barrel_in: u32,
    bonus_in: u32,
    pub rng: Rng,
}

impl GameState {
    /// A new game of `level` on `Difficulty::Normal`, with the full lives and no score.
    #[cfg(test)]
    pub fn new(level: Level, seed: u64) -> Self {
        Self::start(level, seed, Difficulty::Normal, None, 0)
    }

    /// The start of `level` on `difficulty`. `lives` is `None` for the full lives of the
    /// difficulty, `score` is what was carried over from the previous level.
    pub fn start(
        mut level: Level,
        seed: u64,
        difficulty: Difficulty,
        lives: Option<u32>,
        score: u32,
    ) -> Self {
        let params = difficulty.params();
        // The ape's pace, the hammers and every score of the level follow the difficulty.
        level.barrel_every = level.barrel_every * params.spawn_interval_pct / 100;
        level.bonus = params.scaled(level.bonus);
        level.bonus_drop = params.scaled(level.bonus_drop);
        level.extra_life = params.scaled(level.extra_life);
        if params.hammer_delta >= 0 {
            let extra = (params.hammer_delta as usize).min(level.bonus_hammers.len());
            let bonus = level.bonus_hammers[..extra].to_vec();
            level.hammers.extend(bonus);
        } else {
            // Take hammers away, but never the last one.
            let keep = level
                .hammers
                .len()
                .saturating_sub(params.hammer_delta.unsigned_abs() as usize)
                .max(1.min(level.hammers.len()));
            level.hammers.truncate(keep);
        }
        // The lifts become girders at the end of the list (moved every step).
        let lifts: Vec<f32> = level
            .lifts
            .iter()
            .map(|l| if l.dir < 0 { l.bottom } else { l.top })
            .collect();
        for (l, &y) in level.lifts.iter().zip(&lifts) {
            let half = LIFT_WIDTH / 2.0;
            level
                .geometry
                .girders
                .push(Girder::new(l.x - half, y, l.x + half, y));
        }
        let every = level.extra_life;
        Self {
            tick: 0,
            player: Player::new(level.start),
            barrels: Vec::new(),
            fireballs: Vec::new(),
            rivets: level.rivets.clone(),
            hammers: level.hammers.clone(),
            hammer_left: 0,
            lifts,
            belts: level
                .belts
                .iter()
                .map(|b| BeltState {
                    dir: b.dir,
                    left: b.every,
                })
                .collect(),
            score,
            lives: lives.unwrap_or(params.lives),
            bonus: level.bonus,
            status: Status::Playing,
            difficulty,
            params,
            // A carried-over score that already earned the extra life must not earn it twice.
            extra_life_given: every > 0 && score >= every,
            barrel_in: FIRST_BARREL_STEPS,
            bonus_in: level.bonus_every,
            rng: Rng::new(seed),
            level,
        }
    }

    /// The geometry that never moves (without the lift platforms): what the background shows.
    pub fn scenery(&self) -> Geometry {
        let mut g = self.level.geometry.clone();
        g.girders.truncate(g.girders.len() - self.lifts.len());
        g
    }

    /// The direction of the belt under (`x`, `y`), `None` when the ground there is no belt.
    pub fn belt_dir(&self, x: f32, y: f32) -> Option<i8> {
        let g = &self.level.geometry;
        let i = g
            .girders
            .iter()
            .position(|gi| gi.y_at(x).is_some_and(|s| (s - y).abs() <= player::SNAP))?;
        let at = self.level.belts.iter().position(|b| b.girder == i)?;
        Some(self.belts[at].dir)
    }

    pub fn goal_rect(&self) -> Rect {
        let (x, y) = self.level.goal;
        Rect {
            x0: x - GOAL_HALF_W,
            y0: y - GOAL_HEIGHT,
            x1: x + GOAL_HALF_W,
            y1: y,
        }
    }

    /// The ape has just thrown a barrel (for its animation).
    pub fn ape_throwing(&self) -> bool {
        let every = self.level.barrel_every;
        self.barrel_in <= every && self.barrel_in + APE_THROW_STEPS > every
    }

    /// The area the swung hammer covers: the player's box stretched in front of it.
    pub fn hammer_rect(&self) -> Rect {
        let h = self.player.hitbox();
        let (x0, x1) = if self.player.facing < 0 {
            (h.x0 - HAMMER_REACH, h.x1)
        } else {
            (h.x0, h.x1 + HAMMER_REACH)
        };
        Rect {
            x0,
            y0: h.y0 - 4.0,
            x1,
            y1: h.y1 + 2.0,
        }
    }

    fn add_score(&mut self, points: u32, events: &mut Vec<Event>) {
        self.score += self.params.scaled(points);
        let every = self.level.extra_life;
        if every > 0 && !self.extra_life_given && self.score >= every {
            self.extra_life_given = true;
            self.lives += 1;
            events.push(Event::ExtraLife);
        }
    }
}

/// Runs one fixed step and returns the events it produced.
pub fn step(state: &mut GameState, input: &Input) -> Vec<Event> {
    let mut events = Vec::new();
    if state.status != Status::Playing {
        return events;
    }
    state.tick += 1;

    // The lifts ride and start again at the other end.
    let speed = LIFT_SPEED * state.params.obstacle_speed;
    let fixed = state.level.geometry.girders.len() - state.lifts.len();
    for (i, spec) in state.level.lifts.iter().enumerate() {
        let mut y = state.lifts[i] + f32::from(spec.dir) * speed;
        if y < spec.top {
            y = spec.bottom;
        } else if y > spec.bottom {
            y = spec.top;
        }
        state.lifts[i] = y;
        let g = &mut state.level.geometry.girders[fixed + i];
        g.y0 = y;
        g.y1 = y;
    }

    // With the hammer in the hand the player can neither climb nor jump (as in the original).
    let mut input = *input;
    if state.hammer_left > 0 {
        input.up = false;
        input.down = false;
        input.jump_pressed = false;
    }
    match state.player.step(&input, &state.level.geometry) {
        Some(PlayerEvent::Jumped) => events.push(Event::Jump),
        Some(PlayerEvent::Landed) => events.push(Event::Land),
        Some(PlayerEvent::FellToDeath | PlayerEvent::Killed) => die(state, &mut events),
        Some(PlayerEvent::Respawned) => {
            events.push(Event::Respawn);
            if state.lives == 0 {
                state.status = Status::GameOver;
                events.push(Event::GameOver);
                return events;
            }
            state.barrels.clear();
            state.fireballs.clear();
            state.barrel_in = FIRST_BARREL_STEPS;
        }
        None => {}
    }

    // The belts reverse on schedule and carry the player standing on them.
    for (b, spec) in state.belts.iter_mut().zip(&state.level.belts) {
        if spec.every > 0 {
            b.left = b.left.saturating_sub(1);
            if b.left == 0 {
                b.dir = -b.dir;
                b.left = spec.every;
            }
        }
    }
    if state.player.is_walking()
        && let Some(dir) = state.belt_dir(state.player.x, state.player.y)
    {
        let push = f32::from(dir) * BELT_SPEED * state.params.obstacle_speed;
        state.player.carry(push, &state.level.geometry);
    }

    // The ape throws barrels, the barrels move.
    state.barrel_in = state.barrel_in.saturating_sub(1);
    if state.barrel_in == 0 {
        let (ax, ay) = state.level.ape;
        if state.level.kind == LevelKind::Rivets {
            if state.fireballs.len() < MAX_FIREBALLS {
                state.fireballs.push(Fireball::new(ax + 16.0, ay, 1));
            }
        } else {
            state.barrels.push(if state.level.kind == LevelKind::Lifts {
                Barrel::spring(ax + 16.0, ay - 8.0, SPRING_VX * state.params.obstacle_speed)
            } else {
                Barrel::thrown(ax + 16.0, ay - 8.0, 1)
            });
        }
        state.barrel_in = state.level.barrel_every;
    }
    for i in 0..state.barrels.len() {
        // A barrel rolling on a belt goes where the belt goes.
        let (x, y) = (state.barrels[i].x, state.barrels[i].y);
        let belt = state.belt_dir(x, y);
        let b = &mut state.barrels[i];
        if let Some(dir) = belt {
            b.ride(dir);
        }
        b.step(&state.level.geometry, &mut state.rng, &state.params);
    }
    state.barrels.retain(|b| !b.is_gone());

    // The fireballs hunt the player.
    let target = (state.player.x, state.player.y);
    for f in &mut state.fireballs {
        f.step(
            &state.level.geometry,
            &mut state.rng,
            &state.params,
            target,
            state.tick,
        );
    }
    state.fireballs.retain(|f| !f.is_gone());

    if !state.player.is_dead() {
        player_meets_world(state, &mut events);
    }
    if state.status != Status::Playing {
        return events;
    }

    state.bonus_in -= 1;
    if state.bonus_in == 0 {
        state.bonus = state.bonus.saturating_sub(state.level.bonus_drop);
        state.bonus_in = state.level.bonus_every;
    }
    events
}

fn die(state: &mut GameState, events: &mut Vec<Event>) {
    state.lives = state.lives.saturating_sub(1);
    state.hammer_left = 0;
    events.push(Event::Died);
}

/// Hammer, barrels, goal: everything the living player touches.
fn player_meets_world(state: &mut GameState, events: &mut Vec<Event>) {
    let hit = state.player.hitbox();

    // Picking up a hammer (only while walking, and only when the hand is empty).
    if state.hammer_left == 0
        && state.player.is_walking()
        && let Some(i) = state.hammers.iter().position(|&(x, y)| {
            hit.overlaps(Rect {
                x0: x - HAMMER_ITEM_R,
                y0: y - 2.0 * HAMMER_ITEM_R,
                x1: x + HAMMER_ITEM_R,
                y1: y,
            })
        })
    {
        state.hammers.remove(i);
        state.hammer_left = HAMMER_STEPS;
        events.push(Event::HammerPickup);
    }
    state.hammer_left = state.hammer_left.saturating_sub(1);

    // Smashing barrels with the hammer.
    if state.hammer_left > 0 {
        let zone = state.hammer_rect();
        let before = state.barrels.len();
        state.barrels.retain(|b| !b.rect().overlaps(zone));
        for _ in state.barrels.len()..before {
            state.add_score(SCORE_SMASH, events);
            events.push(Event::Smash);
        }
    }

    // The hammer burns fireballs too.
    if state.hammer_left > 0 {
        let zone = state.hammer_rect();
        let before = state.fireballs.len();
        state.fireballs.retain(|f| !f.rect().overlaps(zone));
        for _ in state.fireballs.len()..before {
            state.add_score(SCORE_FIRE, events);
            events.push(Event::Smash);
        }
    }

    // Walking over a rivet removes it; the last one completes the rivets level.
    if state.player.is_walking() {
        let (px, py) = (state.player.x, state.player.y);
        let before = state.rivets.len();
        state
            .rivets
            .retain(|&(x, y)| (x - px).abs() > RIVET_REACH || (y - py).abs() > RIVET_REACH);
        for _ in state.rivets.len()..before {
            state.add_score(SCORE_RIVET, events);
            events.push(Event::Rivet);
        }
        if state.level.kind == LevelKind::Rivets && state.rivets.is_empty() {
            state.score += state.bonus;
            state.status = Status::Complete;
            events.push(Event::LevelComplete);
            return;
        }
    }

    // Jumping over a barrel: the feet pass just above it.
    if state.player.is_airborne() {
        let (px, py) = (state.player.x, state.player.y);
        let mut awarded = 0;
        for b in &mut state.barrels {
            let top = b.y - 2.0 * barrel::RADIUS;
            if !b.jumped
                && (b.x - px).abs() <= JUMP_OVER_HALF_W
                && py <= top
                && top - py <= JUMP_OVER_HEIGHT
            {
                b.jumped = true;
                awarded += 1;
            }
        }
        for _ in 0..awarded {
            state.add_score(SCORE_JUMP, events);
            events.push(Event::JumpedBarrel);
        }
    }

    // A barrel or a fireball that touches the player kills.
    if (state.barrels.iter().any(|b| state.player.hit_by(b.rect()))
        || state
            .fireballs
            .iter()
            .any(|f| state.player.hit_by(f.rect())))
        && state.player.kill().is_some()
    {
        die(state, events);
        return;
    }

    // The goal (on the rivets level the rivets decide).
    if state.level.kind != LevelKind::Rivets && hit.overlaps(state.goal_rect()) {
        state.score += state.bonus;
        state.status = Status::Complete;
        events.push(Event::LevelComplete);
    }
}

#[cfg(test)]
mod tests {
    use super::barrel::BarrelMode;
    use super::player::Mode;
    use super::*;

    fn level() -> Level {
        builtin_level(0)
    }

    fn state(seed: u64) -> GameState {
        GameState::new(level(), seed)
    }

    fn run(seed: u64, inputs: &[Input]) -> (GameState, Vec<Event>) {
        let mut s = state(seed);
        let mut ev = Vec::new();
        for i in inputs {
            ev.extend(step(&mut s, i));
        }
        (s, ev)
    }

    fn script() -> Vec<Input> {
        (0..300)
            .map(|n| Input {
                left: n % 7 < 3,
                right: n % 5 < 2,
                up: n % 11 == 0,
                down: n % 13 < 4,
                jump_held: false,
                auto_grab: false,
                jump_pressed: n % 50 == 0,
            })
            .collect()
    }

    fn idle(s: &mut GameState, steps: usize) -> Vec<Event> {
        (0..steps)
            .flat_map(|_| step(s, &Input::default()))
            .collect()
    }

    #[test]
    fn same_seed_and_inputs_give_the_same_state() {
        assert_eq!(run(5, &script()), run(5, &script()));
        // Different seeds differ once barrels pick ladders.
        let long: Vec<Input> = vec![Input::default(); 1500];
        assert_ne!(run(5, &long).0.barrels, run(6, &long).0.barrels);
    }

    #[test]
    fn player_stays_on_the_floor_when_idle() {
        let mut s = state(1);
        s.barrel_in = u32::MAX; // no barrels in this test
        idle(&mut s, 100);
        assert!((s.player.y - s.level.start.1).abs() < 1.0, "{}", s.player.y);
        assert_eq!(s.player.mode, Mode::Walking);
    }

    #[test]
    fn jump_edge_gives_one_event() {
        let jump = Input {
            jump_pressed: true,
            ..Input::default()
        };
        let (_, ev) = run(1, &[jump, Input::default(), Input::default()]);
        assert_eq!(ev, vec![Event::Jump]);
    }

    #[test]
    fn the_ape_throws_barrels_on_schedule() {
        let mut s = state(1);
        s.player.kill(); // stay out of the way
        s.player.mode = Mode::Dead { left: u32::MAX };
        idle(&mut s, FIRST_BARREL_STEPS as usize);
        assert_eq!(s.barrels.len(), 1);
        let every = s.level.barrel_every as usize;
        idle(&mut s, every);
        assert_eq!(s.barrels.len(), 2);
    }

    #[test]
    fn the_ape_shows_the_throw_only_right_after_a_barrel() {
        let mut s = state(1);
        s.player.mode = Mode::Dead { left: u32::MAX };
        idle(&mut s, FIRST_BARREL_STEPS as usize - 1);
        assert!(!s.ape_throwing());
        idle(&mut s, 1);
        assert!(s.ape_throwing());
        idle(&mut s, APE_THROW_STEPS as usize);
        assert!(!s.ape_throwing());
    }

    /// A state with the player standing on the floor and a barrel placed right at it.
    fn state_with_barrel_at_player() -> GameState {
        let mut s = state(1);
        s.barrel_in = u32::MAX;
        idle(&mut s, 2);
        let (x, y) = (s.player.x, s.player.y);
        s.barrels.push(Barrel::thrown(x, y, 1));
        s.barrels[0].mode = BarrelMode::Rolling { dir: 1 };
        s
    }

    #[test]
    fn a_barrel_kills_costs_a_life_and_respawn_clears_barrels() {
        let mut s = state_with_barrel_at_player();
        let ev = step(&mut s, &Input::default());
        assert_eq!(ev, vec![Event::Died]);
        assert_eq!(s.lives, START_LIVES - 1);
        assert!(s.player.is_dead());
        let ev = idle(&mut s, 100);
        assert!(ev.contains(&Event::Respawn));
        assert!(s.barrels.is_empty());
        assert_eq!(s.status, Status::Playing);
    }

    #[test]
    fn losing_the_last_life_ends_the_game_after_the_death_animation() {
        let mut s = state_with_barrel_at_player();
        s.lives = 1;
        let ev = step(&mut s, &Input::default());
        assert_eq!(
            (ev, s.lives, s.status),
            (vec![Event::Died], 0, Status::Playing)
        );
        let ev = idle(&mut s, 100);
        assert!(ev.contains(&Event::GameOver));
        assert_eq!(s.status, Status::GameOver);
        // Nothing moves any more.
        let tick = s.tick;
        assert!(idle(&mut s, 10).is_empty());
        assert_eq!(s.tick, tick);
    }

    #[test]
    fn the_hammer_smashes_barrels_and_scores() {
        let mut s = state_with_barrel_at_player();
        s.hammer_left = 100;
        let ev = step(&mut s, &Input::default());
        assert_eq!(ev, vec![Event::Smash]);
        assert_eq!((s.score, s.lives), (SCORE_SMASH, START_LIVES));
        assert!(s.barrels.is_empty());
    }

    #[test]
    fn the_hammer_is_picked_up_runs_out_and_blocks_jumping() {
        let mut s = state(1);
        s.barrel_in = u32::MAX;
        let (hx, hy) = (s.player.x, s.player.y);
        s.hammers.push((hx, hy));
        let ev = step(&mut s, &Input::default());
        assert_eq!(ev, vec![Event::HammerPickup]);
        assert!(s.hammers.len() == s.level.hammers.len());
        assert_eq!(s.hammer_left, HAMMER_STEPS - 1);
        let jump = Input {
            jump_pressed: true,
            ..Input::default()
        };
        assert!(step(&mut s, &jump).is_empty()); // no jumping with the hammer
        assert_eq!(s.player.mode, Mode::Walking);
        idle(&mut s, HAMMER_STEPS as usize);
        assert_eq!(s.hammer_left, 0);
        assert_eq!(step(&mut s, &jump), vec![Event::Jump]);
    }

    #[test]
    fn jumping_over_a_barrel_scores_once_and_does_not_kill() {
        let mut s = state(1);
        s.barrel_in = u32::MAX;
        idle(&mut s, 2);
        // A barrel rolls towards the player from 14 px away; the player jumps straight up.
        let (x, y) = (s.player.x + 14.0, s.player.y);
        s.barrels.push(Barrel::thrown(x, y, -1));
        s.barrels[0].mode = BarrelMode::Rolling { dir: -1 };
        let jump = Input {
            jump_pressed: true,
            ..Input::default()
        };
        let mut ev = step(&mut s, &jump);
        for _ in 0..60 {
            ev.extend(step(&mut s, &Input::default()));
        }
        let jumps = ev.iter().filter(|e| **e == Event::JumpedBarrel).count();
        assert_eq!(jumps, 1, "{ev:?}");
        assert_eq!((s.score, s.lives), (SCORE_JUMP, START_LIVES));
    }

    #[test]
    fn the_bonus_counts_down_and_is_added_at_the_goal() {
        let mut s = state(1);
        s.barrel_in = u32::MAX;
        let every = s.level.bonus_every as usize;
        idle(&mut s, every * 3);
        assert_eq!(s.bonus, s.level.bonus - 3 * s.level.bonus_drop);
        // Teleport the player to the goal.
        let (gx, gy) = s.level.goal;
        s.player.x = gx;
        s.player.y = gy;
        let bonus = s.bonus;
        let ev = step(&mut s, &Input::default());
        assert_eq!(ev, vec![Event::LevelComplete]);
        assert_eq!((s.status, s.score), (Status::Complete, bonus));
        assert!(idle(&mut s, 5).is_empty());
    }

    #[test]
    fn the_bonus_never_goes_below_zero() {
        let mut s = state(1);
        s.barrel_in = u32::MAX;
        s.bonus = 50;
        let every = s.level.bonus_every as usize;
        idle(&mut s, every * 2);
        assert_eq!(s.bonus, 0);
    }

    #[test]
    fn an_extra_life_is_given_once_at_the_score() {
        let mut s = state_with_barrel_at_player();
        s.hammer_left = 100;
        s.score = s.level.extra_life - SCORE_SMASH;
        let ev = step(&mut s, &Input::default());
        assert!(ev.contains(&Event::ExtraLife), "{ev:?}");
        assert_eq!(s.lives, START_LIVES + 1);
        let mut events = Vec::new();
        s.add_score(100_000, &mut events);
        assert!(events.is_empty());
        assert_eq!(s.lives, START_LIVES + 1);
    }

    #[test]
    fn barrels_travel_down_the_whole_level_and_leave_it() {
        let mut s = state(7);
        s.player.mode = Mode::Dead { left: u32::MAX }; // out of the way
        let mut lowest = 0.0f32;
        let mut max_alive = 0;
        for _ in 0..60 * 120 {
            step(&mut s, &Input::default());
            max_alive = max_alive.max(s.barrels.len());
            for b in &s.barrels {
                lowest = lowest.max(b.y);
            }
        }
        // They reach the floor (y ~ 240-248) and fall out at its low end; never pile up.
        assert!(lowest > 235.0, "lowest {lowest}");
        assert!((3..=14).contains(&max_alive), "max alive {max_alive}");
    }

    #[test]
    fn the_difficulty_sets_lives_pace_hammers_and_scores() {
        let start = |d| GameState::start(level(), 1, d, None, 0);
        let normal = start(Difficulty::Normal);
        assert_eq!(normal.lives, START_LIVES);
        assert_eq!(normal.level.barrel_every, level().barrel_every);
        assert_eq!(normal.hammers.len(), 2);
        assert_eq!(normal.bonus, level().bonus);

        let relaxed = start(Difficulty::Relaxed);
        assert_eq!(relaxed.lives, 7);
        assert!(relaxed.level.barrel_every > normal.level.barrel_every);
        assert_eq!(relaxed.hammers.len(), 4); // two of the level's own, two bonus ones
        assert_eq!(relaxed.bonus, level().bonus / 2);

        assert_eq!(start(Difficulty::Easy).hammers.len(), 3);
        assert_eq!(start(Difficulty::Hard).hammers.len(), 2);
        let insane = start(Difficulty::Insane);
        assert_eq!((insane.lives, insane.hammers.len()), (2, 1));
        assert!(insane.level.barrel_every < normal.level.barrel_every);
        assert_eq!(insane.bonus, level().bonus * 2);
    }

    #[test]
    fn points_follow_the_score_multiplier() {
        let mut s = GameState::start(level(), 1, Difficulty::Easy, None, 0);
        let mut ev = Vec::new();
        s.add_score(SCORE_SMASH, &mut ev);
        assert_eq!(s.score, 225); // 300 * 0.75
    }

    #[test]
    fn a_carried_over_score_keeps_lives_and_does_not_repeat_the_extra_life() {
        let every = level().extra_life;
        let mut s = GameState::start(level(), 1, Difficulty::Normal, Some(2), every + 10);
        assert_eq!((s.lives, s.score), (2, every + 10));
        let mut ev = Vec::new();
        s.add_score(100, &mut ev);
        assert_eq!(s.lives, 2);
        assert!(ev.is_empty());
    }

    /// The same seed gives a different game on another difficulty, and the same one twice.
    #[test]
    fn difficulty_is_deterministic() {
        let go = |d| {
            let mut s = GameState::start(level(), 5, d, None, 0);
            for _ in 0..600 {
                step(&mut s, &Input::default());
            }
            s.barrels.iter().map(|b| (b.x, b.y)).collect::<Vec<_>>()
        };
        assert_eq!(go(Difficulty::Hard), go(Difficulty::Hard));
        assert_ne!(go(Difficulty::Hard), go(Difficulty::Relaxed));
    }

    fn conveyors() -> GameState {
        GameState::new(builtin_level(1), 1)
    }

    #[test]
    fn a_standing_hero_is_carried_by_the_belt_and_falls_off_its_end() {
        let mut s = conveyors();
        s.barrel_in = u32::MAX;
        s.player.x = 100.0;
        s.player.y = 208.0; // belt 0 runs right
        assert_eq!(s.belt_dir(100.0, 208.0), Some(1));
        idle(&mut s, 60);
        assert!(s.player.x > 100.0 + 25.0, "{}", s.player.x);
        assert!(s.player.is_walking());
        // The floor without a belt does not move the hero.
        let mut s = conveyors();
        s.barrel_in = u32::MAX;
        idle(&mut s, 60);
        assert_eq!(s.player.x, 20.0);
        // Left alone on the belt the hero reaches the open end; the fall to the floor below
        // (40 px) is deadly.
        let mut s = conveyors();
        s.barrel_in = u32::MAX;
        s.player.x = 100.0;
        s.player.y = 208.0;
        let events = idle(&mut s, 400);
        assert!(events.contains(&Event::Died));
    }

    #[test]
    fn walking_against_a_belt_is_slower() {
        let mut s = conveyors();
        s.barrel_in = u32::MAX;
        s.player.x = 100.0;
        s.player.y = 208.0;
        let left = Input {
            left: true,
            ..Input::default()
        };
        for _ in 0..40 {
            step(&mut s, &left);
        }
        // 1.0 px/step against 0.5 px/step: 20 px in 40 steps.
        assert!((s.player.x - 80.0).abs() < 1.0, "{}", s.player.x);
    }

    #[test]
    fn belts_reverse_on_schedule() {
        let mut s = conveyors();
        s.barrel_in = u32::MAX;
        s.player.mode = Mode::Dead { left: u32::MAX };
        let every = s.level.belts[0].every as usize;
        assert_eq!(s.belts[0].dir, 1);
        idle(&mut s, every - 1);
        assert_eq!(s.belts[0].dir, 1);
        idle(&mut s, 1);
        assert_eq!(s.belts[0].dir, -1);
        idle(&mut s, every);
        assert_eq!(s.belts[0].dir, 1);
        assert_eq!(s.belt_dir(100.0, 248.0), None);
    }

    #[test]
    fn a_load_on_a_belt_goes_where_the_belt_goes() {
        let mut s = conveyors();
        s.barrel_in = u32::MAX;
        s.player.mode = Mode::Dead { left: u32::MAX };
        s.barrels.push(Barrel::thrown(100.0, 200.0, -1)); // thrown left, lands on belt 0 (right)
        idle(&mut s, 60);
        let b = &s.barrels[0];
        assert!(b.x > 100.0, "{}", b.x);
        assert!(matches!(b.mode, BarrelMode::Rolling { dir: 1 }));
    }

    fn lifts() -> GameState {
        let mut s = GameState::new(builtin_level(2), 1);
        s.barrel_in = u32::MAX;
        s
    }

    #[test]
    fn lifts_are_ground_at_the_end_of_the_girder_list_but_not_scenery() {
        let s = lifts();
        assert_eq!(s.lifts.len(), 2);
        assert_eq!(s.level.geometry.girders.len(), 4 + 2);
        assert_eq!(s.scenery().girders.len(), 4);
        // The rising lift starts at the bottom, the falling one at the top.
        assert_eq!(s.lifts, vec![248.0, 100.0]);
    }

    #[test]
    fn lifts_ride_and_start_again_at_the_other_end() {
        let mut s = lifts();
        s.player.mode = Mode::Dead { left: u32::MAX };
        idle(&mut s, 100);
        assert!((s.lifts[0] - (248.0 - 100.0 * LIFT_SPEED)).abs() < 1e-3);
        assert!((s.lifts[1] - (100.0 + 100.0 * LIFT_SPEED)).abs() < 1e-3);
        idle(&mut s, 300); // 148 px at 0.4 px per step = 370 steps: the rising lift wrapped
        assert!(s.lifts[0] > 200.0, "{}", s.lifts[0]);
        let fixed = s.level.geometry.girders.len() - 2;
        assert_eq!(s.level.geometry.girders[fixed].y0, s.lifts[0]);
    }

    #[test]
    fn the_hero_rides_a_lift_up() {
        let mut s = lifts();
        s.player.x = 76.0;
        s.player.y = 248.0;
        idle(&mut s, 120);
        assert!(s.player.is_walking());
        assert!((s.player.y - s.lifts[0]).abs() < 1e-3);
        assert!(s.player.y < 248.0 - 40.0, "{}", s.player.y);
    }

    #[test]
    fn the_ape_drops_springs_on_the_lifts_level() {
        let mut s = GameState::new(builtin_level(2), 1);
        s.player.mode = Mode::Dead { left: u32::MAX };
        idle(&mut s, FIRST_BARREL_STEPS as usize);
        assert!(matches!(s.barrels[0].mode, BarrelMode::Bouncing { .. }));
        idle(&mut s, 200);
        // It has bounced to the right along the ape's floor.
        assert!(s.barrels[0].x > 36.0);
    }

    #[test]
    fn a_hero_standing_on_nothing_next_to_the_lift_falls() {
        // Left of the start platform's end, between the platform and the lift.
        let mut s = lifts();
        s.player.x = 60.0;
        s.player.y = 248.0;
        let events = idle(&mut s, 200);
        assert!(events.contains(&Event::Died));
    }

    fn rivets() -> GameState {
        let mut s = GameState::new(builtin_level(3), 1);
        s.barrel_in = u32::MAX;
        s
    }

    #[test]
    fn walking_over_a_rivet_removes_it_and_scores() {
        let mut s = rivets();
        assert_eq!(s.rivets.len(), 8);
        s.player.x = 24.0;
        s.player.y = 200.0;
        let ev = idle(&mut s, 2);
        assert_eq!(s.rivets.len(), 7);
        assert!(ev.contains(&Event::Rivet));
        assert_eq!(s.score, 100);
        // Not walking: a jump over a rivet leaves it.
        s.player.x = 200.0;
        s.player.y = 140.0;
        s.player.mode = Mode::Airborne {
            vx: 0.0,
            vy: 0.0,
            top: 100.0,
        };
        idle(&mut s, 1);
        assert_eq!(s.rivets.len(), 7);
    }

    #[test]
    fn the_last_rivet_completes_the_level_and_the_goal_does_not() {
        let mut s = rivets();
        let last = s.rivets.pop().unwrap();
        s.rivets.truncate(1);
        s.rivets = vec![last];
        s.player.x = last.0;
        s.player.y = last.1;
        let ev = idle(&mut s, 1);
        assert_eq!(s.status, Status::Complete);
        assert!(ev.contains(&Event::LevelComplete));
        // Standing at the friend with rivets left does nothing.
        let mut s = rivets();
        s.player.x = s.level.goal.0;
        s.player.y = s.level.goal.1;
        idle(&mut s, 5);
        assert_eq!(s.status, Status::Playing);
    }

    #[test]
    fn a_fireball_kills_and_the_hammer_burns_it() {
        let mut s = rivets();
        s.fireballs.push(Fireball::new(20.0, 248.0, 1));
        let ev = idle(&mut s, 1);
        assert!(ev.contains(&Event::Died));
        let mut s = rivets();
        s.hammer_left = HAMMER_STEPS;
        s.fireballs.push(Fireball::new(26.0, 248.0, 1));
        let ev = idle(&mut s, 1);
        assert!(ev.contains(&Event::Smash));
        assert!(s.fireballs.is_empty());
        assert_eq!(s.score, 500);
    }

    #[test]
    fn the_ape_sends_at_most_four_fireballs() {
        let mut s = GameState::new(builtin_level(3), 1);
        s.player.mode = Mode::Dead { left: u32::MAX };
        idle(&mut s, 3000);
        assert!(s.barrels.is_empty());
        assert!(!s.fireballs.is_empty() && s.fireballs.len() <= 4);
    }
}
