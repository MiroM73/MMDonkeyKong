# History

What was done when (one line per issue / release) and the open ideas of the owner.

[← Back to README](../README.md)

## Log

- Project created from the MMDonkeyKong (skeleton: window, menu, theme, languages, settings, About,
  CI and release scripts).
- 2026-11-20: convention added – every separately opened window must be movable outside the main
  window (own OS window with in-window fallback, MM PortLens `native_window.rs`); `CLAUDE.md`,
  `TEMPLATE.md`, `docs/reuse.md`. RusTrace migration: issues #68, #69.

- 2026-11-21: game specified – `docs/game-design.md` (mechanics, 4 level types, 5 difficulty steps,
  saves, level select, controls), `docs/platforms.md` (one Rust code base → wasm for the browser on
  GitHub Pages + PWA offline + Windows exe), roadmap in 8 phases, issue drafts 01–20 in
  `docs/issues/`. Nothing implemented yet.
- 2026-11-21: owner answered the design questions (recorded in `docs/game-design.md`): saves per
  difficulty with shared unlocking, local records, level generator, save export / import, optional
  cloud sync, deploy on every push, Windows-only exe. Issues 21-25 added to the roadmap.
- 2026-11-21: GitHub repository `MiroM73/MMDonkeyKong` created (public, `origin` added, nothing
  pushed yet) and issues #1-#25 created from `docs/issues/`.
- 2026-11-21: #2 game core skeleton: `src/game/` (state, `step`, `Input`, `Event`, seeded `Rng`,
  `FixedStep`), `Storage` and `Audio` traits with null implementations.
- 2026-11-21: #1 game window: the central panel is the game (224 x 256 playfield, integer scaling in
  physical pixels, 60 Hz fixed step from the frame time, keyboard, pause on Esc / P / click / lost
  focus, no repaint while paused). Placeholder marker moves with the arrow keys.
- 2026-11-21: #3 browser build: wasm32 target with Trunk (`index.html`, `Trunk.toml`), `main.rs`
  split into desktop / wasm entry points (`native.rs`), desktop-only code behind `cfg`, frame time from
  egui instead of `Instant`.
- 2026-11-21: #4 GitHub Pages: `pages.yml` deploys the Trunk build on every push to `master`,
  CI job `web` (clippy wasm32 + Trunk build), README play link and badge.

## Open ideas

- _(none yet)_
