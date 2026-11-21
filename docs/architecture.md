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
  `Player`; the geometry is `demo_geometry()` (a test level drawn as lines) until the levels (#6).
- `Player` (`player.rs`, #5): `step(&Input, &Geometry) -> Option<PlayerEvent>`. `x` / `y` = middle of
  the feet, y down, f32 (IEEE basic ops only, so deterministic). `Mode`: `Walking`, `Climbing`,
  `Airborne {vx, vy, top}`, `Dead {left}`. Walking: x moves 1 px / step and the feet snap to the
  girder surface nearest to `y` within `SNAP` (4 px); no girder -> falls straight down. Ladders
  (`Geometry::ladders`, `broken` ones are not grabbable): up from below / down from the top when
  within `LADDER_REACH` of the ladder x; x snaps to the ladder, at either end the player steps onto
  the girder there. Jump: `jump_pressed` on the ground, `vx` (-1/0/1) fixed at take-off - no air
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

Planned (see `docs/game-design.md`, `docs/platforms.md`): levels parsed from `levels/*.txt`.

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
whole view is a desktop convenience; #8 must replace it with a pause button, because on a touch
screen the d-pad and jump taps would toggle the pause.

## Tests

`cargo test`: `i18n` (file format, completeness of the built-in languages, language resolution),
`config` (defaults of a partial file), `icon` (pixels and `.ico` layout).
