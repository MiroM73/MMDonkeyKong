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
  the girder there. A ladder in reach is also grabbed in the air with up / down held (not right after take-off, `GRAB_MIN_VY`), and `jump_pressed` on a ladder jumps off it. **Touch assist** (`Input::auto_grab`, set when the on-screen controls are in use): a jump that started sideways also grabs a ladder *ahead* of it without "up" (a thumb cannot hold sideways + up on a d-pad); the ladder must be in the jump direction so a sideways jump off a ladder does not catch it again. Keyboard and gamepad keep the original rule. Jump: `jump_pressed` on the ground, `vx` (-1/0/1) fixed at take-off - no air
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
  (`NullAudio`; its `mod` line in `main.rs` carries `#[allow(dead_code)]` until #16 uses it),
  `src/storage.rs` the `Storage` trait (see Saved progress).

### Level format (`level.rs`, `levels.rs`, `levels/*.txt`, #6)

One `key: values` per line, `#` starts a comment. Keys: `name`, `type` (`barrels`), `bonus` /
`bonus_drop` / `bonus_every` (seconds), `barrel_every` (seconds), `extra_life` (score),
`start x y`, `ape x y`, `goal x y`, `girder x0 y0 x1 y1`, `ladder x top bottom [broken]`,
`hammer x y`, `bonus_hammer x y` (extra hammers of the easier difficulties; all in playfield pixels; ladder ends are girder surface heights). `Level::parse`
reports syntax / range errors as `line N: message` (`ParseError`, line 0 = whole file, e.g. a
missing key). `Level::validate` checks that start, goal, ape, hammers and ladder ends stand on
girders (within `SNAP`) and that the goal is reachable from the start by a graph search (girders =
nodes, intact ladders = edges; jumps are not needed). `levels.rs` embeds the files with
`include_str!`; the tests parse and validate every embedded file and check that every `levels/*.txt`
is embedded. A new level: add the file and one line in `FILES`.

### Difficulty (`difficulty.rs`, #10)

`Difficulty` (Relaxed, Easy, Normal, Hard, Insane; serde name = lowercase) and `Params`, one table
in `Difficulty::params()`: `lives`, `obstacle_speed` (multiplies the barrel speed), `spawn_interval_pct`
(percent of the level's `barrel_every`), `ladder_chance` (`num` in `den`, drawn as
`rng.below(den) < num`, so Normal `1/3` consumes the `Rng` exactly as before), `hammer_delta` and
`score_pct`. `GameState::start(level, seed, difficulty, lives, score)` applies them once, to its own
copy of the level: `barrel_every`, `bonus`, `bonus_drop` and `extra_life` are scaled, hammers are
added from the level's `bonus_hammer` lines (Relaxed +2, Easy +1; as many as the level has) or the
last ones removed (Insane -1, never the last one); `add_score` scales jump / smash points. `lives` /
`score` carry a game from one level to the next (`None` = full lives of the difficulty); a carried
score that already earned the extra life does not earn it again. Geometry is never touched. Normal
is bit-identical to the pre-#10 game (tests). The difficulty of the next level that starts is
`Config::difficulty` (menu Game -> Difficulty); the running level keeps its numbers.
The numbers are a first proposal, not tuned by playing yet.

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
(`GameOver`). What happens after the level ends is in Saved progress.

## Saved progress (`src/save.rs`, `src/storage.rs`, `src/app/progress.rs`, #11)

`Storage` (`load(key)`, `save(key, value) -> Result`) has `FileStorage::new(dir)` (desktop: key `save`
-> `%APPDATA%\MMDonkeyKong\save.json`, key `save.bak` -> `save.json.bak`; written to a temporary file
and renamed, **no** direct-write fallback, so a failed write leaves the old file untouched),
`WebStorage` (wasm: `localStorage["mmdonkeykong.<key>"]`), `NullStorage` and (tests) `MemoryStorage`;
`platform_storage()` picks one. `Config` goes through `WebStorage` (key `config`) in the browser too.
The tests of `FileStorage` use a folder under the system temp folder, never `%APPDATA%`.

`SaveData` (pure, serde, `#[serde(default)]`): `version` (1; a file without it is version 0 and is
migrated by stamping the version, a newer one is `TooNew`), `completed` (level indices completed on
*any* difficulty = unlocked for all) and per `Difficulty` a `Progress`: `resume` (level, lives, score
at the **start** of that level), `best` (level -> best score, only levels completed on that
difficulty), `high_scores`. `SaveData::load(storage, level_count)` never fails: a missing file gives
an empty save; garbage or a too-new version is copied to `save.bak` and the game starts fresh (the
status bar says so); impossible values (resume level out of range, zero lives) are repaired by
`sanitize`. Rules (all unit-tested): `new_game` sets the resume to level 1 with full lives;
`complete_level` unlocks, keeps the best score and moves the resume **forward only** (replaying an
earlier level never lowers it; after the last level it is cleared); `game_over` restarts the resume
level with full lives and zero score; `is_playable(completed, level, count)` = level 0, a completed
level, or the one after a completed level.

Front end (`progress.rs`, `screens.rs`): `Screen::Title` / `Screen::Game`. The title screen has
Continue (only with a resume point of the selected difficulty), New game (asks a second press when a
saved game would be discarded) and the difficulty; keyboard / gamepad / pointer share `MenuInput`
and `move_cursor` (list or grid). `begin_level` starts a `GameSession` (level, difficulty, carried
lives and score). `Event::LevelComplete` -> `on_level_complete` (unlock + autosave);
Enter / tap then goes to the next level with lives and score carried; game over restarts the same
level with zero score; after the **last** level the confetti runs, Enter opens the question
"Play again / Quit to the title screen". The pause screen is a menu (Resume, Level select, Back to the title screen =
autosave and leave). A failing storage is reported in the status bar, the game goes on.

### Level select (`src/app/level_select.rs`, #12)

`Screen::LevelSelect`, reached from the title screen and from the pause menu (which autosaves
first). A grid of tiles (132 x 116 points, 12 apart; `grid_cols` fits as many per row as the width
allows, a vertical `ScrollArea` holds the rows, the cursor tile is scrolled into view). `tile_info`
(pure, tested) gives each tile its `TileState` - `Locked` (not `SaveData::is_playable`), `Open`,
`Done` (completed on any difficulty) - the best score on the **selected** difficulty and one medal per
difficulty (filled when that difficulty has a best score for the level). Locked tiles are greyed with
a padlock and cannot be selected; the cursor (`MenuInput` + `move_cursor` with the column count),
a click / tap or Enter / A starts the level with the full lives and no score (`begin_level`) - a
replay never lowers the progress (`complete_level` only moves Continue forward). Esc / Start / the
Back button return to the title screen. Level names are read once at start-up (`level_names`).

## Game view (`src/app/game_view.rs`)

The central panel (no frame) is the game. `GameSession` (state, `FixedStep`, `paused`, `latch`)
lives in `App`. Per frame: the view is allocated first, `controls::layout` says where the
playfield and the on-screen controls go, then `handle_pause` (Esc / P toggle, losing the window
focus pauses - `InputState::focused`), the pause button, and, when not paused, `run_steps`:
`FixedStep::advance(unstable_dt)` says how many 60 Hz steps to run and
`ctx.request_repaint()` keeps the game going. A paused game requests no repaint (idle CPU); the
frame time comes from egui, not `std::time`, so the same code works in the browser.

Drawing: the 224 x 256 playfield is scaled by `field_scale` - the largest whole number of
**physical** pixels that fits (crisp pixel art at any DPI), a fraction only when the area is
smaller than the field - centred and snapped to whole physical pixels; the rest is letter-box.

A frame longer than 0.5 s (hidden browser tab, sleep, window drag) pauses the game as well.
After a pause the first frame's time (the whole pause) is dropped (`skip_dt`).

## Input and touch controls (`src/app/controls.rs`, `touch_view.rs`)

Keyboard, touch and (later) gamepad all produce `Buttons` (left / right / up / down / jump):
the **held** set of this frame and the set that **went down** in this frame. The pressed set goes
into `PressLatch`, the held set is passed to `PressLatch::take_step`, which returns the
`game::Input` of the next step: held | latched, `jump_pressed` from the latch. The latch is
consumed by the first step, kept by a frame without a step and cleared on pause. So a key tap
shorter than a frame (down and up inside one frame: `key_pressed` but not `key_down`) still moves
one step - for every direction, not only the jump.

**Touch.** egui's pointer follows one finger only, so `Touches` keeps a map of the raw
`egui::Event::Touch` fingers (start / move insert, end / cancel remove) and `touch_buttons` tests
every finger against the d-pad (3 x 3 grid around the centre with a dead zone, diagonals work,
a finger may slide 50 % outside) and the jump circle: move and jump at the same time work.
`Touches::seen` (any touch event) switches the controls on in the mode "auto"; `Config::touch_controls`
is "auto" / "on" / "off" (View -> Controls); in "on" without a touch screen the mouse acts as one
finger, which makes desktop testing possible. Size (`touch_size` 0.6 - 1.6) and the left-handed
swap are settings too. The smallest hit areas stay above ~9 mm (34 css px); a test checks it.

**Layout** (`controls::layout`, pure and tested): portrait (taller than wide) puts the playfield
on top and the controls in a band below (at most 42 % of the height, the controls shrink to fit);
landscape keeps the whole area for the playfield and draws the controls over the left / right
edges. `index.html` already disables page scroll, pull-to-refresh and (double-tap) zoom.

**Pause button.** Top right of the view, a widget of its own (`Sense::click`). The whole view is
not a pause click any more (every d-pad tap is a pointer click too). While paused a tap anywhere
resumes; after game over / level complete a tap anywhere restarts (touch has no Enter / R).

**Gamepad** (`src/app/gamepad.rs`, #9). Native: `gilrs` (`Gilrs::new()` failing means no gamepad, quietly);
browser: the Gamepad API through `web-sys` with the standard mapping (buttons 0 = A, 9 = Start,
12 - 15 = d-pad, axes 0 / 1 = left stick). Both are poll-only: `Gamepad::poll` runs once per frame and
returns a `PadFrame` (held, pressed edges, Start edge, connected) built by pure functions (`buttons_of`:
d-pad or stick with a 0.5 dead zone, A = jump; `frame_of`: edges against the previous poll, several pads
are combined). It feeds the same `PressLatch` as the keyboard and touch. Start pauses / resumes and, after
the game ended, Start or A restarts. Because a pad cannot wake an idle app, a paused / finished game
requests a repaint after 100 ms while a pad is connected and after 1 s otherwise (to notice a newly
plugged pad; the browser lists a pad only after its first button press): a deliberate small idle cost.
Linux builds need `libudev-dev` for `gilrs` (installed in the CI job).

**Vibration.** `Event::Died` calls `navigator.vibrate(200)` in the browser (`web-sys`
`Navigator`, wasm only) when `Config::vibration` is on; nowhere else.

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
