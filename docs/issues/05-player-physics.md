# Player: walking on slopes, ladders, jump, falling, death

Implement in `src/game/player.rs` as pure, tested code: walking along sloped girders (snap to the girder under the feet), grabbing / leaving ladders, a fixed-arc jump without air control, landing, fall damage above a height limit, hit detection with small forgiving hitboxes, death and respawn.
Tests: slope following, ladder enter / leave at the ends, jump arc length, fall limit, no tunnelling through a girder at the maximum speed.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
