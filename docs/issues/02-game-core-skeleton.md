# Platform-independent game module and the Input / Storage / Audio traits

Create `src/game/` with no egui, no I/O and no wall-clock time: `GameState`, `Input` (left, right, up, down, jump pressed / held), `step(&mut state, &Input)` returning a list of `Event`s (sounds, score popups), a seeded RNG (own small xorshift, no crate, deterministic for tests).

Add the traits `Storage` (load / save a string by key), `Audio` (play an `Event`) with null implementations for tests. Document the module in `docs/architecture.md` and `docs/source-layout.md`.
Test: the same seed and input sequence always gives the same state.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
