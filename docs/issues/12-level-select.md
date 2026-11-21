# Level-select screen (completed levels + the next one)

Grid of level tiles with number, name, best score and a star / medal per difficulty. Completed levels and the next one are playable; later ones are greyed out with a lock icon. Replaying a level never lowers the progress. Reachable from the title screen and from the pause menu; large touch-friendly tiles, keyboard / gamepad focus navigation. Tests for the unlock logic.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
