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
- `Event` is what the front end turns into sound / effects. `src/audio/` has the `Audio` trait
  (see Sound, #16),
  `src/storage.rs` the `Storage` trait (see Saved progress).

### Level format (`level.rs`, `levels.rs`, `levels/*.txt`, #6)

One `key: values` per line, `#` starts a comment. Keys: `name`, `type` (`barrels`, `conveyors`, `lifts`, `rivets`), `bonus` /
`bonus_drop` / `bonus_every` (seconds), `barrel_every` (seconds), `extra_life` (score),
`start x y`, `ape x y`, `goal x y`, `girder x0 y0 x1 y1`, `ladder x top bottom [broken]`,
`rivet x y`, `lift x top bottom up|down`, `conveyor girder left|right seconds` (belt on the girder with that 0-based number, reverses every
`seconds`, 0 = never), `hammer x y`, `bonus_hammer x y` (extra hammers of the easier difficulties; all in playfield pixels; ladder ends are girder surface heights). `Level::parse`
reports syntax / range errors as `line N: message` (`ParseError`, line 0 = whole file, e.g. a
missing key). `Level::validate` (rules in `validate.rs`) checks that start, goal, ape, hammers and ladder ends
stand on girders (within `SNAP`); that a barrel thrown by the ape lands on a girder; that no two
girders cross or come closer than `MIN_CLEARANCE` (20 px); that a ladder joins two different
girders and does not run through a third; and that the goal **and every hammer** are reachable from
the start. The search runs over a directed graph (girders = nodes): intact ladders (both ways),
walking off a girder end onto one at most `WALK_DROP` (31 px) lower, and jumping over a gap of at
most `JUMP_REACH` (≈ 28 px = 80 % of the real jump) - the jump limits are derived from `JUMP_VY`,
`GRAVITY`, `WALK_SPEED`, `FALL_LIMIT`. `Level::to_text` writes the file format back
(`parse(to_text(l)) == l`). `levels.rs` embeds the files with
`include_str!`; the tests parse and validate every embedded file and check that every `levels/*.txt`
is embedded. A new level: add the file and one line in `FILES`.

### Conveyor belts (`Level::belts`, `GameState::belts`, `scene.rs`, #14)

A `Belt { girder, dir, every }` makes a girder carry whatever stands on it. The state that changes
lives in `GameState::belts` (`BeltState { dir, phase, left }`, same index): each step the timer counts
down, the direction flips every `every` steps and `phase` (px travelled, signed) advances by
`dir * BELT_SPEED * obstacle_speed`. `GameState::belt_dir(x, y)` finds the belt under a
point. In `step`: a *walking* player is moved by `BELT_SPEED` (0.5 px, times the difficulty's
obstacle speed) through `Player::carry` (it may be carried off the end and falls; a fall of more
than `FALL_LIMIT` kills as everywhere); a *rolling* barrel gets the belt's direction via
`Barrel::ride` before its step, so the ape's loads travel with the belt and drop off its end.
Climbing and airborne players are not carried. The view draws arrows that move with the belt
(`scene::belt_arrows(x0, x1, phase)`; the accumulated `phase` follows the real speed of every
difficulty and stays continuous when a belt reverses, #30).
The validator ignores belts: the belt is slower than the walk, so every route stays possible.
Level 2 (`levels/02-conveyors.txt`) is five flat belts under the ape; the generator makes the same
structure from a seed (`LevelKind::Conveyors`).

### Lifts and springs (`Level::lifts`, `GameState::lifts`, `barrel.rs`, #14)

`Lift { x, top, bottom, dir }` is a `LIFT_WIDTH` (16 px) platform that rides between the surface
heights `top` and `bottom` (`dir` -1 = up) at `LIFT_SPEED` (0.4 px/step, times the difficulty's
obstacle speed) and starts again at the other end. In `GameState::start` every lift becomes a short
girder appended to `level.geometry.girders`; `step` moves them *before* the player, so the
existing walking / landing / barrel code treats a lift as ground and a player standing on one
rides with it (a lift that wraps takes its ground away: the hero falls). `Geometry::landing` accepts a surface up to `LANDING_SLACK` (1.5 px) above the feet, so a rising lift cannot move past a falling hero between two steps (#33). `GameState::scenery()` is
the geometry without the lifts (the static background texture); the view draws `Id::Lift` at
`GameState::lifts`. The validator joins every girder whose end lies next to a lift's ride (within
a jump) with all the others near it, both ways. On the `lifts` type the ape drops **springs**
instead of barrels: `Barrel::spring` / `BarrelMode::Bouncing` flies right at `SPRING_VX` and
bounces up with `SPRING_BOUNCE_VY` on every landing (below Normal the whole arc runs in slow motion:
`spring_gravity` / `spring_bounce_vy` scale with `obstacle_speed`, so the time the hero has to run under a
spring grows on the easy steps, #40; test `running_under_a_spring_is_easier_on_the_easier_steps`); springs use the barrel code for hits, hammer
smashes and jump-over points. Level 3 (`levels/03-lifts.txt`): start platform, rising lift, upper
floor with a ladder to the ape, falling lift to the right-hand floor. Level 7 (`levels/07-lifts-2.txt`,
written by hand, #29) is a two-stage climb: a lift to a middle floor, a second lift to a ledge on the
right, a ladder to the ape's floor. `levelgen::lifts` picks one of the two layouts by the seed
(`lifts_one_stage` = level 3 with jitter, `lifts_two_stages` = level 7 with jitter).

### Rivets and fireballs (`fireball.rs`, `Level::rivets`, `GameState::rivets`, #14)

On the `rivets` type the level is won by removing every rivet (`rivet: x y` on a girder): a
*walking* hero within 4 px of one pulls it out (`Event::Rivet`, 100 points); the last one adds the
time bonus and completes the level; the goal point is only the friend's place (`step` ignores it
for this type). The validator wants at least one rivet and every rivet reachable. The ape sends
**fireballs** instead of barrels every `barrel_every` (at most `Params::max_fireballs` alive). `Fireball` walks a
girder at 0.45 px/step (times the difficulty's obstacle speed), turns at the ends, every
`Params::fire_turn_every` steps (45 on Normal) turns towards the player with probability 2/3, and takes an intact ladder it passes (50 %) when
that leads towards the player's floor (`target.1` more than 8 px above / below). A touch kills the
hero, the hammer burns it (`Event::Smash`, 500 points); fireballs are cleared on respawn. The view
draws the rivets as dots and the fireballs with the two `Fire` frames. Level 4
(`levels/04-rivets.txt`) has three full floors, the ape's floor and eight rivets; the generator
(`LevelKind::Rivets`) reuses the flat stacked floors and puts rivets near the floor ends.

### Level generator (`levelgen.rs`, #22)

`generate(kind, seed, size_hint) -> Level` (`LevelKind::ALL` lists the types): a candidate is built
from `Rng::new(sub_seed)` with only `+ - * /` (same level on every platform), rounded to 0.1 so
that `to_text` / `parse` give the identical level, then `validate`d; an invalid candidate is
re-rolled with the next sub-seed (at most 64, then the first shipped level). `barrels`: 4-7 floors
(`size_hint` clamped), alternating slopes with an open end for the barrels, 2-3 ladders per floor
pair (the first one always intact), two hammers and two bonus hammers. The generators are
constructive, so the test asserts that the *first* candidate of 300 seeds per type is already
valid. Print a level as a file: `GEN_TYPE=barrels GEN_SEED=7 GEN_SIZE=5 cargo test
print_generated_level -- --ignored --nocapture` (the crate is a binary, so there is no example).

### Difficulty (`difficulty.rs`, #10)

`Difficulty` (Relaxed, Easy, Normal, Hard, Insane; serde name = lowercase) and `Params`, one table
in `Difficulty::params()`: `lives`, `obstacle_speed` (multiplies the barrel speed), `spawn_interval_pct`
(percent of the level's `barrel_every`), `ladder_chance` (`num` in `den`, drawn as
`rng.below(den) < num`, so Normal `1/3` consumes the `Rng` exactly as before), `hammer_delta`,
`score_pct`, `max_fireballs` / `fire_turn_every` (the rivets levels: how many fireballs the ape keeps
alive and how often a walking one may turn towards the hero; Relaxed 2 / 90 steps, Easy 3 / 60,
Normal 4 / 45, Hard 4 / 36, Insane 4 / 30, #36) and `bonus_timeout` (Insane only: when `step()` drops the bonus to 0 the player is killed,
`Event::TimeUp` follows `Died`, the bonus is reset to the level's full value; `GameState::bonus_low()`
makes the HUD blink). `GameState::start(level, seed, difficulty, lives, score)` applies them once, to its own
copy of the level: `barrel_every`, `bonus`, `bonus_drop` and `extra_life` are scaled, hammers are
added from the level's `bonus_hammer` lines (Relaxed +2, Easy +1; as many as the level has) or the
last ones removed (Insane -1, never the last one); `add_score` scales jump / smash points. `lives` /
`score` carry a game from one level to the next (`None` = full lives of the difficulty); a carried
score that already earned the extra life does not earn it again. Geometry is never touched. Normal
is bit-identical to the pre-#10 game (tests). The difficulty of the next level that starts is
`Config::difficulty` (menu Game -> Difficulty); the running level keeps its numbers.
The numbers are a first proposal, not tuned by playing yet; the balance bots (#34) measure them.

### Balance bots (`balance.rs`, test only, #34)

Two bots play every built-in level on every difficulty through the real `step` and `Input`;
`cargo test --release balance -- --ignored --nocapture` (`BALANCE_SEEDS=8` for more runs, default 4;
about 3 minutes for 8 in release on two cores). Every `SLOT` (4) steps a bot tries each of its 9 moves on a copy of the
game: the queued moves of its reaction time, the move, then a second move held for 4–32 steps or the
whole look-ahead and then standing still; a jump still in the air at the end is played to its
landing. The copy gets a fresh `Rng` seed, so the bot cannot know the barrels' ladder choices. A
plan's value is minus the discounted mean of the distance to the goal along the rollout (a death or
a win ends it); the distance comes from `Nav`, a Dijkstra graph over the fixed girders (both ends of
every intact ladder, walk / jump edges by the validator's rules, lift stops at a jump's take-off
point, the goal or every rivet; on the rivets level 1000 per rivet left plus the way to the nearest).
`WEAK`: reaction 4 slots (0.27 s), 15 % chance per slot of reacting one more slot late, 24-step
look-ahead, one guess. `STRONG`: instant, 40 steps, worst of two guesses. A run is one level from the
full lives of the difficulty to the win, the game over or 3 minutes. The tests:
`balance_bots_finish_every_empty_level` (no hazards: both bots must win every level, so a stuck bot
is a bot bug, not a balance result) and `balance_difficulty_steps` (prints the tables; fails when the
weak bot wins no run of a level on Relaxed or the strong bot none on Insane).

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
autosave and leave, Exit = autosave and close the window; Exit is also an item of the title screen;
both are desktop-only, a web page cannot be closed by the program). A failing storage is reported in the status bar, the game goes on.

### Level select (`src/app/level_select.rs`, #12)

`Screen::LevelSelect`, reached from the title screen and from the pause menu (which autosaves
first). A grid of tiles (132 x 116 points, 12 apart; `grid_cols` fits as many per row as the width
allows, a vertical `ScrollArea` holds the rows, the cursor tile is scrolled into view). `tile_info`
(pure, tested) gives each tile its `TileState` - `Locked` (not `SaveData::is_playable`), `Open`,
`Done` (completed on any difficulty) - the best score on the **selected** difficulty and one medal per
difficulty (filled when that difficulty has a best score for the level). A legend (`draw_legend`) under the grid names the colours and every circle has a hover tooltip. Locked tiles are greyed with
a padlock and cannot be selected; the cursor (`MenuInput` + `move_cursor` with the column count),
a click / tap or Enter / A starts the level with the full lives and no score (`begin_level`) - a
replay never lowers the progress (`complete_level` only moves Continue forward). Esc / Start / the
Back button return to the title screen. Level names are read once at start-up (`level_names`).

### High scores (`save.rs`, `name_entry.rs`, `high_scores.rs`, #13)

Per difficulty the save keeps the top ten (`Progress::high_scores`, best first). `Progress::qualifies`
(score > 0 and a free place or better than the last) and `SaveData::add_high_score` (insert at the
first place with a lower score, so an earlier entry wins a tie; truncate to ten) are pure and tested;
a loaded table is sorted, cut to ten and its names limited to 12 characters.

A run ends with a score when all lives are lost (game over) or the last level is completed:
`finish_run` asks for a name when the score qualifies, otherwise goes straight on. **Name entry**
(`NameEntry`, pure state machine, arcade style, `NAME_LEN` = 3 letters A-Z - decided with the owner):
up / down change the letter under the cursor (wrapping), left / right move it, typing a letter sets
it and moves on (W / A / S / D are letters here, so only the arrow keys navigate), Backspace goes
back, Enter / gamepad A go to the next place and finish after the last one; for touch every place has
up / down buttons and there is an OK button; the touch d-pad / jump button also drive it like the
gamepad (`touch_pressed`), and the screen shrinks (`entry_scale`) to fit the small field of a phone so
the OK button is never cut off (#27, #28). It starts with the last name (`Config::last_name`).
After it the score is stored and autosaved and the run goes on (restart the level after a game over,
the "Play again / Quit" question after the last level). While a name is entered the game keys
(restart, pause) are off. A menu (name entry, victory question) that opens in a frame never sees the
key / tap that opened it (`entry_open` / `victory_open` are read before `after_level`).
The high-score screen (title screen item) shows the table of one difficulty with tabs (left / right).

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

### Installable web app (`web/`, `index.html`, `src/app/pwa.rs`, #18)
- **Files.** `index.html` links the manifest, the icons and `apple-touch-icon` and lets Trunk copy
  `web/manifest.webmanifest`, `web/sw.js` and the icon PNGs to the root of `dist/` (`data-trunk rel="copy-file"`,
  no hash in the names). Every URL is relative (`start_url: "./"`, `register('./sw.js')`), so the same files work
  under `/MMDonkeyKong/` on Pages and under `/` on a local server. The manifest asks for `display: fullscreen`
  with a `standalone` fallback (iOS ignores fullscreen).
- **Icons** are drawn in `src/icon.rs` (`rgba` = rounded square on transparent, `rgba_maskable` = opaque full-bleed,
  picture shrunk into the safe circle; also used for `apple-touch-icon.png`). The PNGs in `web/` are committed;
  the test `icon::tests::web_icons_are_current` fails when they differ from the code, and
  `UPDATE_ICONS=1 cargo test web_icons` rewrites them (`png` is a dev-dependency).
- **Service worker (`web/sw.js`), cache-first.** On install it fetches `index.html`, picks every `.js` / `.wasm` /
  `.css` it names (Trunk puts the hashed file names there) plus the manifest and icons, and stores them in the
  cache `mmdonkeykong-<BUILD>`. Every GET of the same origin is answered from that cache (a page load always gets
  the cached `index.html`), so the game starts with no network and instantly on a weak one; a miss goes to the
  network and is cached.
- **Updates.** `__BUILD__` in `sw.js` is replaced with the commit hash by `pages.yml` after the Trunk build, so
  every deployment has new bytes: the browser installs the new worker next to the old one (the old one keeps
  serving). `index.html` then sets `window.mmUpdateReady` and fires the event `mm-update-ready`; `pwa::UpdateWatch`
  (wasm only) listens (one repaint, no polling) and `App::menu_bar` shows the green button *Update available -
  reload* (`update.available` / `update.hover`). A click calls `window.mmApplyUpdate()`: it posts `skip-waiting`
  to the waiting worker, waits for `controllerchange` and reloads; `activate` deletes the old caches. An open
  app also checks for a new build when it becomes visible again.
- **`trunk serve`** (port 8080) never registers the worker, its live reload would fight the cache. To try the
  worker locally: `trunk build --release --public-url /`, replace `__BUILD__` in `dist/sw.js` by hand, serve `dist`
  on another port (`python -m http.server 8081`); change the stamp and restart the server to see the hint.

### Sound (`src/audio/`, #16)
- `synth.rs`: pure rendering of `Tone`s (square / triangle / noise, linear frequency slide,
  2 ms attack + decay) into mono `f32` buffers at 22 050 Hz. `effect(Event)` and `music(level)` are
  deterministic and unit-tested (range, length, seamless, every level differs). `music(level)` (#38)
  generates a 4-bar lead + bass loop from the level number only: a seeded xorshift picks root,
  scale (minor / dorian / phrygian / harmonic minor), chord progression, bass pattern, tempo
  (110-150 ms per eighth) and lead wave; the lead is chord tones on strong beats plus a random
  walk, starts on the tonic and ends a step beside it. Any level number (also generated levels)
  gets a tune.
- `mixer.rs`: `Library` renders every effect once and the music of a level on demand (`Arc<[f32]>`); `Mixer` adds voices (max 8
  effects, the oldest is dropped; one looping music voice) and clips to -1..1.
- `win.rs` (Windows): a thread owns a `waveOut` device (winmm declared directly, `// SAFETY:`
  comments), four 512-sample buffers, commands arrive through an `mpsc` channel; while nothing plays
  and all buffers are done it blocks on the channel (no idle CPU). Dropping `WinAudio` ends it. No sound
  device → the thread ends and the game stays silent. Manual test: `cargo test -- --ignored plays_on`.
- `web.rs` (wasm): Web Audio; the `AudioContext` is created on the first sound and `resume()`d on
  every play (browsers need a user gesture first). Other systems use `NullAudio`.
- Front end: `App::sync_audio` (every frame) gives the volume (`Config::muted` / `volume`) and the
  music (`Some(level)` while a game runs, not paused, `Config::music`; `None` otherwise) to the
  backend only when they change, so a new level replaces the loop; `run_steps` plays every `Event`.

### Settings window (#17)
`dialogs/settings.rs`: sound (mute, volume, music), display scale (the UI zoom), theme, language,
difficulty and the controls (`controls_widgets`, shared with View → Controls). Opened from
View → Settings… and the title screen item. Like About it is a tool window (#32, below).

### Tool windows (`src/app/native_window.rs`, #32)
Taken from MM PortLens. `ToolWindow` (open / focus / geometry it opened with) + `ToolSpec` (title,
default and minimum size). `ToolWindow::show` draws a bottom bar (A− / zoom % / A+ / Close) and the
content in a central panel, either as an OS window (`ctx.show_viewport_immediate`) when
`Config::separate_windows` (View → Open windows separately, default on) is set and
`!ctx.embed_viewports()` (always false in the browser), or as a resizable `egui::Window` inside the
main window. The text zoom (Ctrl + wheel / `+` / `-` / `0`) scales the window's text styles only,
because `set_zoom_factor` is shared by all viewports. Position (native), size and zoom live in
`Config::about_window` / `settings_window` (`ToolWindowState`, written back by
`App::store_window_state`); the native window is built from the state once when it opens (a builder
changing between frames moves the window). Esc, ×, Close close it. The Settings content needs
`&mut self`, so `settings_window` takes its `ToolWindow` out of `App` for the call. A new window:
a `ToolWindow` field in `App`, a `ToolWindowState` in `Config`, a `dialogs/<name>.rs`.
