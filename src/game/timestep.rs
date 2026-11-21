//! Fixed time step: the screen refreshes at any rate (30 Hz phone, 144 Hz monitor), the game
//! simulation always runs at exactly `STEP_HZ` steps per second.

/// Simulation steps per second.
pub const STEP_HZ: u32 = 60;
const STEP: f64 = 1.0 / STEP_HZ as f64;
/// After a stall (window drag, tab in the background) at most this much time is caught up; the
/// rest is dropped, so the game never freezes in a "spiral of death" of catching up.
const MAX_CATCH_UP: f64 = 0.1;

#[derive(Clone, Debug, Default)]
pub struct FixedStep {
    acc: f64,
}

impl FixedStep {
    /// Adds the time of one frame (seconds) and returns how many simulation steps to run now.
    pub fn advance(&mut self, dt: f32) -> u32 {
        let dt = if dt.is_finite() {
            f64::from(dt.max(0.0))
        } else {
            0.0
        };
        self.acc = (self.acc + dt).min(MAX_CATCH_UP);
        // The small epsilon keeps exact multiples (dt = 1/60) from rounding down to one step less.
        let n = (self.acc / STEP + 1e-9).floor();
        self.acc = (self.acc - n * STEP).max(0.0);
        n as u32
    }

    /// Forgets the collected time (after a pause).
    pub fn reset(&mut self) {
        self.acc = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steps_in_one_second(fps: u32) -> u32 {
        let mut f = FixedStep::default();
        (0..fps).map(|_| f.advance(1.0 / fps as f32)).sum()
    }

    #[test]
    fn step_rate_does_not_depend_on_frame_rate() {
        for fps in [30, 60, 75, 120, 144, 240] {
            let n = steps_in_one_second(fps);
            assert!((59..=60).contains(&n), "{fps} fps gave {n} steps");
        }
    }

    #[test]
    fn remainder_carries_over() {
        let mut f = FixedStep::default();
        assert_eq!(f.advance(0.010), 0);
        assert_eq!(f.advance(0.010), 1); // 0.020 s >= 1/60 s
        assert_eq!(f.advance(0.010), 0); // 0.0033 + 0.010 < 1/60
    }

    #[test]
    fn long_stall_is_capped() {
        let mut f = FixedStep::default();
        let n = f.advance(10.0);
        assert!((1..=7).contains(&n), "{n}");
        assert_eq!(
            f.advance(0.0),
            0,
            "the excess is dropped, not caught up later"
        );
    }

    #[test]
    fn bad_input_is_ignored() {
        let mut f = FixedStep::default();
        assert_eq!(f.advance(f32::NAN), 0);
        assert_eq!(f.advance(-1.0), 0);
        assert_eq!(f.advance(f32::INFINITY), 0);
    }
}
