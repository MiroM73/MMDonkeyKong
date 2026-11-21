# Levels 2-4: Conveyors, Lifts, Rivets

Add the new hazards to the simulation (conveyor belts with changing direction, loads, lifts, springs, fireballs that follow ladders, rivets to remove) and one level file each. Tests per hazard and the reachability test of issue 06 for every level.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
