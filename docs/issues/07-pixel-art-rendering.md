# Draw the game in code (pixel-art sprites, no image files)

Sprites as small const pixel arrays (hero 2-4 walk frames, climb, jump, hammer, ape throw, barrel roll frames, fire, explosion), drawn with `painter.rect_filled` per pixel run or cached in a texture (one `ColorImage` atlas, `TextureOptions::NEAREST`). Original designs only (see Legal in `docs/game-design.md`). Light / dark theme does not touch the playfield colours. Test: sprite dimensions and palette indices are valid.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
