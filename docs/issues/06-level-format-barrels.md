# Level file format, parser and the first level 'Barrels'

- Text format in `levels/*.txt` (name, type, grid or coordinates of girders, ladders, broken ladders, hammers, goal, ape position, spawn timings); a pure parser with clear line-numbered errors; levels embedded with `include_str!`.
- First level: rolling barrels (random descent on ladders, fall off the ends), hammer pickup (timed), the goal at the top, score for jumping over / smashing barrels, the time bonus counting down and added on completion, extra life at a score.
Tests: parser (valid, invalid), barrel movement, scoring, bonus. A test that parses every file in `levels/` and checks that the goal is reachable (simple graph search over girders and ladders).

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
