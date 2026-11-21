# Synthesised sound effects and music (native + Web Audio)

Square / triangle / noise generator producing sample buffers (pure, tested); effects: jump, land, hit, smash, pickup, level done, game over, short music loop. Web backend with Web Audio (unlocked on the first user gesture), native backend (`rodio` or a tiny `waveOut` wrapper - decide when starting, keep the dependency light). Mute and volume in the settings, saved.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
