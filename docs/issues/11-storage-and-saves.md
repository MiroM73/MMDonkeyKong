# Storage backends and saved progress (file + localStorage)

- `FileStorage` (`%APPDATA%\MMDonkeyKong\save.json`, atomic write as in `config.rs`) and `WebStorage` (`localStorage`, key `mmdonkeykong.save`).
- `SaveData` with `version`, difficulty, highest completed level, per-level best scores, lives, score, settings; migration by version; a damaged save is ignored and kept as `.bak`.
- Autosave after every completed level and when leaving a level through the menu; 'Continue' on the title screen.
Tests: round trip, old version migrates, corrupt data does not crash, the save is not touched when the file cannot be written. Tests must not touch the real `%APPDATA%` file (CLAUDE.md).

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
