# Architecture

How the program works inside. One `##` section per feature; update the section with every
change of the feature. Never read this file whole – grep the heading (see `CLAUDE.md`).

[← Back to README](../README.md)

## UI basics

`main.rs` loads the settings, sets the language and starts eframe. `app::App::ui` is the frame
loop: menu panel, status panel, central panel (the game view), then the windows (About), then a delayed write of
changed settings. The program repaints only on input or when a timer is pending
(`request_repaint_after`), so an idle window costs no CPU.

## Desktop and browser entry points

`main.rs` has two `fn main`s: desktop (`native::start`: window options, renderer choice, see
below) and wasm32 (`eframe::WebRunner` on the canvas `game_canvas` of `index.html`, which also
shows / removes the "Loading" text and any start error). Both create the same `app::App`.
`Cargo.toml` has two `eframe` entries: desktop with the default features + `glow`, wasm32 with
`default-features = false` + `glow` + `default_fonts` (WebGL, no wgpu, small bundle). Desktop-only
code is behind `cfg(not(target_arch = "wasm32"))`. `App` keeps its own clock from egui's frame
time (`now`), because `std::time::Instant` panics in the browser. `index.html` sets
`touch-action: none`, `overscroll-behavior: none`, a no-zoom viewport and a full-screen canvas so
a touch screen does not scroll or zoom the page while playing.

## Web deployment

`pages.yml` (push to `master`): Ubuntu runner, Rust + `wasm32-unknown-unknown`, prebuilt Trunk
(version + SHA-256 pinned in the workflow), `trunk build --release --public-url "/MMDonkeyKong/"`,
`actions/upload-pages-artifact` of `dist/`, `actions/deploy-pages`. The `web` job of `ci.yml` runs
the same build plus clippy for wasm32 on every push / pull request.

## Renderer and refresh frames

OpenGL (glow) is the default; when it cannot start (also a panic before the app exists) the
program starts again with wgpu, restricted to DX12 on Windows when a hardware DX12 adapter exists
(less memory). `MMDONKEYKONG_RENDERER=wgpu` forces wgpu.
Why: with wgpu the AMD driver kept one CPU core busy while idle (RusTrace issue #6).

## Localization and settings

`i18n.rs`: language files `key = value`, built-in (embedded) + external (`lang\` next to the exe,
`%APPDATA%\MMDonkeyKong\lang`) overriding key by key; fallback to English, then to the key.
`config.rs`: `Config` with `#[serde(default)]`, written through a temporary file (atomic rename).
The app marks `cfg_dirty` and `save_config` writes at most every 2 s and on exit.

## Game core (`src/game/`)

Pure simulation: no egui, no I/O, no wall-clock time, so it runs unchanged natively and in the
browser and is unit-tested.

- `GameState` + `step(&mut state, &Input) -> Vec<Event>`: one fixed step. `Input` holds the held
  directions, `jump_held` and the edge `jump_pressed`. `GameState` holds a `Geometry` and the
  `Level` (geometry, start, ape, goal, hammers, timings), the `Player`, the `Barrel`s, score, lives,
  time bonus and `Status` (`Playing` / `Complete` / `GameOver`; `step` does nothing otherwise).
- `Player` (`player.rs`, #5): `step(&Input, &Geometry) -> Option<PlayerEvent>`. `x` / `y` = middle of
  the feet, y down, f32 (IEEE basic ops only, so deterministic). `Mode`: `Walking`, `Climbing`,
  `Airborne {vx, vy, top}`, `Dead {left}`. Walking: x moves 1 px / step and the feet snap to the
  girder surface nearest to `y` within `SNAP` (4 px); no girder -> falls straight down. Ladders
  (`Geometry::ladders`, `broken` ones are not grabbable): up from below / down from the top when
  within `LADDER_REACH` of the ladder x; x snaps to the ladder, at either end the player steps onto
  the girder there. A ladder in reach is also grabbed in the air with up / down held (not right after take-off, `GRAB_MIN_VY`), and `jump_pressed` on a ladder jumps off it. Jump: `jump_pressed` on the ground, `vx` (-1/0/1) fixed at take-off - no air
  control; `JUMP_VY` -2.6, `GRAVITY` 0.15 -> about 22 px high and 35 px long. Landing is *swept*: a
  girder surface between the previous and the new `y` stops the fall, so even at `MAX_FALL_SPEED`
  nothing tunnels. A landing more than `FALL_LIMIT` (32 px) below the highest point (`top`) kills
  (`FellToDeath`); so does leaving the field. Hitbox: 6 x 12 px (`hitbox()`), narrower than the
  sprite; `hit_by(Rect)` is for hazards, `kill()` for outside kills. Death lasts `DEATH_STEPS`, then
  the player respawns at its spawn point (`Respawned`); lives are counted by the game (later).
- `Rng` (`rng.rs`): own xorshift64*, seeded; the same seed and inputs always give the same state
  (test `same_seed_and_inputs_give_the_same_state`).
- `FixedStep` (`timestep.rs`): the accumulator. `advance(dt)` returns how many 60 Hz
  (`STEP_HZ`) steps to run; the remainder carries over, a stall is capped at 0.1 s (the excess is
  dropped), NaN / negative `dt` is ignored.
- `Event` is what the front end turns into sound / effects. `src/audio.rs` has the `Audio` trait
  (`NullAudio`), `src/storage.rs` the `Storage` trait (`NullStorage`, `MemoryStorage`); the real
  backends come with #16 and #11. Until the front end uses them, their `mod` lines in `main.rs`
  carry `#[allow(dead_code)]` with that reason.

### Level format (`level.rs`, `levels.rs`, `levels/*.txt`, #6)

One `key: values` per line, `#` starts a comment. Keys: `name`, `type` (`barrels`), `bonus` /
`bonus_drop` / `bonus_every` (seconds), `barrel_every` (seconds), `extra_life` (score),
`start x y`, `ape x y`, `goal x y`, `girder x0 y0 x1 y1`, `ladder x top bottom [broken]`,
`hammer x y` (all in playfield pixels; ladder ends are girder surface heights). `Level::parse`
reports syntax / range errors as `line N: message` (`ParseError`, line 0 = whole file, e.g. a
missing key). `Level::validate` checks that start, goal, ape, hammers and ladder ends stand on
girders (within `SNAP`) and that the goal is reachable from the start by a graph search (girders =
nodes, intact ladders = edges; jumps are not needed). `levels.rs` embeds the files with
`include_str!`; the tests parse and validate every embedded file and check that every `levels/*.txt`
is embedded. A new level: add the file and one line in `FILES`.

### Barrels, hammer, scoring (`barrel.rs`, `mod.rs`)

The ape throws a barrel every `barrel_every` (the first after 1 s; a respawn clears the barrels and
restarts the timer). A barrel rolls 0.8 px / step downhill along its girder (flat girder: keeps the
throw direction); when it crosses the x of an intact ladder whose top is at its height, one in
three (from the seeded `Rng`) climbs down; at a girder end it falls (0.5 px / step outwards, gravity,
swept landing) onto the girder below and leaves the field at the bottom. The barrel box is 8 x 8.
A barrel touching the player's box kills (`Player::kill`, one life). Hammer: touching an item on the
ground gives 10 s (`HAMMER_STEPS`); meanwhile up / down / jump are ignored and barrels inside the
box stretched 12 px in front of the player are smashed (+300). A jump whose feet pass up to 14 px
above a barrel (within 6 px horizontally) scores +100 once per barrel. The time bonus loses
`bonus_drop` every `bonus_every` and is added to the score at the goal. One extra life when the
score reaches `extra_life`. Lives: 3; losing the last one ends the game after the death animation
(`GameOver`). The view restarts with Enter / R (a game over resets the score; after a completed
level the same level is replayed until the level select, #12).

## Game view (`src/app/game_view.rs`)

The central panel (no frame) is the game. `GameSession` (state, `FixedStep`, `paused`,
`jump_latch`) lives in `App`. Per frame: `handle_pause` (Esc / P toggle, click on the view toggles,
losing the window focus pauses - `InputState::focused`), then, when not paused, `run_steps`:
the keyboard is read once (`read_input`: arrows / WASD, Space), `jump_latch` remembers a Space
press until a step consumes it (a frame without a step does not lose it, a frame with two steps
delivers it once), `FixedStep::advance(unstable_dt)` says how many 60 Hz steps to run, and
`ctx.request_repaint()` keeps the game going. A paused game requests no repaint (idle CPU); the
frame time comes from egui, not `std::time`, so the same code works in the browser.

Drawing: the 224 x 256 playfield is scaled by `field_scale` - the largest whole number of
**physical** pixels that fits (crisp pixel art at any DPI), a fraction only when the area is
smaller than the field - centred and snapped to whole physical pixels; the rest is letter-box.
The status bar shows the key hint. Hiding the menu on touch screens is part of #8.

A frame longer than 0.5 s (hidden browser tab, sleep, window drag) pauses the game as well.
After a pause the first frame's time (the whole pause) is dropped (`skip_dt`). Click-to-pause on the
whole view (a pointer click checked by hand, not `Sense::click`, which would also fire on Enter / Space of a focused view) is a desktop convenience; #8 must replace it with a pause button, because on a touch
screen the d-pad and jump taps would toggle the pause.

## Rendering (`src/app/sprites.rs`, `scene.rs`)

Sprites are const arrays of text rows, one character per pixel (`palette`: `.` transparent, letters
are fixed colours - the UI light / dark theme never touches the playfield). The `sprites!` macro
declares `Id` and `Id::def()`; a sprite with `mirror` stores only its left half (ape, friend).
`decode` checks that rows have equal length and known characters (tests check every sprite and the
sizes the renderer assumes: hero 10 x 12, ape 24 x 24, barrel 8 x 8 ...). `Sprites::load` makes one
`NEAREST` texture per sprite once, in `App::new`. Sprites face right; the front end flips them.

`scene::render_background` rasterises girders (8 rows below the walking surface, rivets) and
ladders (broken ones miss their middle third) once per level into a 224 x 256 texture kept in
`GameSession::background`; a new session rebuilds it. `scene::draw_scene` blits that, then friend,
hammers, ape, barrels, hero (+ hammer in the hand) and explosions at whole playfield pixels, so
the art stays crisp at the integer scale. Frame choice is in pure functions (`hero_sprite`,
`ape_sprite`, `barrel_sprite`, `explosion_sprite`): the walk cycle runs by tick while the hero
moved in the last step (`GameSession::hero_moving`), the climb flips every 4 px of height, rolling
barrels turn with their x, the ape shows the throw for `APE_THROW_STEPS` after a barrel
(`GameState::ape_throwing`). A hammer smash spawns an `Effect` (age in steps, `EXPLOSION_STEPS`).

## Tests

`cargo test`: `i18n` (file format, completeness of the built-in languages, language resolution),
`config` (defaults of a partial file), `icon` (pixels and `.ico` layout).
