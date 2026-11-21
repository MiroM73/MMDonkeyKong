# Turn the template into a game window (playfield, scaling, fixed step, pause)

Replace the template's central panel with a game view.

- Logical playfield 224 x 256 drawn with the egui painter, scaled to the available area (integer scale when it fits, otherwise fractional), letter-boxed, centred, in portrait and landscape.
- Fixed-step game loop (60 Hz) fed by the frame time with an accumulator and a cap against the spiral of death; rendering interpolation is not needed at first.
- Repaint only while the game runs (`request_repaint`), idle in menus / pause (CLAUDE.md: idle CPU).
- Pause on Esc / P, on window focus loss and (web) on tab hidden.
- Keep the menu bar on desktop; hide it on small / touch screens.
- Update the About text, the README description and `Cargo.toml` description (typo 'plathorm').

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
