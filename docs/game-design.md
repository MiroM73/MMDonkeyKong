# Game design

What the game is and how it should behave. This is the specification the implementation follows;
change it together with the code. Open questions for the owner are at the end.

[← Back to README](../README.md)

## Concept

A single-player arcade platformer in the spirit of the 1981 *Donkey Kong* cabinet: the player
climbs ladders and platforms towards the top of a screen while an ape throws obstacles down. The
game is **not** a copy: all names, characters, graphics, sounds and level layouts are original
(the *Donkey Kong* name, Mario, Pauline and the original sprites / music are Nintendo's trademarks
and copyrights – see "Legal" below). Working names: hero **Jumper**, ape **Kong-o-Tron** (to be
decided with the owner).

Runs in a web browser (PC, tablet, phone) and as an offline Windows exe – one code base, see
`docs/platforms.md`.

## Core mechanics

- Screen = one fixed-size playfield (logical resolution 224 × 256, scaled to any window / screen,
  integer scaling when possible, letter-boxed), pixel-art look drawn in code (no image files at
  first; sprites are small pixel arrays drawn with the egui painter).
- The player **walks** left / right on platforms (slightly sloped girders), **climbs** ladders up /
  down, **jumps** (fixed arc, no air control, like the original) and can grab a **hammer** for a few
  seconds to smash obstacles.
- Touching an obstacle or falling from too high costs a life; reaching the goal (top) completes the
  level.
- Obstacles: rolling barrels (random ladder descent), fireballs (follow ladders), conveyor-belt
  loads, springs, depending on the level.
- Scoring: jumping over an obstacle, smashing with the hammer, bonus items, a **time bonus** that
  counts down from the start of the level and is added on completion; extra life at fixed scores.
- Game state is simulated with a **fixed time step (60 Hz)**, independent of the frame rate, so the
  game plays the same on a 30 Hz phone and a 144 Hz monitor, and the simulation is deterministic
  (seeded RNG) and unit-testable.

## Levels

Four level types, repeating with a rising difficulty (as in the original cycle), each with a few
variants – **at least 8 levels** for the first release:

| Type | Idea |
|------|------|
| Barrels | sloped girders, ladders, barrels rolling down, hammers |
| Conveyors | conveyor belts that change direction, loads dropping off, moving platforms |
| Lifts | rising / falling lifts, bouncing springs |
| Rivets | remove all rivets from the structure while fireballs chase the player |

Levels are **data**, not code: a small text format (platforms, ladders, hazards, spawn points,
timings) in `levels/*.txt`, embedded with `include_str!` and parsed by a pure, tested function. A
level file has a name, a type and a "difficulty base"; the difficulty setting scales it (see below).
Adding a level must not require touching the game loop.

### Level generation and creation

Creating new levels has to be cheap, so levels are produced in three ways that share one format:

1. **Hand-written** `levels/*.txt` – readable `key: values` lines (girders and ladders as
   coordinates, because the girders are sloped; hammers, goal, ape, start, time bonus, spawn
   timings; see `docs/architecture.md` → Level format). Easy to edit in any text editor and to diff in git.
2. **Generated** – a pure function `generate(type, seed, size_hint) -> Level` (module
   `game::levelgen`) builds a valid level from a seed: stacked sloped girders with alternating
   direction, ladders placed so that every girder is reachable, hammers and hazards by the level
   type. Every generated level is run through the **validator** (reachability of the goal by a
   graph search over girders and ladders, no ladder into a wall, jump gaps within the jump arc,
   spawn points on girders) and rejected / re-rolled when invalid. The first-release levels may be
   hand-tuned outputs of the generator saved as files (seed in the header).
3. **In-game editor** (later) – paint the ASCII grid on screen, test play, export / import as
   text (same format, so a level can be pasted between phone and PC).

The format, parser, validator and generator are pure code with unit tests (every shipped level
and a few hundred generated seeds must validate).

## Difficulty (5 steps)

Chosen in the new-game screen and changeable in the settings between levels:

| Step | Name | Lives | Obstacle speed | Spawn interval | Ladder-descent chance | Hammer count | Score multiplier |
|------|------|-------|----------------|----------------|----------------------|--------------|------------------|
| 1 | Relaxed | 7 | 0.60× | 1.6× | low | extra | 0.5× |
| 2 | Easy | 5 | 0.80× | 1.3× | lower | +1 | 0.75× |
| 3 | Normal | 3 | 1.00× | 1.0× | normal | normal | 1.0× |
| 4 | Hard | 3 | 1.20× | 0.8× | higher | normal | 1.5× |
| 5 | Insane | 2 | 1.45× | 0.65× | high | −1 | 2.0× |

Rivets levels also scale the fireballs: at most 2 / 3 / 4 / 4 / 4 alive and a turn towards the hero every 90 / 60 / 45 / 36 / 30 steps (Relaxed … Insane).

The numbers are a first proposal (a `Difficulty` struct in code, one table, tuned by playing). Hammer count: the level file lists its own `hammer` lines and optional `bonus_hammer` lines; "extra" = +2, "+1" = +1 of those (as many as the level has), "−1" removes one hammer (never the last). The extra-life score and the time bonus scale with the score multiplier.
**Time limit (Insane only):** when the time bonus reaches 0 the player loses a life (as in the original)
and the bonus starts again from its full value; the bonus blinks red during its last ~10 seconds.
On every other step the bonus just stays at 0. Every level gives at least 100 s (5000 / 100 per 2 s).
Difficulty only scales parameters; it never changes level geometry. High scores are kept per
difficulty.

Goal of the table: a weak player can finish every level on Relaxed, a very good player every level on
Insane. The balance bots check it (`docs/architecture.md` -> Balance bots, #34).

## Progress, saving and level selection

- **Autosave after every completed level** (and when the player leaves a level through the menu):
  per difficulty the score, lives and best scores; shared across difficulties the set of
  completed levels; plus high scores and settings.
- The **level-select screen** lists all levels the player has already **completed plus the next one**;
  locked levels are shown greyed out. A level can be replayed at any time; replaying does not lose
  the progress of the highest level.
- *Continue* only ever moves forward (replaying an earlier level never lowers it); after the last level
  the game is finished: victory animation, then the question "Play again / Quit to the title screen".
- *Continue* resumes at the last saved level start (mid-level saving is not planned: the simulation
  is real time, so "save at the start of a level" is the checkpoint).
- Storage is behind one small trait (`Storage`): a JSON file in `%APPDATA%\MMDonkeyKong\` on
  desktop, `localStorage` in the browser. Saves carry a `version` field and are migrated, a damaged
  save is ignored (never crashes) and kept as `.bak`. Export / import of the save as text is a
  later nice-to-have (moving progress between phone and PC).

## Controls

| Device | Move | Climb | Jump | Hammer | Pause |
|--------|------|-------|------|--------|-------|
| Keyboard | ← → / A D | ↑ ↓ / W S | Space | automatic on pickup | Esc / P |
| Touch (phone, tablet) | on-screen d-pad (left side) | d-pad up / down | big button (right side) | automatic | pause icon |
| Gamepad | stick / d-pad | stick / d-pad | A | automatic | Start |

The touch controls are drawn over / beside the playfield, sized for thumbs, multi-touch (move and
jump at the same time), and shown only when a touch screen is detected (or forced in the settings).
In portrait orientation the playfield sits at the top and the controls below it; in landscape the
controls overlay the left / right edges. The page prevents scrolling, pinch zoom and the
double-tap zoom while playing.

## Audio

Generated in code (square / noise waves – chip-tune style), no audio files: jump, hit, barrel
smash, level done, game over, a simple background loop. Mute and volume in the settings; the
browser requires a user gesture before audio starts (the first tap / key press unlocks it).

## User interface

Screens: title → new game / continue → difficulty → level select → game → pause / level complete /
game over → high scores; settings (language, theme, controls, sound, difficulty, display scale).
Menu text goes through `t!()` (English, Slovak, Czech, German as in the template).

## Legal

Original names, art and sounds only. No ripped sprites, no original music, no "Donkey Kong" /
"Mario" naming, to avoid trademark problems – especially because the game is published on GitHub
Pages. The README explains that it is a homage to the arcade genre.

## Decisions of the owner (2026-11-21)

- Original characters: hero **Jumper**, ape **Kong-o-Tron** (working names, may change).
- Graphics: retro pixel art drawn in code, no image files.
- Audio: chip-tune generated in code, music and effects.
- Progress / saves are kept **per difficulty** (continue state, scores, high scores). A level
  completed on any difficulty is **unlocked for every other difficulty** (unlocking is shared,
  scores are not).
- Game over (all lives lost): the player restarts at the **beginning of the current level with the
  score reset to zero** (no return to level 1).
- 8 levels for the first release; levels must be easy to **generate and create** (see "Level
  generation" above).
- Touch: virtual d-pad + jump button only (no swipe gestures). Gamepad optional.
- Languages: all four (en, sk, cs, de).
- Records: local only (no online leaderboard). High-score names are three arcade-style letters.
- Save transfer between phone and PC: text **export / import** first; optional **cloud-folder
  sync** later (see `docs/platforms.md` -> Saves across platforms).
- Web deployment on every push to `master`.
- Release exe: Windows only; the README describes how to build on Linux and macOS.
