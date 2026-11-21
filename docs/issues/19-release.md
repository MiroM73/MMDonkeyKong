# Release: Windows exe, web bundle, README with the play link

Use `scripts\release.ps1`; attach `mmdonkeykong-web.zip` (Trunk output) to the release; written release notes as required by CLAUDE.md; README 'Play online' / 'Play offline' sections. The owner runs the release script himself.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
