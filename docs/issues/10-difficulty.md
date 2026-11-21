# Five difficulty steps

A `Difficulty` enum (Relaxed, Easy, Normal, Hard, Insane) and one table of parameters (lives, obstacle speed, spawn interval, ladder-descent chance, hammer count, score multiplier) as in `docs/game-design.md`. Chosen when starting a new game, changeable between levels in the settings. Geometry never changes. Tests: table is monotone (harder = faster / fewer lives), every parameter within sane bounds, Normal equals the 1.0x baseline. Balance numbers are tuned by playing and recorded in `docs/history.md`.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
