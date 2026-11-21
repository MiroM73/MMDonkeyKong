# Roadmap

The order in which the GitHub issues are solved, agreed with the owner. The owner refers to the
phases by number ("fáza 1" = phase 1). Keep the status column up to date: mark an issue done when
its commit lands, add new issues to the phase where they fit. When every issue of a phase is done,
add "(done)" to the phase heading, and once a release ships it, name that release there
("(done, released in vX.Y.Z)"; "(done, not released yet)" until then).

Every issue follows the procedure of `CLAUDE.md`: take the code from the sibling projects where it
exists (`docs/reuse.md`), `t!` strings in all four languages, `Config` defaults, fmt + clippy +
tests, check in the running program, README / `docs/architecture.md` / `docs/history.md`, detailed
commit with `Closes #N` (or `Part of #N`). Nothing is pushed or released without asking.

What the game is: `docs/game-design.md`. How it is built and published: `docs/platforms.md`.
The issue drafts are in `docs/issues/` (`scripts\create-issues.ps1`); numbers below are the draft
numbers (`NN-*.md`) until the issues exist on GitHub, then the GitHub numbers are added.

Principle: **playable as early as possible** – a walking, jumping hero on one level (phase 3) comes
before menus, saves and polish, and the web build (phase 2) exists from the start so that touch
and browser problems are found early, not at the end.

## Phase 0 – decisions
| Issue | What | Status |
|-------|------|--------|
| – | Owner answers the open questions of `docs/game-design.md` (name, art, saves, …) | open |

## Phase 1 – foundation
| Issue | What | Status |
|-------|------|--------|
| 01 | Turn the template into a game window: playfield with logical resolution, scaling, fixed-step loop, pause | open |
| 02 | Platform-independent `game` module skeleton + `Input` / `Storage` / `Audio` traits | open |

## Phase 2 – browser build (early, so every later step is tested there too)
| Issue | What | Status |
|-------|------|--------|
| 03 | wasm32 build with Trunk, `cfg`-gate native-only code, local `trunk serve` | open |
| 04 | GitHub Pages deployment workflow + CI `wasm` job | open |

## Phase 3 – first playable level
| Issue | What | Status |
|-------|------|--------|
| 05 | Player physics: platforms, slopes, ladders, jump, death, tests | open |
| 06 | Level format + parser + first level "Barrels" (barrels, hammer, goal, score, time bonus) | open |
| 07 | Pixel-art rendering in code (hero, ape, barrels, girders, ladders) | open |

## Phase 4 – input on every device
| Issue | What | Status |
|-------|------|--------|
| 08 | Keyboard + touch controls (multi-touch d-pad and jump), portrait / landscape layouts, no page zoom / scroll | open |
| 09 | Gamepad support (optional) | open |

## Phase 5 – difficulty, progress, level select
| Issue | What | Status |
|-------|------|--------|
| 10 | Five difficulty steps (`Difficulty` table: speed / spawn / lives / score) | open |
| 11 | `Storage` backends (file, localStorage), versioned save, autosave after each level, Continue | open |
| 12 | Level-select screen: completed levels + the next one, locked levels greyed out | open |
| 13 | High scores per difficulty | open |

## Phase 6 – more content
| Issue | What | Status |
|-------|------|--------|
| 14 | Levels 2–4: Conveyors, Lifts, Rivets (new hazards, level files) | open |
| 15 | Levels 5–8: harder variants; balance pass over all five difficulties | open |

## Phase 7 – sound and polish
| Issue | What | Status |
|-------|------|--------|
| 16 | Synthesised sound effects and music, native + Web Audio backends, mute / volume | open |
| 17 | Menus, title screen, settings, translations (en / sk / cs / de), About / credits | open |

## Phase 8 – offline and release
| Issue | What | Status |
|-------|------|--------|
| 18 | PWA: manifest, icons, service worker, "add to home screen", offline check | open |
| 19 | Windows exe release (existing scripts), web zip attached to the release, README with the play link | open |
| 20 | Test on real phone / tablet / browsers, performance and battery check, fix list | open |

## Later ideas
- Level editor (levels are data, so an in-game editor is possible), custom level import.
- Save export / import as text (move progress between devices).
- Daily challenge with a fixed seed; online leaderboard (needs a backend – out of scope for now).
