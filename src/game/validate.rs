//! The level validator: can this level be played at all? Pure. Used by `Level::validate`, by the
//! tests of every shipped level and by the generator (`levelgen.rs`), which re-rolls a level that
//! fails. The limits come from the player constants, so a change of the physics changes them too.

use super::level::Level;
use super::player::{FALL_LIMIT, GRAVITY, HIT_HEIGHT, JUMP_VY, SNAP, WALK_SPEED};
use super::{Geometry, Girder};

/// A jump is planned with this fraction of its real reach, so a level never needs a perfect jump.
const JUMP_MARGIN: f32 = 0.8;
/// Horizontal distance covered by a jump (time in the air times the walking speed).
pub const JUMP_REACH: f32 = 2.0 * -JUMP_VY / GRAVITY * WALK_SPEED * JUMP_MARGIN;
/// How much higher than the take-off a jump can still land.
pub const JUMP_RISE: f32 = JUMP_VY * JUMP_VY / (2.0 * GRAVITY) * JUMP_MARGIN;
/// How far below the take-off a jump may land (the fall is counted from the top of the arc).
pub const JUMP_DROP: f32 = FALL_LIMIT - JUMP_VY * JUMP_VY / (2.0 * GRAVITY) - 1.0;
/// How far a walk off a girder end may fall (kept one pixel inside the lethal height).
pub const WALK_DROP: f32 = FALL_LIMIT - 1.0;
/// Girders above each other keep at least this much room (the player and some air).
pub const MIN_CLEARANCE: f32 = HIT_HEIGHT + 8.0;
/// How far below the ape a barrel must find a girder.
const BARREL_DROP_LIMIT: f32 = 48.0;

/// The girder whose surface at (`x`, `y`) is within `SNAP` of `y`.
fn girder_at(g: &Geometry, x: f32, y: f32) -> Option<usize> {
    g.girders
        .iter()
        .position(|gi| gi.y_at(x).is_some_and(|s| (s - y).abs() <= SNAP))
}

/// Girders `a` and `b` over their common x range: `Err` when they cross or come closer than
/// `MIN_CLEARANCE`.
fn check_pair(a: &Girder, b: &Girder) -> Result<(), String> {
    let (x0, x1) = (a.x0.max(b.x0), a.x1.min(b.x1));
    if x0 > x1 {
        return Ok(());
    }
    let d0 = a.y_at(x0).unwrap_or(0.0) - b.y_at(x0).unwrap_or(0.0);
    let d1 = a.y_at(x1).unwrap_or(0.0) - b.y_at(x1).unwrap_or(0.0);
    if d0.signum() != d1.signum() && d0 != 0.0 && d1 != 0.0 {
        return Err(format!(
            "the girders at x={x0}..{x1} cross each other (y {} and {})",
            a.y_at(x0).unwrap_or(0.0),
            b.y_at(x0).unwrap_or(0.0)
        ));
    }
    if d0.abs().min(d1.abs()) < MIN_CLEARANCE {
        return Err(format!(
            "two girders are closer than {MIN_CLEARANCE} px at x={x0}..{x1}"
        ));
    }
    Ok(())
}

/// Moves between girders the player can make: climbing intact ladders (both ways), walking off an
/// end onto a girder not far below, and jumping over a gap. `from -> to` pairs.
fn moves(g: &Geometry, ladder_edges: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut edges = Vec::new();
    for &(a, b) in ladder_edges {
        edges.push((a, b));
        edges.push((b, a));
    }
    for (i, a) in g.girders.iter().enumerate() {
        for (x_end, y_end, dir) in [(a.x0, a.y0, -1.0_f32), (a.x1, a.y1, 1.0_f32)] {
            for (j, b) in g.girders.iter().enumerate() {
                if i == j {
                    continue;
                }
                // Distance from the end to the nearest point of `b` in walking direction; a
                // girder under the end counts as 0 (the walk off the end lands on it).
                let gap = if b.x0 <= x_end && x_end <= b.x1 {
                    0.0
                } else if dir > 0.0 && b.x0 > x_end {
                    b.x0 - x_end
                } else if dir < 0.0 && b.x1 < x_end {
                    x_end - b.x1
                } else {
                    continue;
                };
                let probe = if gap == 0.0 {
                    x_end + dir * 2.0
                } else if dir > 0.0 {
                    b.x0
                } else {
                    b.x1
                };
                let Some(y_b) = b.y_at(probe) else { continue };
                let dy = y_b - y_end; // positive = `b` is lower
                if gap == 0.0 {
                    // Walking off: the surface must be below the end (not a wall) and near.
                    if (-SNAP..=WALK_DROP).contains(&dy) {
                        edges.push((i, j));
                    }
                } else if gap <= JUMP_REACH && (-JUMP_RISE..=JUMP_DROP).contains(&dy) {
                    edges.push((i, j));
                }
            }
        }
    }
    edges
}

/// Checks that `level` can be played. See `docs/architecture.md` → Level format / validator.
pub fn validate(level: &Level) -> Result<(), String> {
    let g = &level.geometry;
    if g.girders.is_empty() {
        return Err("the level has no girder".into());
    }
    let on = |x: f32, y: f32| girder_at(g, x, y);
    let start = on(level.start.0, level.start.1).ok_or("the start is not on a girder")?;
    let goal = on(level.goal.0, level.goal.1).ok_or("the goal is not on a girder")?;
    on(level.ape.0, level.ape.1).ok_or("the ape is not on a girder")?;
    let hammers: Vec<(f32, f32)> = level
        .hammers
        .iter()
        .chain(&level.bonus_hammers)
        .copied()
        .collect();
    let mut hammer_girders = Vec::new();
    for (x, y) in &hammers {
        hammer_girders.push(
            on(*x, *y).ok_or_else(|| format!("the hammer at ({x}, {y}) is not on a girder"))?,
        );
    }

    // The ape's barrels need something to land on.
    let (bx, by) = (level.ape.0 + 16.0, level.ape.1 - 8.0);
    if g.girders.iter().all(|gi| {
        gi.y_at(bx)
            .is_none_or(|s| s < by - SNAP || s > by + BARREL_DROP_LIMIT)
    }) {
        return Err("a barrel thrown by the ape would land on no girder".into());
    }

    for (i, a) in g.girders.iter().enumerate() {
        for b in &g.girders[i + 1..] {
            check_pair(a, b)?;
        }
    }

    let mut ladder_edges = Vec::new();
    for l in &g.ladders {
        let top = on(l.x, l.top)
            .ok_or_else(|| format!("the top of the ladder at x={} is not on a girder", l.x))?;
        let bottom = on(l.x, l.bottom)
            .ok_or_else(|| format!("the bottom of the ladder at x={} is not on a girder", l.x))?;
        if top == bottom {
            return Err(format!(
                "the ladder at x={} starts and ends on the same girder",
                l.x
            ));
        }
        let walled = g.girders.iter().enumerate().any(|(i, gi)| {
            i != top
                && i != bottom
                && gi
                    .y_at(l.x)
                    .is_some_and(|s| s > l.top + SNAP && s < l.bottom - SNAP)
        });
        if walled {
            return Err(format!("the ladder at x={} runs into a girder", l.x));
        }
        if !l.broken {
            ladder_edges.push((top, bottom));
        }
    }

    let edges = moves(g, &ladder_edges);
    let mut seen = vec![false; g.girders.len()];
    let mut todo = vec![start];
    seen[start] = true;
    while let Some(n) = todo.pop() {
        for &(from, to) in &edges {
            if from == n && !seen[to] {
                seen[to] = true;
                todo.push(to);
            }
        }
    }
    if !seen[goal] {
        return Err("the goal cannot be reached from the start".into());
    }
    for ((x, y), gi) in hammers.iter().zip(&hammer_girders) {
        if !seen[*gi] {
            return Err(format!("the hammer at ({x}, {y}) cannot be reached"));
        }
    }
    Ok(())
}
