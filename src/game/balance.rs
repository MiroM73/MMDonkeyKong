//! Balance check of the difficulty steps: two bots – a weak and a strong player – play every
//! built-in level on every difficulty and the result is printed as a table.
//!
//! The bots play through the real simulation (`step`) with the real `Input`, nothing is faked.
//! Every few steps a bot tries each of its moves on a copy of the game, looks a short time ahead
//! and takes the move that keeps it alive and brings it nearest to the goal (along the girders,
//! ladders and lifts of the level, see `Nav`). The copies get a fresh random seed, so the bot
//! cannot see which ladder a barrel will take – like a human, it has to guess.
//!
//! What makes the weak bot weak: it reacts late (what it decides now is done a quarter of a second
//! later), now and then even later than it planned (a jump or a stop comes too late), it looks
//! ahead only a little and it guesses the barrels from one rollout instead of the worst of several.
//!
//! The run is slow (a few minutes in release), so the tests are ignored:
//! `cargo test --release balance -- --ignored --nocapture`.
//! The goal of the difficulty table (docs/game-design.md -> Difficulty): the weak bot finishes every
//! level on Relaxed, the strong bot finishes every level on Insane.

use super::player::{CLIMB_SPEED, GRAVITY, JUMP_VY, Mode, SNAP, WALK_SPEED};
use super::validate::{JUMP_DROP, JUMP_REACH, JUMP_RISE, WALK_DROP};
use super::*;

/// A bot's skill. A bot decides every `SLOT` steps; what it decides now is done only after its
/// reaction time (the moves decided before are queued until then and it knows them).
#[derive(Clone, Copy, Debug)]
struct Skill {
    name: &'static str,
    /// Reaction time in slots of `SLOT` steps.
    delay: usize,
    /// Steps the bot looks ahead behind its next move.
    horizon: usize,
    /// Rollouts (each with another guess of the barrels) per candidate; the worst one counts.
    guesses: u64,
    /// Chance (per mille per slot) that the bot reacts one slot later than it planned: the old
    /// move goes on a bit too long (a jump too late, a stop too late).
    late: u32,
}

/// Steps of one decision of a bot.
const SLOT: usize = 4;

/// Reacts in 0.25 s, sometimes later, looks ahead 0.4 s, takes the barrels as they come.
const WEAK: Skill = Skill {
    name: "weak",
    delay: 4,
    horizon: 24,
    guesses: 1,
    late: 150,
};

/// Reacts at once, looks ahead 0.7 s, plays safe against two guesses of the barrels.
const STRONG: Skill = Skill {
    name: "strong",
    delay: 0,
    horizon: 40,
    guesses: 2,
    late: 0,
};

/// Weight of every next distance sample (every 4 steps) in a rollout.
const DISCOUNT: f32 = 0.95;

/// How long the second move of a plan may be held before the bot stands still (or the whole
/// look-ahead).
const HOLDS: [usize; 6] = [4, 8, 12, 16, 24, 32];

/// Game time a bot gets for one level before it counts as stuck (3 minutes).
const MAX_STEPS: u32 = 3 * 60 * STEP_HZ;

/// The moves a bot can choose from (a jump is pressed on the first step of the move only).
fn moves() -> [Input; 9] {
    let k = |left, right, up, down, jump| Input {
        left,
        right,
        up,
        down,
        jump_held: jump,
        jump_pressed: jump,
        auto_grab: false,
    };
    [
        k(false, false, false, false, false),
        k(true, false, false, false, false),
        k(false, true, false, false, false),
        k(false, false, true, false, false),
        k(false, false, false, true, false),
        k(false, false, false, false, true),
        k(true, false, false, false, true),
        k(false, true, false, false, true),
        // Up while jumping sideways grabs a ladder on the way.
        k(false, false, true, false, true),
    ]
}

/// The input of step `k` of a held move: the jump is an edge, pressed only on the first step.
fn held(m: Input, k: usize) -> Input {
    Input {
        jump_pressed: m.jump_pressed && k == 0,
        ..m
    }
}

/// How far (in steps of walking / climbing / riding) every place of a level is from the goal –
/// or, on the rivets level, from every rivet. A graph of points on the girders: both ends of every
/// intact ladder, the girder ends one can walk or jump off, the stops next to a lift, the targets.
struct Nav {
    /// (girder, x, y) of every point; only the fixed girders (`GameState::scenery`).
    nodes: Vec<(usize, f32, f32)>,
    /// Per target the distance of every node to it.
    dist: Vec<Vec<f32>>,
    /// The position of every target (the goal, or the rivets in level order).
    targets: Vec<(f32, f32)>,
    /// Per lift the nodes one can step onto it from.
    lift_stops: Vec<Vec<usize>>,
    /// Number of fixed girders (the lifts follow them in `level.geometry.girders`).
    fixed: usize,
}

const FAR: f32 = 1.0e5;
/// The distance of a player in the air over no ground: not counted.
const UNKNOWN: f32 = f32::INFINITY;
/// Horizontal length of a full jump on flat ground.
const JUMP_LEN: f32 = 2.0 * -JUMP_VY / GRAVITY * WALK_SPEED;

/// The girder whose surface at (`x`, `y`) is nearest to `y`, within `SNAP`.
fn girder_at(g: &Geometry, x: f32, y: f32) -> Option<usize> {
    g.girders
        .iter()
        .enumerate()
        .filter_map(|(i, gi)| gi.y_at(x).map(|s| (i, (s - y).abs())))
        .filter(|&(_, d)| d <= SNAP)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

impl Nav {
    fn new(state: &GameState) -> Self {
        let geo = state.scenery();
        let level = &state.level;
        let mut nodes: Vec<(usize, f32, f32)> = Vec::new();
        let mut edges: Vec<(usize, usize, f32)> = Vec::new();
        let node = |nodes: &mut Vec<(usize, f32, f32)>, g: usize, x: f32| {
            let y = geo.girders[g].y_at(x).unwrap_or(geo.girders[g].y0);
            nodes.push((g, x, y));
            nodes.len() - 1
        };

        // Ladders, both ways.
        for l in geo.ladders.iter().filter(|l| !l.broken) {
            if let (Some(b), Some(t)) =
                (girder_at(&geo, l.x, l.bottom), girder_at(&geo, l.x, l.top))
            {
                let (nb, nt) = (node(&mut nodes, b, l.x), node(&mut nodes, t, l.x));
                let cost = (l.bottom - l.top) / CLIMB_SPEED + 4.0;
                edges.push((nb, nt, cost));
                edges.push((nt, nb, cost));
            }
        }

        // Walking or jumping off a girder end onto another girder (the rules of the validator).
        for (i, a) in geo.girders.iter().enumerate() {
            for (x_end, y_end, dir) in [(a.x0, a.y0, -1.0_f32), (a.x1, a.y1, 1.0_f32)] {
                for (j, b) in geo.girders.iter().enumerate() {
                    if i == j {
                        continue;
                    }
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
                        b.x0 + 2.0
                    } else {
                        b.x1 - 2.0
                    };
                    let Some(y_b) = b.y_at(probe) else { continue };
                    let dy = y_b - y_end;
                    let ok = if gap == 0.0 {
                        (-SNAP..=WALK_DROP).contains(&dy)
                    } else {
                        gap <= JUMP_REACH && (-JUMP_RISE..=JUMP_DROP).contains(&dy)
                    };
                    if ok {
                        let from = node(&mut nodes, i, x_end - dir);
                        let to = node(&mut nodes, j, probe);
                        edges.push((from, to, gap / WALK_SPEED + 30.0));
                    }
                }
            }
        }

        // Lifts: every girder end next to the ride is a stop; riding costs the height and a wait.
        let half = LIFT_WIDTH / 2.0;
        let mut lift_stops = Vec::new();
        for lift in &level.lifts {
            let mut stops = Vec::new();
            for (i, gi) in geo.girders.iter().enumerate() {
                for (x, y) in [(gi.x0, gi.y0), (gi.x1, gi.y1)] {
                    let gap = ((lift.x - half) - x).max(x - (lift.x + half)).max(0.0);
                    if gap <= JUMP_REACH && y >= lift.top - SNAP && y <= lift.bottom + SNAP {
                        // The stop is where a jump that lands in the middle of the lift starts.
                        let dir = if x <= gi.x0 { 1.0 } else { -1.0 };
                        let take_off = (lift.x + dir * JUMP_LEN).clamp(gi.x0 + 1.0, gi.x1 - 1.0);
                        stops.push(node(&mut nodes, i, take_off));
                    }
                }
            }
            for &a in &stops {
                for &b in &stops {
                    if a != b {
                        let (xa, ya) = (nodes[a].1, nodes[a].2);
                        let (xb, yb) = (nodes[b].1, nodes[b].2);
                        let cost = (xa - lift.x).abs()
                            + (xb - lift.x).abs()
                            + (ya - yb).abs() / (LIFT_SPEED * 0.6)
                            + 120.0;
                        edges.push((a, b, cost));
                    }
                }
            }
            lift_stops.push(stops);
        }

        // The targets.
        let targets: Vec<(f32, f32)> = if level.kind == LevelKind::Rivets {
            level.rivets.clone()
        } else {
            vec![level.goal]
        };
        let target_nodes: Vec<usize> = targets
            .iter()
            .map(|&(x, y)| {
                let g = girder_at(&geo, x, y).expect("every target lies on a girder");
                node(&mut nodes, g, x)
            })
            .collect();

        // Walking along a girder between any two of its points.
        for a in 0..nodes.len() {
            for b in 0..nodes.len() {
                if a != b && nodes[a].0 == nodes[b].0 {
                    edges.push((a, b, (nodes[a].1 - nodes[b].1).abs() / WALK_SPEED));
                }
            }
        }

        // Distances to every target (Dijkstra backwards over the edges).
        let n = nodes.len();
        let dist = target_nodes
            .iter()
            .map(|&t| {
                let mut d = vec![FAR; n];
                let mut done = vec![false; n];
                d[t] = 0.0;
                while let Some(u) = (0..n)
                    .filter(|&i| !done[i] && d[i] < FAR)
                    .min_by(|&a, &b| d[a].total_cmp(&d[b]))
                {
                    done[u] = true;
                    for &(from, to, c) in &edges {
                        if to == u && d[u] + c < d[from] {
                            d[from] = d[u] + c;
                        }
                    }
                }
                d
            })
            .collect();

        Nav {
            nodes,
            dist,
            targets,
            lift_stops,
            fixed: geo.girders.len(),
        }
    }

    /// Distance to target `t` from (`x`) on fixed girder `g`.
    fn on_girder(&self, t: usize, g: usize, x: f32) -> f32 {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.0 == g)
            .map(|(i, n)| (n.1 - x).abs() / WALK_SPEED + self.dist[t][i])
            .fold(FAR, f32::min)
    }

    /// Distance to target `t` from a point standing on surface `k` of the live geometry.
    fn on_surface(&self, t: usize, k: usize, x: f32, y: f32) -> f32 {
        if k < self.fixed {
            return self.on_girder(t, k, x);
        }
        // On a lift: ride to a stop, step off.
        self.lift_stops[k - self.fixed]
            .iter()
            .map(|&i| {
                let n = self.nodes[i];
                (n.1 - x).abs() + (n.2 - y).abs() / LIFT_SPEED + self.dist[t][i]
            })
            .fold(FAR, f32::min)
    }

    /// Distance of the player to target `t`.
    fn to_target(&self, s: &GameState, t: usize) -> f32 {
        let geo = &s.level.geometry;
        let p = &s.player;
        match p.mode {
            Mode::Walking => match girder_at(geo, p.x, p.y) {
                Some(k) => self.on_surface(t, k, p.x, p.y),
                None => FAR,
            },
            Mode::Climbing { ladder } => {
                let l = geo.ladders[ladder];
                let up = girder_at(geo, l.x, l.top)
                    .map(|k| (p.y - l.top) / CLIMB_SPEED + self.on_surface(t, k, l.x, l.top));
                let down = girder_at(geo, l.x, l.bottom)
                    .map(|k| (l.bottom - p.y) / CLIMB_SPEED + self.on_surface(t, k, l.x, l.bottom));
                up.unwrap_or(FAR).min(down.unwrap_or(FAR))
            }
            // In the air over nothing (yet): a lift may still come; a real fall ends in a death,
            // which the rollout sees anyway.
            Mode::Airborne { .. } => match geo.landing(p.x, p.y, f32::MAX) {
                Some(s_y) => match girder_at(geo, p.x, s_y) {
                    Some(k) => (s_y - p.y).abs() / 3.0 + self.on_surface(t, k, p.x, s_y),
                    None => UNKNOWN,
                },
                None => UNKNOWN,
            },
            Mode::Dead { .. } => FAR,
        }
    }

    /// How far the player is from finishing the level (lower is better).
    fn cost(&self, s: &GameState) -> f32 {
        if s.level.kind != LevelKind::Rivets {
            return self.to_target(s, 0);
        }
        // Rivets: every rivet left is worth a lot, plus the way to the nearest one.
        let nearest = self
            .targets
            .iter()
            .enumerate()
            .filter(|(_, r)| s.rivets.contains(r))
            .map(|(t, _)| self.to_target(s, t))
            .fold(f32::INFINITY, f32::min);
        s.rivets.len() as f32 * 1000.0 + nearest
    }
}

/// A look-ahead on a copy of the game with a guessed future (the barrels' choices).
/// Alive, its value is minus the mean distance to the goal over the rollout, the near future
/// weighted more, so of two plans that end at the same place the one that gets there sooner wins.
#[derive(Clone)]
struct Roll {
    s: GameState,
    t: f32,
    sum: f32,
    weights: f32,
    w: f32,
    /// Set once the rollout ended: won or dead.
    end: Option<f32>,
    /// The move of the previous step and how long it is held.
    last: Input,
    k: usize,
}

impl Roll {
    fn new(state: &GameState, guess: u64) -> Self {
        let mut s = state.clone();
        s.rng = Rng::new(guess);
        Roll {
            s,
            t: 0.0,
            sum: 0.0,
            weights: 0.0,
            w: 1.0,
            end: None,
            last: Input::default(),
            k: 0,
        }
    }

    /// One step with move `m` (held on when it is the move of the previous step).
    fn step(&mut self, m: Input, nav: &Nav) {
        if self.end.is_some() {
            return;
        }
        self.k = if m == self.last { self.k + 1 } else { 0 };
        self.last = m;
        step(&mut self.s, &held(m, self.k));
        self.t += 1.0;
        if self.s.status == Status::Complete {
            self.end = Some(1.0e7 - self.t);
        } else if self.s.player.is_dead() || self.s.status == Status::GameOver {
            self.end = Some(-1.0e7 + self.t);
        } else if (self.t as u32).is_multiple_of(4) {
            let c = nav.cost(&self.s);
            if c.is_finite() {
                self.sum += self.w * c;
                self.weights += self.w;
            }
            self.w *= DISCOUNT;
        }
    }

    fn run(&mut self, m: Input, n: usize, nav: &Nav) {
        for _ in 0..n {
            self.step(m, nav);
        }
    }

    /// The value; a jump still in the air is played to its landing first (a fatal one counts).
    fn value(mut self, nav: &Nav) -> f32 {
        for _ in 0..4 * STEP_HZ {
            if self.end.is_some() || !self.s.player.is_airborne() {
                break;
            }
            self.step(Input::default(), nav);
        }
        if let Some(v) = self.end {
            return v;
        }
        let c = nav.cost(&self.s);
        let c = if c.is_finite() { c } else { FAR };
        -((self.sum + self.w * c) / (self.weights + self.w))
    }
}

/// The values of every plan "the queued moves, `a`, then `b` for a while, then stand still" for one
/// guess, in a fixed order (the same for every guess). `start` is the game as the bot plays it.
fn plan_values(
    start: &Roll,
    queue: &[Input],
    skill: Skill,
    a: Input,
    guess: u64,
    nav: &Nav,
) -> Vec<f32> {
    let all = moves();
    let mut base = start.clone();
    base.s.rng = Rng::new(guess);
    for &q in queue {
        base.run(q, SLOT, nav);
    }
    base.run(a, SLOT, nav);
    let mut values = Vec::new();
    for &b in &all {
        let mut r = base.clone();
        for k in 1..=skill.horizon {
            r.step(b, nav);
            if HOLDS.contains(&k) && k < skill.horizon {
                let mut tail = r.clone();
                tail.run(all[0], skill.horizon - k, nav);
                values.push(tail.value(nav));
            }
        }
        values.push(r.value(nav));
    }
    values
}

/// What one bot run of one level gave.
#[derive(Clone, Copy, Debug, Default)]
struct Outcome {
    completed: bool,
    deaths: u32,
    steps: u32,
    /// The steps since the last death (or the start): the length of the life that won. The time
    /// bonus starts again after a death, so this is the one to compare with it.
    life_steps: u32,
}

/// One bot plays `level` on `difficulty` from the full lives until it wins, loses all lives or
/// runs out of time.
fn play(level: usize, difficulty: Difficulty, skill: Skill, seed: u64) -> Outcome {
    let s = GameState::start(builtin_level(level), seed, difficulty, None, 0);
    play_state(s, skill, seed)
}

/// A bot plays on from `s`.
fn play_state(s: GameState, skill: Skill, seed: u64) -> Outcome {
    let nav = Nav::new(&s);
    let all = moves();
    let mut bot = Rng::new(seed ^ 0xB07);
    // The real game, played through a `Roll` so that held moves work as in the look-ahead.
    let mut game = Roll::new(&s, 0);
    game.s.rng = s.rng.clone();
    let mut queue: std::collections::VecDeque<Input> = vec![all[0]; skill.delay].into();
    let mut out = Outcome::default();
    let mut guess = seed.wrapping_mul(1000);
    while out.steps < MAX_STEPS {
        // Decide the move after the queued ones: the best plan counted with its worst guess.
        let next = if game.s.player.is_dead() {
            all[0]
        } else {
            let q: Vec<Input> = queue.iter().copied().collect();
            let mut probe = game.clone();
            probe.t = 0.0;
            probe.sum = 0.0;
            probe.weights = 0.0;
            probe.w = 1.0;
            let mut best = (f32::MIN, all[0]);
            for &a in &all {
                let per_guess: Vec<Vec<f32>> = (0..skill.guesses)
                    .map(|g| plan_values(&probe, &q, skill, a, guess + g, &nav))
                    .collect();
                let value = (0..per_guess[0].len())
                    .map(|p| per_guess.iter().map(|v| v[p]).fold(f32::MAX, f32::min))
                    .fold(f32::MIN, f32::max);
                if value > best.0 {
                    best = (value, a);
                }
            }
            guess += skill.guesses;
            best.1
        };
        queue.push_back(next);
        // Play one slot: the oldest queued move, or – reacting late – the move before once more.
        let planned = queue.pop_front().unwrap_or(all[0]);
        let m = if bot.below(1000) < skill.late {
            game.last
        } else {
            planned
        };
        for _ in 0..SLOT {
            let lives = game.s.lives;
            game.step(m, &nav);
            out.steps += 1;
            out.life_steps += 1;
            if game.s.lives < lives {
                out.deaths += 1;
                out.life_steps = 0;
            }
            match game.s.status {
                Status::Complete => {
                    out.completed = true;
                    return out;
                }
                Status::GameOver => return out,
                Status::Playing => {}
            }
            // The look-ahead's end marks are not wanted for the real game.
            game.end = None;
        }
    }
    out
}

/// Runs every level × difficulty × seed for `skill` on all cores; `[difficulty][level]`.
fn run_all(skill: Skill, seeds: u64) -> Vec<Vec<Vec<Outcome>>> {
    run_levels(skill, seeds, &(0..level_count()).collect::<Vec<_>>())
}

/// Like `run_all`, but only the given levels (0-based); the others stay empty.
fn run_levels(skill: Skill, seeds: u64, levels: &[usize]) -> Vec<Vec<Vec<Outcome>>> {
    let jobs: Vec<(usize, usize, u64)> = (0..Difficulty::ALL.len())
        .flat_map(|d| {
            levels
                .iter()
                .flat_map(move |&l| (1..=seeds).map(move |s| (d, l, s)))
        })
        .collect();
    let threads = std::thread::available_parallelism().map_or(2, |n| n.get());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results =
        std::sync::Mutex::new(vec![vec![Vec::new(); level_count()]; Difficulty::ALL.len()]);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(&(d, l, seed)) = jobs.get(i) else {
                        break;
                    };
                    let o = play(l, Difficulty::ALL[d], skill, seed);
                    results.lock().unwrap()[d][l].push(o);
                }
            });
        }
    });
    results.into_inner().unwrap()
}

/// Prints one table: per difficulty and level how many runs were won and the deaths per run.
fn report(skill: Skill, r: &[Vec<Vec<Outcome>>]) {
    println!(
        "\n{} bot: won runs / deaths per run (a run = the lives of one level)",
        skill.name
    );
    print!("{:<8}", "");
    for l in 0..level_count() {
        print!(" {:>11}", builtin_level(l).name);
    }
    println!();
    for (d, row) in r.iter().enumerate() {
        print!("{:<8}", format!("{:?}", Difficulty::ALL[d]));
        for runs in row {
            if runs.is_empty() {
                print!(" {:>11}", "-");
                continue;
            }
            let won = runs.iter().filter(|o| o.completed).count();
            let deaths: u32 = runs.iter().map(|o| o.deaths).sum();
            print!(
                " {:>5} {:>5.1}",
                format!("{won}/{}", runs.len()),
                deaths as f32 / runs.len() as f32
            );
        }
        println!();
    }
}

/// Levels where not a single run of `difficulty` was won.
fn lost_levels(r: &[Vec<Vec<Outcome>>], difficulty: Difficulty) -> Vec<String> {
    let d = Difficulty::ALL
        .iter()
        .position(|&x| x == difficulty)
        .unwrap();
    r[d].iter()
        .enumerate()
        .filter(|(_, runs)| !runs.iter().any(|o| o.completed))
        .map(|(l, _)| builtin_level(l).name)
        .collect()
}

fn seeds() -> u64 {
    std::env::var("BALANCE_SEEDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(4)
}

/// The bots themselves work: with nothing in the way both bots walk every level to the end.
#[test]
#[ignore = "slow; run with --release"]
fn balance_bots_finish_every_empty_level() {
    for skill in [WEAK, STRONG] {
        for l in 0..level_count() {
            let mut s = GameState::start(builtin_level(l), 1, Difficulty::Normal, None, 0);
            // No barrels, springs or fireballs.
            s.level.barrel_every = u32::MAX;
            s.barrel_in = u32::MAX;
            assert!(
                Nav::new(&s).cost(&s) < FAR,
                "{}: no way to the goal",
                s.level.name
            );
            let o = play_state(s, skill, 1);
            assert!(
                o.completed,
                "{} bot is stuck on level {}: {o:?}",
                skill.name,
                l + 1
            );
        }
    }
}

/// The weak bot must win every level on Relaxed, the strong bot every level on Insane (at least
/// one run of `BALANCE_SEEDS`, default 4; a lost run on Relaxed only costs the level's restart).
/// `cargo test --release balance_difficulty_steps -- --ignored --nocapture` (BALANCE_SEEDS=8 for
/// a closer look).
#[test]
#[ignore = "slow (minutes); run with --release"]
fn balance_difficulty_steps() {
    let n = seeds();
    let weak = run_all(WEAK, n);
    report(WEAK, &weak);
    let strong = run_all(STRONG, n);
    report(STRONG, &strong);
    report_bonus_time(&strong);
    let weak_lost = lost_levels(&weak, Difficulty::Relaxed);
    let strong_lost = lost_levels(&strong, Difficulty::Insane);
    assert!(
        weak_lost.is_empty() && strong_lost.is_empty(),
        "weak bot never won on Relaxed: {weak_lost:?}; strong bot never won on Insane: {strong_lost:?}"
    );
    let slow = too_slow_levels(&strong);
    assert!(
        slow.is_empty(),
        "strong bot on Insane needs over {LIMIT_PCT} % of the bonus time: {slow:?}"
    );
}

/// The steps the time bonus of `level` lasts: `bonus` / `bonus_drop` drops, one every `bonus_every`
/// steps (the score multiplier scales both, so the difficulty does not change it).
fn bonus_steps(level: usize) -> u32 {
    let l = builtin_level(level);
    l.bonus / l.bonus_drop.max(1) * l.bonus_every
}

/// Levels where a winning life of the strong bot on Insane needed more than `LIMIT_PCT` percent of
/// the bonus time, as `(level name, longest life in steps, bonus steps)`.
fn too_slow_levels(r: &[Vec<Vec<Outcome>>]) -> Vec<(String, u32, u32)> {
    let d = Difficulty::ALL
        .iter()
        .position(|&x| x == Difficulty::Insane)
        .unwrap();
    r[d].iter()
        .enumerate()
        .filter_map(|(l, runs)| {
            let longest = runs
                .iter()
                .filter(|o| o.completed)
                .map(|o| o.life_steps)
                .max()?;
            let limit = bonus_steps(l);
            (longest * 100 > limit * LIMIT_PCT).then(|| (builtin_level(l).name, longest, limit))
        })
        .collect()
}

/// Insane takes a life when the time bonus runs out (#35): the strong bot must finish every level
/// well before that. A winning life may take at most this share of the bonus time.
const LIMIT_PCT: u32 = 60;

/// Prints how much of the bonus time the longest winning life of the strong bot on Insane used.
fn report_bonus_time(r: &[Vec<Vec<Outcome>>]) {
    let d = Difficulty::ALL.len() - 1;
    println!(
        "
strong bot on Insane: longest winning life vs. the bonus time"
    );
    for (l, runs) in r[d].iter().enumerate() {
        if runs.is_empty() {
            continue;
        }
        let longest = runs
            .iter()
            .filter(|o| o.completed)
            .map(|o| o.life_steps)
            .max();
        let limit = bonus_steps(l);
        match longest {
            Some(n) => println!(
                "  {:<14} {:>5.1} s of {:>5.1} s ({} %)",
                builtin_level(l).name,
                n as f32 / STEP_HZ as f32,
                limit as f32 / STEP_HZ as f32,
                n * 100 / limit
            ),
            None => println!("  {:<14} no win", builtin_level(l).name),
        }
    }
}

/// Prints the tables of both bots for some levels only, to try a change quickly:
/// `BALANCE_LEVELS=2,4,6 BALANCE_SEEDS=8 cargo test --release balance_explore -- --ignored --nocapture`
/// (levels counted from 1; all of them when unset). Asserts nothing.
#[test]
#[ignore = "slow; run with --release"]
fn balance_explore() {
    let levels: Vec<usize> = match std::env::var("BALANCE_LEVELS") {
        Ok(v) => v
            .split(',')
            .filter_map(|x| x.trim().parse::<usize>().ok())
            .filter(|&l| (1..=level_count()).contains(&l))
            .map(|l| l - 1)
            .collect(),
        Err(_) => (0..level_count()).collect(),
    };
    let n = seeds();
    for skill in [WEAK, STRONG] {
        let r = run_levels(skill, n, &levels);
        report(skill, &r);
        if skill.name == STRONG.name {
            report_bonus_time(&r);
        }
    }
}
