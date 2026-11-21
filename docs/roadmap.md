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
The issue drafts are in `docs/issues/` (`scripts\create-issues.ps1`); the issues exist on GitHub
(`MiroM73/MMDonkeyKong`) and their numbers are the same as the draft numbers (`NN-*.md`): issue 03
in the tables = GitHub #3.

Principle: **playable as early as possible** – a walking, jumping hero on one level (phase 3) comes
before menus, saves and polish, and the web build (phase 2) exists from the start so that touch
and browser problems are found early, not at the end.

## Phase 0 – decisions
| Issue | What | Status |
|-------|------|--------|
| – | Owner answers the open questions of `docs/game-design.md` (answered 2026-11-21, recorded there) | done |

## Phase 1 – foundation (done, not released yet)
| Issue | What | Status |
|-------|------|--------|
| 01 | Turn the template into a game window: playfield with logical resolution, scaling, fixed-step loop, pause | done |
| 02 | Platform-independent `game` module skeleton + `Input` / `Storage` / `Audio` traits | done |

## Phase 2 – browser build (done, not released yet; early, so every later step is tested there too)
| Issue | What | Status |
|-------|------|--------|
| 03 | wasm32 build with Trunk, `cfg`-gate native-only code, local `trunk serve` | done |
| 04 | GitHub Pages deployment workflow (every push to `master`) + CI `wasm` job | done (verify the first deployment after the push) |

## Phase 3 – first playable level
| Issue | What | Status |
|-------|------|--------|
| 05 | Player physics: platforms, slopes, ladders, jump, death, tests | done |
| 06 | Level format + parser + first level "Barrels" (barrels, hammer, goal, score, time bonus) | done |
| 07 | Pixel-art rendering in code (hero, ape, barrels, girders, ladders) | done |

## Phase 4 – input on every device
| Issue | What | Status |
|-------|------|--------|
| 08 | Keyboard + touch controls (multi-touch d-pad and jump), portrait / landscape layouts, no page zoom / scroll | done |
| 09 | Gamepad support (optional) | done |

## Phase 5 – difficulty, progress, level select
| Issue | What | Status |
|-------|------|--------|
| 10 | Five difficulty steps (`Difficulty` table: speed / spawn / lives / score) | done |
| 11 | `Storage` backends (file, localStorage), versioned save, autosave after each level, Continue | done |
| 12 | Level-select screen: completed levels + the next one, locked levels greyed out | open |
| 13 | High scores per difficulty | open |
| 21 | Save export / import as text (merge rule), moving progress between phone and PC | open |

## Phase 6 – more content
| Issue | What | Status |
|-------|------|--------|
| 14 | Levels 2–4: Conveyors, Lifts, Rivets (new hazards, level files) | open |
| 15 | Levels 5–8: harder variants; balance pass over all five difficulties | open |
| 22 | Level generator + validator (`game::levelgen`), levels from seeds, test over many seeds | open |

## Phase 7 – sound and polish
| Issue | What | Status |
|-------|------|--------|
| 16 | Synthesised sound effects and music, native + Web Audio backends, mute / volume | open |
| 17 | Menus, title screen, settings, translations (en / sk / cs / de), About / credits | open |

## Phase 8 – offline and release
| Issue | What | Status |
|-------|------|--------|
| 18 | PWA: manifest, icons, service worker, "add to home screen", offline check | open |
| 19 | Windows exe release (existing scripts), web zip attached to the release, README with the play link and the Linux / macOS build instructions | open |
| 20 | Test on real phone / tablet / browsers, performance and battery check, fix list | open |

## Phase 9 – after the first release
| Issue | What | Status |
|-------|------|--------|
| 23 | Cloud-folder sync of the save on the desktop (path in settings, merge rule) | open |
| 24 | Phone sync via private GitHub Gist or WebDAV (token in settings) | open |
| 25 | In-game level editor with test play and text export / import | open |

## Later ideas
- Daily challenge with a fixed seed; online leaderboard (needs a backend – out of scope for now).
