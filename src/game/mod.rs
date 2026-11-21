//! The platform-independent game: no egui, no I/O, no wall-clock time. The front end (`app`)
//! feeds it an `Input` once per fixed step (`STEP_HZ`) and plays the returned `Event`s.
//! With a seed and an input sequence the result is always the same (deterministic, testable).
//!
//! The state is a placeholder until the player physics (issue #5) and the levels (issue #6) land.

mod input;
mod rng;
mod timestep;

pub use input::Input;
pub use rng::Rng;
pub use timestep::FixedStep;

/// Logical playfield size in pixels; the front end scales it to the screen.
pub const FIELD_W: i32 = 224;
pub const FIELD_H: i32 = 256;
/// Side of the placeholder marker.
pub const MARKER_SIZE: i32 = 12;
const MARKER_SPEED: i32 = 1;

/// Something that happened during a step; the front end turns it into sound and effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Jump,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameState {
    /// Simulation steps done since the start.
    pub tick: u64,
    /// Placeholder marker position (top left, playfield pixels); replaced by the player.
    pub x: i32,
    pub y: i32,
    pub rng: Rng,
}

impl GameState {
    pub fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let x = rng.below((FIELD_W - MARKER_SIZE) as u32) as i32;
        Self {
            tick: 0,
            x,
            y: FIELD_H - MARKER_SIZE,
            rng,
        }
    }
}

/// Runs one fixed step and returns the events it produced.
pub fn step(state: &mut GameState, input: &Input) -> Vec<Event> {
    let mut events = Vec::new();
    state.tick += 1;
    let dx = i32::from(input.right) - i32::from(input.left);
    let dy = i32::from(input.down) - i32::from(input.up);
    state.x = (state.x + dx * MARKER_SPEED).clamp(0, FIELD_W - MARKER_SIZE);
    state.y = (state.y + dy * MARKER_SPEED).clamp(0, FIELD_H - MARKER_SIZE);
    if input.jump_pressed {
        events.push(Event::Jump);
    }
    events
}

#[cfg(test)]
mod tests {
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
    fn marker_stays_inside_the_field() {
        let right_down = Input {
            right: true,
            down: true,
            ..Input::default()
        };
        let (s, _) = run(1, &vec![right_down; 1000]);
        assert_eq!((s.x, s.y), (FIELD_W - MARKER_SIZE, FIELD_H - MARKER_SIZE));
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
