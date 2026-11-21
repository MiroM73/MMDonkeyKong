//! The platform-independent game: no egui, no I/O, no wall-clock time. The front end (`app`)
//! feeds it an `Input` once per fixed step (`STEP_HZ`) and plays the returned `Event`s.
//! With a seed and an input sequence the result is always the same (deterministic, testable).
//!
//! The levels are text files (`levels/*.txt`, parsed by `level.rs`) embedded by `levels.rs`.

mod barrel;
mod input;
mod level;
mod levels;
mod player;
mod rng;
mod timestep;

pub use barrel::{Barrel, BarrelMode};
pub use input::Input;
pub use level::Level;
pub use levels::builtin as builtin_level;
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

pub const START_LIVES: u32 = 3;
/// Steps the hammer stays in the hand.
pub const HAMMER_STEPS: u32 = 10 * STEP_HZ;
/// Steps before the ape throws the first barrel (and after a death).
const FIRST_BARREL_STEPS: u32 = STEP_HZ;
pub const SCORE_JUMP: u32 = 100;
pub const SCORE_SMASH: u32 = 300;
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

#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    /// Simulation steps done since the start.
    pub tick: u64,
    pub level: Level,
    pub player: Player,
    pub barrels: Vec<Barrel>,
    /// Hammer items still lying around.
    pub hammers: Vec<(f32, f32)>,
    /// Steps left of the hammer in the player's hand (0 = none).
    pub hammer_left: u32,
    pub score: u32,
    pub lives: u32,
    pub bonus: u32,
    pub status: Status,
    extra_life_given: bool,
    barrel_in: u32,
    bonus_in: u32,
    pub rng: Rng,
}

impl GameState {
    pub fn new(level: Level, seed: u64) -> Self {
        Self {
            tick: 0,
            player: Player::new(level.start),
            barrels: Vec::new(),
            hammers: level.hammers.clone(),
            hammer_left: 0,
            score: 0,
            lives: START_LIVES,
            bonus: level.bonus,
            status: Status::Playing,
            extra_life_given: false,
            barrel_in: FIRST_BARREL_STEPS,
            bonus_in: level.bonus_every,
            rng: Rng::new(seed),
            level,
        }
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
        self.score += points;
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
            state.barrel_in = FIRST_BARREL_STEPS;
        }
        None => {}
    }

    // The ape throws barrels, the barrels move.
    state.barrel_in = state.barrel_in.saturating_sub(1);
    if state.barrel_in == 0 {
        let (ax, ay) = state.level.ape;
        state.barrels.push(Barrel::thrown(ax + 16.0, ay - 8.0, 1));
        state.barrel_in = state.level.barrel_every;
    }
    for b in &mut state.barrels {
        b.step(&state.level.geometry, &mut state.rng);
    }
    state.barrels.retain(|b| !b.is_gone());

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

    // A barrel that touches the player kills.
    if state.barrels.iter().any(|b| state.player.hit_by(b.rect())) && state.player.kill().is_some()
    {
        die(state, events);
        return;
    }

    // The goal.
    if hit.overlaps(state.goal_rect()) {
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
}
