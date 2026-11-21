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

## Difficulty (5 steps)

Chosen in the new-game screen and changeable in the settings between levels:

| Step | Name | Lives | Obstacle speed | Spawn interval | Ladder-descent chance | Hammer count | Score multiplier |
|------|------|-------|----------------|----------------|----------------------|--------------|------------------|
| 1 | Relaxed | 7 | 0.60× | 1.6× | low | extra | 0.5× |
| 2 | Easy | 5 | 0.80× | 1.3× | lower | +1 | 0.75× |
| 3 | Normal | 3 | 1.00× | 1.0× | normal | normal | 1.0× |
| 4 | Hard | 3 | 1.20× | 0.8× | higher | normal | 1.5× |
| 5 | Insane | 2 | 1.45× | 0.65× | high | −1 | 2.0× |

The numbers are a first proposal (a `Difficulty` struct in code, one table, tuned by playing).
Difficulty only scales parameters; it never changes level geometry. High scores are kept per
difficulty.

## Progress, saving and level selection

- **Autosave after every completed level** (and when the player leaves a level through the menu):
  difficulty, highest reached level, score, lives, high scores, settings.
- The **level-select screen** lists all levels the player has already **completed plus the next one**;
  locked levels are shown greyed out. A level can be replayed at any time; replaying does not lose
  the progress of the highest level.
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
| Gamepad (later) | stick / d-pad | stick / d-pad | A | automatic | Start |

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

## Open questions for the owner

See the list in the chat that came with this document; answers are recorded here (then removed
from the list):

- _(none answered yet)_
