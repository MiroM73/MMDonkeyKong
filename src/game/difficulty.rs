//! The five difficulty steps and the one table of parameters behind them
//! (`docs/game-design.md` -> Difficulty). Difficulty only scales numbers, never the level geometry.
//! The numbers are a first proposal and are tuned by playing.

use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Relaxed,
    Easy,
    #[default]
    Normal,
    Hard,
    Insane,
}

/// What a difficulty changes. `Normal` is the 1.0x baseline the game was balanced for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// Lives at the start of a game (and after a game over).
    pub lives: u32,
    /// Speed of the barrels (rolling, climbing a ladder, drifting off a girder end).
    pub obstacle_speed: f32,
    /// The time between two barrels in percent of the level's `barrel_every`
    /// (more = rarer barrels).
    pub spawn_interval_pct: u32,
    /// The chance that a barrel crossing an intact ladder climbs down: `num` in `den`.
    pub ladder_chance: (u32, u32),
    /// Hammers added to (+) or taken from (-) the level's own ones. Added ones come from the
    /// level's `bonus_hammer` lines (as many as it has).
    pub hammer_delta: i32,
    /// Every point (jumps, smashes, the time bonus) in percent; the extra-life score scales too.
    pub score_pct: u32,
}

impl Difficulty {
    pub const ALL: [Difficulty; 5] = [
        Difficulty::Relaxed,
        Difficulty::Easy,
        Difficulty::Normal,
        Difficulty::Hard,
        Difficulty::Insane,
    ];

    /// The language-file key of the name, e.g. `difficulty.hard`.
    pub fn key(self) -> &'static str {
        match self {
            Difficulty::Relaxed => "difficulty.relaxed",
            Difficulty::Easy => "difficulty.easy",
            Difficulty::Normal => "difficulty.normal",
            Difficulty::Hard => "difficulty.hard",
            Difficulty::Insane => "difficulty.insane",
        }
    }

    pub fn params(self) -> Params {
        let (lives, obstacle_speed, spawn_interval_pct, ladder_chance, hammer_delta, score_pct) =
            match self {
                Difficulty::Relaxed => (7, 0.60, 160, (1, 10), 2, 50),
                Difficulty::Easy => (5, 0.80, 130, (1, 5), 1, 75),
                Difficulty::Normal => (3, 1.00, 100, (1, 3), 0, 100),
                Difficulty::Hard => (3, 1.20, 80, (2, 5), 0, 150),
                Difficulty::Insane => (2, 1.45, 65, (1, 2), -1, 200),
            };
        Params {
            lives,
            obstacle_speed,
            spawn_interval_pct,
            ladder_chance,
            hammer_delta,
            score_pct,
        }
    }
}

impl Params {
    /// `points` scaled by the score multiplier.
    pub fn scaled(&self, points: u32) -> u32 {
        (u64::from(points) * u64::from(self.score_pct) / 100) as u32
    }

    #[cfg(test)]
    fn ladder_probability(&self) -> f32 {
        self.ladder_chance.0 as f32 / self.ladder_chance.1 as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> Vec<Params> {
        Difficulty::ALL.iter().map(|d| d.params()).collect()
    }

    #[test]
    fn harder_means_faster_denser_and_fewer_lives() {
        let p = all();
        for w in p.windows(2) {
            assert!(w[0].lives >= w[1].lives);
            assert!(w[0].obstacle_speed < w[1].obstacle_speed);
            assert!(w[0].spawn_interval_pct > w[1].spawn_interval_pct);
            assert!(w[0].ladder_probability() < w[1].ladder_probability());
            assert!(w[0].hammer_delta >= w[1].hammer_delta);
            assert!(w[0].score_pct < w[1].score_pct);
        }
        assert!(p[0].lives > p[4].lives);
        assert!(p[0].hammer_delta > p[4].hammer_delta);
    }

    #[test]
    fn every_parameter_is_within_sane_bounds() {
        for p in all() {
            assert!((1..=9).contains(&p.lives));
            assert!((0.3..=2.0).contains(&p.obstacle_speed));
            assert!((40..=250).contains(&p.spawn_interval_pct));
            let (n, d) = p.ladder_chance;
            assert!(n >= 1 && n < d);
            assert!((-1..=3).contains(&p.hammer_delta));
            assert!((25..=300).contains(&p.score_pct));
        }
    }

    /// Normal is the baseline the game was balanced for: nothing is scaled.
    #[test]
    fn normal_is_the_baseline() {
        let p = Difficulty::Normal.params();
        assert_eq!(Difficulty::default(), Difficulty::Normal);
        assert_eq!(p.lives, 3);
        assert_eq!(p.obstacle_speed, 1.0);
        assert_eq!(p.spawn_interval_pct, 100);
        assert_eq!(p.ladder_chance, (1, 3));
        assert_eq!(p.hammer_delta, 0);
        assert_eq!(p.score_pct, 100);
        assert_eq!(p.scaled(300), 300);
    }

    #[test]
    fn scaling_the_score() {
        assert_eq!(Difficulty::Relaxed.params().scaled(100), 50);
        assert_eq!(Difficulty::Easy.params().scaled(100), 75);
        assert_eq!(Difficulty::Insane.params().scaled(300), 600);
    }

    #[test]
    fn order_and_serde_names() {
        assert!(Difficulty::ALL.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(
            serde_json::to_string(&Difficulty::Hard).unwrap(),
            "\"hard\""
        );
        let d: Difficulty = serde_json::from_str("\"relaxed\"").unwrap();
        assert_eq!(d, Difficulty::Relaxed);
    }
}
