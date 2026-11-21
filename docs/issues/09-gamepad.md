# Gamepad support (optional)

Gamepad API in the browser (`web-sys`) and `gilrs` natively (cfg-gated); stick / d-pad move and climb, A jumps, Start pauses. Low priority – do after the first release if wanted.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
