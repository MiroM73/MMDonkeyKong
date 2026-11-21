# Levels 5-8 and balance pass

Harder variants of the four types (more hazards, tighter timings, new layouts). Play every level on every difficulty step, tune `Difficulty` and the level timings, write the findings to `docs/history.md`.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
