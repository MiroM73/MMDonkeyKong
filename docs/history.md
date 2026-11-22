# History

What was done when (one line per issue / release) and the open ideas of the owner.

[← Back to README](../README.md)

## Log

- Project created from the MMDonkeyKong (skeleton: window, menu, theme, languages, settings, About,
  CI and release scripts).
- 2026-11-20: convention added – every separately opened window must be movable outside the main
  window (own OS window with in-window fallback, MM PortLens `native_window.rs`); `CLAUDE.md`,
  `TEMPLATE.md`, `docs/reuse.md`. RusTrace migration: issues #68, #69.

- 2026-11-21: game specified – `docs/game-design.md` (mechanics, 4 level types, 5 difficulty steps,
  saves, level select, controls), `docs/platforms.md` (one Rust code base → wasm for the browser on
  GitHub Pages + PWA offline + Windows exe), roadmap in 8 phases, issue drafts 01–20 in
  `docs/issues/`. Nothing implemented yet.
- 2026-11-21: owner answered the design questions (recorded in `docs/game-design.md`): saves per
  difficulty with shared unlocking, local records, level generator, save export / import, optional
  cloud sync, deploy on every push, Windows-only exe. Issues 21-25 added to the roadmap.
- 2026-11-21: GitHub repository `MiroM73/MMDonkeyKong` created (public, `origin` added, nothing
  pushed yet) and issues #1-#25 created from `docs/issues/`.
- 2026-11-21: #2 game core skeleton: `src/game/` (state, `step`, `Input`, `Event`, seeded `Rng`,
  `FixedStep`), `Storage` and `Audio` traits with null implementations.
- 2026-11-21: #1 game window: the central panel is the game (224 x 256 playfield, integer scaling in
  physical pixels, 60 Hz fixed step from the frame time, keyboard, pause on Esc / P / click / lost
  focus, no repaint while paused). Placeholder marker moves with the arrow keys.
- 2026-11-21: #3 browser build: wasm32 target with Trunk (`index.html`, `Trunk.toml`), `main.rs`
  split into desktop / wasm entry points (`native.rs`), desktop-only code behind `cfg`, frame time from
  egui instead of `Instant`.
- 2026-11-21: #4 GitHub Pages: `pages.yml` deploys the Trunk build on every push to `master`,
  CI job `web` (clippy wasm32 + Trunk build), README play link and badge.
- 2026-11-21: #5 Player physics in `src/game/player.rs`: slopes, ladders, fixed-arc jump, swept landing,
  fall limit, forgiving hitbox, death and respawn; the view draws a demo level with the player.
- 2026-11-21: #6 Level format (`levels/*.txt`, `src/game/level.rs`: parser with line-numbered errors +
  validator with a goal-reachability search), the first level "Barrels", rolling barrels
  (`barrel.rs`), hammer, jump-over / smash score, time bonus, extra life, lives, level complete /
  game over (Enter / R restarts); the view draws the level, barrels, hammers, ape, goal and a HUD.
- 2026-11-21: #7 Pixel-art rendering in code (`src/app/sprites.rs`, `scene.rs`): hero (stand, 2 walk
  frames, jump, climb, hammer, dead), ape (idle, chest beat, throw), friend, rolling / climbing
  barrels, hammer, smash explosion, fireball frames (unused until #14); girders and ladders are
  rasterised once per level into a texture. The hero is teal, not red / blue, to stay original.
- 2026-11-21: #8 Controls: direction presses are latched like the jump (a tap shorter than a frame still
  moves), multi-touch on-screen d-pad + jump button (`controls.rs`, `touch_view.rs`), portrait / landscape
  layout, pause button instead of click-to-pause, tap to resume / restart, View -> Controls (mode, size,
  left-handed, vibration), vibration on death in the browser.
- 2026-11-21: #9 Gamepad: `gilrs` on the desktop, the Gamepad API in the browser; stick / d-pad move and
  climb, A jumps, Start pauses / restarts; idle polling while a pad is connected; CI installs libudev-dev.
- 2026-11-22: touch assist: with the on-screen controls a sideways jump grabs a ladder it passes without "up"; a sideways jump from a ladder jumps off in that direction (owner's request after phone testing).
- 2026-11-22: #10 Five difficulty steps: `Difficulty` + one `Params` table (lives, barrel speed, spawn
  interval, ladder-descent chance, hammer count, score multiplier) applied by `GameState::start`;
  `bonus_hammer` lines in level files for the extra hammers of the easier steps; menu Game -> Difficulty
  (`Config::difficulty`), name in the HUD. Normal is bit-identical to the old game. Balance numbers are
  the first proposal of `docs/game-design.md` and are **not tuned by playing yet**.
- 2026-11-22: #11 Saved progress: `FileStorage` (atomic write) / `WebStorage` (`localStorage`), versioned
  `SaveData` (`src/save.rs`: completed levels shared, per difficulty continue point, best scores, high-score
  list), damaged / too-new save kept as `save.bak`, autosave after every completed level and when a level
  is left through the menu, title screen with Continue / New game / difficulty, pause menu, next level with
  lives and score carried, game over restarts the level at score 0. Owner's decisions: after the last
  level a victory animation (confetti) and a "Play again / Quit to the title screen" question; Continue
  only moves forward. The browser settings (`Config`) now persist in `localStorage` too.
- 2026-11-22: #12 Level select: grid of tiles (number, name, best score of the selected difficulty, one
  medal per difficulty), completed levels and the next one playable, later ones greyed out with a padlock;
  from the title screen and the pause menu; keyboard / gamepad cursor, large tiles for touch, scrolling
  for many levels. Replaying never lowers the progress. Checked with temporary extra levels (only one level
  exists until #14).
- 2026-11-22: #13 High scores: top ten per difficulty in the save, 3-letter arcade name entry (owner's
  decision; keyboard typing, arrows, gamepad, touch buttons, remembers the last name), asked after a game over
  and after the last level, high-score screen with a tab per difficulty from the title screen. Fixed on the
  way: a menu that opened with Enter / a tap reacted to the same key / tap at once (the victory question
  chose "Play again" immediately).
- 2026-11-22: #22 Level validator and generator: `validate.rs` now also checks that every hammer is reachable,
  walk-off and jump moves between girders (limits derived from the physics constants), girder clearance /
  crossings, ladders through girders and a landing spot for the ape's barrels; `Level::to_text`;
  `levelgen.rs` builds barrels levels from a seed (300 seeds per type validate on the first try, same seed =
  same level, text round-trips). Print one with the ignored test `print_generated_level`.
- 2026-11-22: #14 (part 1) Level 2 "Conveyors": `conveyor:` lines in the level format, belts that carry the
  walking hero and the ape's loads and reverse on a timer, moving arrows in the view, generator support
  (`LevelKind::Conveyors`). Checked in the running exe (level select, belts moving, load on a belt).
- 2026-11-22: #14 (part 2) Level 3 "Lifts": `lift:` lines, lifts as moving girders (the player and the
  barrels reuse the ground code), springs that bounce along the floors (`BarrelMode::Bouncing`), lift /
  spring sprites, generator support. Checked in the running exe (lifts moving, spring, hammers).
- 2026-11-22: #14 (part 3) Level 4 "Rivets": `rivet:` lines, rivets removed by walking over them (all of
  them win the level), fireballs that hunt the hero over floors and ladders (`fireball.rs`), hammer burns
  them, generator support. Checked in the running exe (rivets, fireballs, ladders). #14 is done.
- 2026-11-22: #15 (part 1) Levels 5-8 "Barrels II", "Conveyors II", "Lifts II", "Rivets II": generator
  output saved as files (seed and size in the file header) and tuned by hand: seven floors, faster
  barrels / loads / springs, more rivets, bigger bonus and extra-life scores. All eight levels appear in
  the level select and start in the running exe. **Not done yet:** the balance pass. Nobody has played
  levels 2-8 on the five difficulty steps, so `Difficulty` and the level timings are untouched; the only
  automated check is a smoke test (every level on every difficulty runs deterministically, keeps its
  hammers and a positive bonus). Level 7 is close to level 3 (the generator has one lifts layout).

- **#16 Sound** – synthesised effects + music loop (`src/audio/`), `waveOut` backend on Windows,
  Web Audio in the browser, silent elsewhere; mute / volume / music in `Config`.
- **#17 Settings** – Settings window (View → Settings…, title screen item): sound, display scale,
  theme, language, difficulty, controls; About shows credits; new keys in all four languages.
- **#33 Lift landing** – a falling hero no longer slips through a rising lift (the landing test now has
  a small slack); the fall distance rule (32 px, as in the original) is unchanged.
- **#34 Balance bots** – `src/game/balance.rs` (tests only, ignored, `--release`): a weak and a strong
  bot play every level on every difficulty and print won runs / deaths per run. First result (8 runs,
  table unchanged since #10): the weak bot wins every level on Relaxed (Rivets 7/8), the strong bot wins
  every level on Insane with 7/8 or 8/8 runs and under one death per run. So both goals are met, but
  Insane is too lenient for a very good player (→ #35, time limit). Conveyors levels are almost
  harmless on every step, Rivets is a difficulty spike, and Normal and Hard are hard to tell apart (→ #36).
- **#35 Insane time limit** – on Insane the hero loses a life when the time bonus reaches 0 (`Params::bonus_timeout`,
  `Event::TimeUp` with its own sound); the bonus restarts from its full value; the HUD bonus blinks red
  during the last five drops; Hard keeps the old behaviour (open: decide after playing). The "waiting" check of the balance bots
  (strong bot's longest Insane run under the bonus time) is still open.
- **#27, #28 Touch name entry** – the name-entry screen is scaled to the (small) game field of a phone so the
  OK button is no longer cut off, and the touch d-pad / jump button drive it like the gamepad.
- **#26 Seed from the clock** – `progress::clock_seed` mixes the wall clock (`SystemTime` / `Date.now()` in the
  browser) into every game's seed, so the first game differs on every launch.
- **#30 Belt arrows** – `BeltState::phase` accumulates the real belt travel; arrows follow the difficulty speed
  and do not jump on reversal.
- **#37 Insane vs. the bonus time** – `balance_difficulty_steps` now records the longest winning life of the strong
  bot on Insane (`Outcome::life_steps`, steps since the last death) and asserts it stays under 60 % of the level's
  bonus time (`bonus_steps`); it prints the share per level. Measured (8 seeds): 1–13 % (Rivets II 32.5 s of 240 s
  is the longest), so no `bonus` had to change. `balance_explore` (`BALANCE_LEVELS=2,4 cargo test --release
  balance_explore -- --ignored --nocapture`) runs chosen levels only.
- **#36 Balance findings (partly)** – fireballs scale with the difficulty (`Params::max_fireballs`,
  `fire_turn_every`); Conveyors / Conveyors II drop a load every 2.2 s / 1.5 s (was 3.5 / 2.5); Rivets / Rivets II
  a fireball every 7 s / 4.5 s (was 6 / 4). Table after the change (weak bot, 8 runs, won runs): Conveyors Normal 7/8
  with 0.4 deaths (was 0.1), Rivets Easy 8/8, Hard 2/8, Insane 0/8; the strong bot still wins every level on Insane
  (Rivets 5/8). Rivets remains the hardest level for the weak bot – its deaths come from the long route and the
  hunting, not from the number of fireballs – and Hard differs from Normal only a little more (→ final pass #15).
- **#29 Lifts II** – level 7 is a hand-written two-stage climb (two rising lifts, a middle floor, a ledge, a
  ladder); the generator got the same layout as a second `lifts` variant chosen by the seed.
- **#31 Platform verification** – CI of `3ca47d9` is green on all three jobs (Linux clippy + tests, wasm32 clippy +
  Trunk build, Windows). The deployed Pages build was opened in Chrome with a save that unlocks all levels: levels 2-8
  (conveyors, lifts, rivets, their second variants) load and render, belts / lifts / fireballs are drawn, pause and
  level select work, no console errors. Not verified: on-screen touch controls (no touch device available, the
  buttons do not show on a desktop browser) and a byte-for-byte comparison of `levelgen` output between wasm and
  native - to be checked on a phone (#20).
- **#18 PWA** – manifest, new game icon (ape, girder, ladder, barrel; also the window / exe icon), maskable and
  `apple-touch-icon` PNGs generated from `src/icon.rs`, cache-first service worker stamped with the commit by
  `pages.yml`, *Update available - reload* button in the menu bar (`src/app/pwa.rs`). Verified in Chrome on a local
  server: the worker controls the page, the cache holds index / js / wasm / manifest / icons, with the server
  stopped the reload shows the title screen, after a new stamp the hint appears and one click loads the new worker
  and deletes the old cache. Not verified: adding to the home screen on Android / iOS (→ #20). The release web zip
  must stamp `sw.js` too (→ #19).

## Open ideas

- _(none yet)_
