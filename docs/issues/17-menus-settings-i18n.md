# Title screen, menus, settings and translations

Title, new game, continue, level select, high scores, settings (language, theme, sound, controls, difficulty, display scale), About with credits. Everything through `t!()` in en / sk / cs / de. Reuse the template's language selector and settings handling.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
