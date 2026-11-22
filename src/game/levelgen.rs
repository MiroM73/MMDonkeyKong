//! The level generator: `generate(kind, seed, size_hint)` builds a level from a seed. Every
//! result passes the validator (`validate.rs`); a candidate that fails is re-rolled with the next
//! sub-seed. Only `+ - * /` and the seeded `Rng` are used, so a seed gives the same level on every
//! platform. Print a level as a file with the ignored test `print_generated_level` (see below).

use super::level::{Belt, Level, LevelKind, Lift};
use super::{Geometry, Girder, Ladder, Rng, STEP_HZ, builtin_level};

/// How many sub-seeds are tried before the generator gives up and returns the shipped level.
const MAX_ATTEMPTS: u64 = 64;
/// Number of floors (girders from the bottom to the ape's) a level may have.
const FLOORS: std::ops::RangeInclusive<u32> = 4..=7;
const BOTTOM_Y: f32 = 248.0;
const TOP_Y: f32 = 60.0;
/// Horizontal room a ladder keeps from the ends of its girders and from other ladders.
const LADDER_MARGIN: f32 = 12.0;
const LADDER_SPACING: f32 = 16.0;

/// Rounds to 0.1 so the generated file is readable and parses back to the very same level.
fn r1(v: f32) -> f32 {
    (v * 10.0).round() / 10.0
}

/// A valid level of `kind` for `seed`. `size_hint` is the wanted number of floors (clamped to
/// 4–7; the generator may differ when the first candidates are invalid).
pub fn generate(kind: LevelKind, seed: u64, size_hint: u32) -> Level {
    for attempt in 0..MAX_ATTEMPTS {
        let sub = seed.wrapping_add(attempt.wrapping_mul(0x9E37_79B9));
        let level = build(kind, sub, size_hint, seed);
        if level.validate().is_ok() {
            return level;
        }
    }
    // Never reached by the shipped generators (the tests prove it); a playable level anyway.
    builtin_level(0)
}

fn kind_title(kind: LevelKind) -> &'static str {
    match kind {
        LevelKind::Barrels => "Barrels",
        LevelKind::Conveyors => "Conveyors",
        LevelKind::Lifts => "Lifts",
        LevelKind::Rivets => "Rivets",
    }
}

/// One candidate (not validated yet).
fn build(kind: LevelKind, sub_seed: u64, size_hint: u32, name_seed: u64) -> Level {
    match kind {
        LevelKind::Lifts => lifts(sub_seed, name_seed),
        LevelKind::Barrels | LevelKind::Conveyors | LevelKind::Rivets => {
            stacked(kind, sub_seed, size_hint, name_seed)
        }
    }
}

/// Stacked girders of alternating slope (barrels roll down them and fall off the open end),
/// 2–3 ladders between neighbouring floors (one of them may be broken), hammers on the way up.
/// `Conveyors` uses flat floors that carry their loads: a belt on every floor but the first and
/// the last, running towards the open end and reversing every 5–9 s.
fn stacked(kind: LevelKind, seed: u64, size_hint: u32, name_seed: u64) -> Level {
    let belts_level = kind == LevelKind::Conveyors;
    // Only the barrels have sloped floors.
    let flat = kind != LevelKind::Barrels;
    let mut rng = Rng::new(seed);
    let n = size_hint.clamp(*FLOORS.start(), *FLOORS.end()) as usize;
    let spacing = (BOTTOM_Y - TOP_Y) / (n - 1) as f32;
    let mut girders = Vec::new();
    for i in 0..n {
        let base = BOTTOM_Y - i as f32 * spacing;
        let g = if i == 0 {
            Girder::new(0.0, 248.0, 224.0, if flat { 248.0 } else { 240.0 })
        } else if i == n - 1 {
            Girder::new(0.0, TOP_Y, (130 + 10 * rng.range(0, 4)) as f32, TOP_Y)
        } else {
            // Steeper floors need more room between them (see MIN_CLEARANCE).
            let steepest = ((spacing - 24.0) as i32).clamp(6, 10);
            let drop = if flat {
                0.0
            } else {
                rng.range(6, steepest) as f32
            };
            let (x0, x1) = if i % 2 == 1 {
                (0.0, (184 + 4 * rng.range(0, 3)) as f32)
            } else {
                ((28 - 4 * rng.range(0, 3)) as f32, 224.0)
            };
            // Odd floors fall to the right (barrels roll right), even floors to the left.
            let (ya, yb) = if i % 2 == 1 {
                (base - drop / 2.0, base + drop / 2.0)
            } else {
                (base + drop / 2.0, base - drop / 2.0)
            };
            Girder::new(x0, r1(ya), x1, r1(yb))
        };
        girders.push(g);
    }

    let mut ladders: Vec<Ladder> = Vec::new();
    for i in 0..n - 1 {
        let (lo, hi) = (&girders[i], &girders[i + 1]);
        let from = lo.x0.max(hi.x0) + LADDER_MARGIN;
        let to = lo.x1.min(hi.x1) - LADDER_MARGIN;
        let wanted = rng.range(2, 3);
        let clashes = |ladders: &[Ladder], x: f32| {
            ladders.iter().any(|l| {
                (l.x - x).abs() < LADDER_SPACING
                    && [i, i + 1].iter().any(|&f| ends_on(l, &girders, f))
            })
        };
        let mut placed = 0;
        let mut tries = 0;
        while placed < wanted && tries < 40 {
            tries += 1;
            let x = (from + rng.range(0, ((to - from) / 5.0) as i32) as f32 * 5.0).round();
            if clashes(&ladders, x) {
                continue;
            }
            // The first ladder of a floor pair is always intact, the others break now and then.
            let broken = placed > 0 && rng.below(3) == 0;
            ladders.push(ladder_between(lo, hi, x, broken));
            placed += 1;
        }
        if placed == 0 {
            // Crowded: the first free spot, scanned from the left, so the pair is never cut off.
            let mut x = from.round();
            while x <= to && clashes(&ladders, x) {
                x += 1.0;
            }
            ladders.push(ladder_between(lo, hi, x, false));
        }
    }

    let mut hammer_floors: Vec<usize> = (1..n - 1).collect();
    let mut pick = |rng: &mut Rng| {
        if hammer_floors.is_empty() {
            1.min(n - 1)
        } else {
            hammer_floors.remove(rng.below(hammer_floors.len() as u32) as usize)
        }
    };
    let item = |rng: &mut Rng, floor: usize| {
        let g = &girders[floor];
        let x =
            (g.x0 + 24.0 + rng.range(0, ((g.x1 - g.x0 - 48.0) / 5.0) as i32) as f32 * 5.0).round();
        (x, r1(g.y_at(x).unwrap_or(0.0)))
    };
    let (f1, f2, f3, f4) = (
        pick(&mut rng),
        pick(&mut rng),
        pick(&mut rng),
        pick(&mut rng),
    );
    let hammers = vec![item(&mut rng, f1), item(&mut rng, f2)];
    let bonus_hammers = vec![item(&mut rng, f3), item(&mut rng, f4)];

    let belts = if belts_level {
        (1..n - 1)
            .map(|girder| Belt {
                girder,
                dir: if girder % 2 == 1 { 1 } else { -1 },
                every: STEP_HZ * rng.range(5, 9) as u32,
            })
            .collect()
    } else {
        Vec::new()
    };
    // 1.5 to 4 seconds in half-second steps (written back as exact seconds); fireballs come
    // slower (4 to 8 seconds).
    let barrel_every = if kind == LevelKind::Rivets {
        STEP_HZ * rng.range(4, 8) as u32
    } else {
        (STEP_HZ / 2) * rng.range(3, 8) as u32
    };
    // Rivets sit near the ends of every floor above the bottom one.
    let rivets: Vec<(f32, f32)> = if kind == LevelKind::Rivets {
        (1..n)
            .flat_map(|i| {
                let g = girders[i];
                let left = if i == n - 1 { g.x0 + 40.0 } else { g.x0 + 16.0 };
                [left, g.x1 - 16.0].map(|x| (x, r1(g.y_at(x).unwrap_or(0.0))))
            })
            .collect()
    } else {
        Vec::new()
    };
    Level {
        name: format!("{} {name_seed}", kind_title(kind)),
        kind,
        bonus: 5000,
        bonus_drop: 100,
        bonus_every: 2 * STEP_HZ,
        barrel_every,
        extra_life: 7000,
        start: (20.0, r1(girders[0].y_at(20.0).unwrap_or(0.0))),
        ape: (20.0, TOP_Y),
        goal: (70.0, TOP_Y),
        geometry: Geometry { girders, ladders },
        hammers,
        bonus_hammers,
        belts,
        lifts: Vec::new(),
        rivets,
    }
}

/// A lifts level in one of two layouts, chosen by the seed: one lift to an upper floor, or a
/// two-stage climb over a middle floor.
fn lifts(seed: u64, name_seed: u64) -> Level {
    let mut rng = Rng::new(seed);
    if rng.below(2) == 0 {
        lifts_one_stage(&mut rng, name_seed)
    } else {
        lifts_two_stages(&mut rng, name_seed)
    }
}

/// A start platform, two rising lifts with a middle floor between them, a ledge on the right and a
/// ladder up to the ape's floor. The ape drops springs.
fn lifts_two_stages(rng: &mut Rng, name_seed: u64) -> Level {
    let half = super::LIFT_WIDTH / 2.0;
    let start_end = (40 + 8 * rng.range(0, 2)) as f32;
    let lift_a = start_end + half + rng.range(6, 14) as f32;
    let mid_y = r1((168 + rng.range(0, 16)) as f32);
    let mid_x0 = lift_a + half + rng.range(6, 14) as f32;
    let mid_x1 = mid_x0 + (40 + 8 * rng.range(0, 3)) as f32;
    let lift_b = mid_x1 + half + rng.range(6, 14) as f32;
    let ledge_y = r1((100 + rng.range(0, 12)) as f32);
    let ledge_x = lift_b + half + rng.range(6, 14) as f32;
    let ladder_x = (ledge_x + 12.0 + 2.0 * rng.range(0, 4) as f32).round();
    let girders = vec![
        Girder::new(0.0, 248.0, start_end, 248.0),
        Girder::new(mid_x0, mid_y, mid_x1, mid_y),
        Girder::new(ledge_x, ledge_y, 224.0, ledge_y),
        Girder::new(0.0, TOP_Y, ladder_x + 10.0, TOP_Y),
        Girder::new(164.0, 248.0, 224.0, 248.0),
    ];
    let ladders = vec![Ladder {
        x: ladder_x,
        top: TOP_Y,
        bottom: ledge_y,
        broken: false,
    }];
    let lifts = vec![
        Lift {
            x: lift_a,
            top: mid_y,
            bottom: 248.0,
            dir: -1,
        },
        Lift {
            x: lift_b,
            top: ledge_y,
            bottom: mid_y,
            dir: -1,
        },
    ];
    Level {
        name: format!("Lifts {name_seed}"),
        kind: LevelKind::Lifts,
        bonus: 9000,
        bonus_drop: 100,
        bonus_every: 2 * STEP_HZ,
        barrel_every: (STEP_HZ / 2) * rng.range(3, 7) as u32,
        extra_life: 16000,
        start: (20.0, 248.0),
        ape: (20.0, TOP_Y),
        goal: (70.0, TOP_Y),
        geometry: Geometry { girders, ladders },
        hammers: vec![((mid_x0 + mid_x1) / 2.0, mid_y), (ledge_x + 14.0, ledge_y)],
        bonus_hammers: vec![(30.0, 248.0), (mid_x0 + 8.0, mid_y)],
        belts: Vec::new(),
        lifts,
        rivets: Vec::new(),
    }
}

/// A start platform, a lift that rises to an upper floor, a ladder up to the ape's floor and a
/// second lift that comes down. The ape drops springs.
fn lifts_one_stage(rng: &mut Rng, name_seed: u64) -> Level {
    let half = super::LIFT_WIDTH / 2.0;
    let start_end = (40 + 8 * rng.range(0, 2)) as f32;
    let lift_x = start_end + half + rng.range(6, 14) as f32;
    let upper_y = r1((92 + rng.range(0, 16)) as f32);
    let upper_x = lift_x + half + rng.range(6, 14) as f32;
    let top_end = (130 + 10 * rng.range(0, 4)) as f32;
    let girders = vec![
        Girder::new(0.0, 248.0, start_end, 248.0),
        Girder::new(upper_x, upper_y, 140.0, upper_y),
        Girder::new(0.0, TOP_Y, top_end, TOP_Y),
        Girder::new(164.0, 248.0, 224.0, 248.0),
    ];
    let ladder_x = (upper_x
        + 12.0
        + rng.range(
            0,
            ((top_end.min(140.0) - 12.0 - upper_x - 12.0) / 2.0) as i32,
        ) as f32
            * 2.0)
        .round();
    let ladders = vec![Ladder {
        x: ladder_x,
        top: TOP_Y,
        bottom: upper_y,
        broken: false,
    }];
    let hammers = vec![(128.0, upper_y), (190.0, 248.0)];
    let lifts = vec![
        Lift {
            x: lift_x,
            top: upper_y,
            bottom: 248.0,
            dir: -1,
        },
        Lift {
            x: 156.0,
            top: upper_y,
            bottom: 248.0,
            dir: 1,
        },
    ];
    Level {
        name: format!("Lifts {name_seed}"),
        kind: LevelKind::Lifts,
        bonus: 7000,
        bonus_drop: 100,
        bonus_every: 2 * STEP_HZ,
        barrel_every: (STEP_HZ / 2) * rng.range(4, 8) as u32,
        extra_life: 12000,
        start: (20.0, 248.0),
        ape: (20.0, TOP_Y),
        goal: (70.0, TOP_Y),
        geometry: Geometry { girders, ladders },
        hammers,
        bonus_hammers: vec![(30.0, 248.0), (190.0, 248.0)],
        belts: Vec::new(),
        lifts,
        rivets: Vec::new(),
    }
}

fn ladder_between(lo: &Girder, hi: &Girder, x: f32, broken: bool) -> Ladder {
    Ladder {
        x,
        top: r1(hi.y_at(x).unwrap_or(0.0)),
        bottom: r1(lo.y_at(x).unwrap_or(0.0)),
        broken,
    }
}

/// Does ladder `l` end on girder `floor` (top or bottom)?
fn ends_on(l: &Ladder, girders: &[Girder], floor: usize) -> bool {
    girders[floor].y_at(l.x).is_some_and(|s| {
        (s - l.top).abs() < super::player::SNAP || (s - l.bottom).abs() < super::player::SNAP
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Prints a generated level as a file:
    /// `GEN_TYPE=barrels GEN_SEED=7 GEN_SIZE=5 cargo test print_generated_level -- --ignored --nocapture`
    #[test]
    #[ignore = "prints a level; run by hand"]
    fn print_generated_level() {
        let var = |k: &str, d: &str| std::env::var(k).unwrap_or_else(|_| d.to_string());
        let kind = *LevelKind::ALL
            .iter()
            .find(|k| k.as_str() == var("GEN_TYPE", "barrels"))
            .expect("GEN_TYPE is a level type");
        let seed: u64 = var("GEN_SEED", "1").parse().expect("GEN_SEED is a number");
        let size: u32 = var("GEN_SIZE", "6").parse().expect("GEN_SIZE is a number");
        let level = build(kind, seed, size, seed);
        println!(
            "# Generated: type {} seed {seed} size {size}\n{}",
            kind.as_str(),
            level.to_text()
        );
    }

    #[test]
    fn hundreds_of_seeds_validate_for_every_type() {
        for &kind in LevelKind::ALL {
            for seed in 0..300u64 {
                let size = 4 + (seed % 4) as u32;
                // The first candidate must already be valid: no re-rolling needed.
                let first = build(kind, seed, size, seed);
                assert_eq!(first.validate(), Ok(()), "{kind:?} seed {seed} size {size}");
                let level = build(kind, seed, size, seed);
                assert_eq!(level.kind, kind);
                assert_eq!(level, first, "{kind:?} seed {seed} was re-rolled");
            }
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_level_and_other_seeds_differ() {
        for &kind in LevelKind::ALL {
            assert_eq!(generate(kind, 9, 6), generate(kind, 9, 6));
            assert_ne!(generate(kind, 9, 6), generate(kind, 10, 6));
        }
    }

    #[test]
    fn generated_levels_round_trip_through_the_file_format() {
        for &kind in LevelKind::ALL {
            for seed in 0..50 {
                let level = generate(kind, seed, 6);
                let parsed = Level::parse(&level.to_text()).unwrap();
                assert_eq!(parsed, level, "{kind:?} seed {seed}");
            }
        }
    }

    #[test]
    fn size_hint_is_clamped_and_sets_the_floors() {
        assert_eq!(generate(LevelKind::Barrels, 1, 0).geometry.girders.len(), 4);
        assert_eq!(
            generate(LevelKind::Barrels, 1, 99).geometry.girders.len(),
            7
        );
        assert_eq!(generate(LevelKind::Barrels, 1, 5).geometry.girders.len(), 5);
    }

    #[test]
    fn the_validator_rejects_broken_levels() {
        let ok = generate(LevelKind::Barrels, 3, 5);
        // A floor lifted into the one above: too close.
        let mut l = ok.clone();
        l.geometry.girders[1].y0 -= 25.0;
        l.geometry.girders[1].y1 -= 25.0;
        assert!(l.validate().is_err());
        // Every ladder broken: the goal is out of reach.
        let mut l = ok.clone();
        l.geometry.ladders.iter_mut().for_each(|x| x.broken = true);
        assert_eq!(
            l.validate(),
            Err("the goal cannot be reached from the start".into())
        );
        // A ladder that stays on one girder.
        let mut l = ok.clone();
        l.geometry.ladders[0].bottom = l.geometry.ladders[0].top;
        assert!(l.validate().is_err());
    }
}
