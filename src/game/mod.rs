//! The platform-independent game: no egui, no I/O, no wall-clock time. The front end (`app`)
//! feeds it an `Input` once per fixed step (`STEP_HZ`) and plays the returned `Event`s.
//! With a seed and an input sequence the result is always the same (deterministic, testable).
//!
//! The levels (issue #6) will supply the geometry; until then `demo_geometry` stands in.

mod input;
mod player;
#[allow(dead_code)] // the barrels (#6) draw random numbers
mod rng;
mod timestep;

pub use input::Input;
pub use player::{Geometry, Girder, Ladder, Player, PlayerEvent};
pub use rng::Rng;
pub use timestep::FixedStep;

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
    #[allow(dead_code)] // hazards (#6)
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
}

#[derive(Clone, Debug, PartialEq)]
pub struct GameState {
    /// Simulation steps done since the start.
    pub tick: u64,
    pub geometry: Geometry,
    pub player: Player,
    pub rng: Rng,
}

/// Temporary test level: two sloped girders over a floor, a ladder and a gap.
pub fn demo_geometry() -> Geometry {
    Geometry {
        girders: vec![
            Girder::new(0.0, 248.0, 224.0, 248.0),
            Girder::new(0.0, 205.0, 190.0, 215.0),
            Girder::new(30.0, 172.0, 224.0, 162.0),
            Girder::new(0.0, 120.0, 90.0, 120.0),
            Girder::new(110.0, 120.0, 224.0, 125.0),
        ],
        ladders: vec![
            Ladder {
                x: 200.0,
                top: 214.0,
                bottom: 248.0,
                broken: false,
            },
            Ladder {
                x: 60.0,
                top: 174.0,
                bottom: 207.0,
                broken: false,
            },
            Ladder {
                x: 160.0,
                top: 125.0,
                bottom: 166.0,
                broken: false,
            },
        ],
    }
}

impl GameState {
    pub fn new(seed: u64) -> Self {
        Self {
            tick: 0,
            geometry: demo_geometry(),
            player: Player::new((20.0, 248.0)),
            rng: Rng::new(seed),
        }
    }
}

/// Runs one fixed step and returns the events it produced.
pub fn step(state: &mut GameState, input: &Input) -> Vec<Event> {
    state.tick += 1;
    let e = state.player.step(input, &state.geometry);
    e.map(|e| match e {
        PlayerEvent::Jumped => Event::Jump,
        PlayerEvent::Landed => Event::Land,
        PlayerEvent::FellToDeath | PlayerEvent::Killed => Event::Died,
        PlayerEvent::Respawned => Event::Respawn,
    })
    .into_iter()
    .collect()
}

#[cfg(test)]
mod tests {
    use super::player::Mode;
    use super::*;

    fn run(seed: u64, inputs: &[Input]) -> (GameState, Vec<Event>) {
        let mut s = GameState::new(seed);
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

    #[test]
    fn same_seed_and_inputs_give_the_same_state() {
        assert_eq!(run(5, &script()), run(5, &script()));
        assert_ne!(run(5, &script()).0, run(6, &script()).0);
    }

    #[test]
    fn player_stays_on_the_floor_when_idle() {
        let (s, _) = run(1, &vec![Input::default(); 100]);
        assert_eq!((s.player.y, s.player.mode), (248.0, Mode::Walking));
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
}
