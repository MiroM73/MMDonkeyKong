# Source layout

The files of `src/` and the conventions of the code.

[← Back to README](../README.md)

| File | Purpose |
|------|---------|
| `src/main.rs` | module list and the two entry points (desktop `main`, wasm32 `main` on the canvas) |
| `src/native.rs` | desktop only: window options, renderer choice (OpenGL, wgpu fallback with DX12 preference) |
| `index.html`, `Trunk.toml` | the web build (Trunk): page, canvas, no-zoom / no-scroll styles; `dist/` is the output (ignored) |
| `src/app/mod.rs` | the `App` struct, helper types / consts, `App::new`, the frame loop (`eframe::App::ui`) |
| `src/app/menu.rs` | menu bar: File, View (theme, language), Help |
| `src/app/sprites.rs` | the pixel-art sprites as const text-row arrays (`sprites!` macro, `Id`), `decode`, the `NEAREST` textures |
| `src/app/scene.rs` | draws the playfield: background raster (girders, ladders), animation frame choice, effects |
| `src/app/game_view.rs` | the game view: fixed-step loop from the frame time, keyboard input, scaled playfield drawing, pause |
| `src/app/dialogs/` | one file per window (`about.rs`) |
| `src/game/` | platform-independent game: `mod.rs` (`GameState`, `step`, `Event`), `player.rs`, `barrel.rs`, `level.rs` (format, parser, validator), `levels.rs` (embedded files), `input.rs`, `rng.rs`, `timestep.rs` (fixed step); later difficulty, progress |
| `src/storage.rs` | `Storage` trait + `NullStorage` / `MemoryStorage` (real backends: #11) |
| `src/audio.rs` | `Audio` trait + `NullAudio` (real backends: #16) |
| `levels/*.txt` | level data, embedded with `include_str!` (`src/game/levels.rs`); format: `docs/architecture.md` → Level format |
| `src/config.rs` | `Config` (serde defaults), load / atomic save |
| `src/i18n.rs` | `t!()` macro, language files, built-in languages, completeness test |
| `src/icon.rs` | the icon drawn in code (window icon, `.ico` for `build.rs`) |
| `build.rs` | embeds the icon and version information into the exe (Windows) |
| `lang/*.lang` | built-in translations (en, sk, cs, de) |

## Conventions

- `src/app/` is split by area (one `impl App` block per file). Each submodule starts with
  `use crate::app::*;`, which gives it the imports, private types and consts of `mod.rs`.
  Methods used outside their own file are `pub(in crate::app) fn`; shared free helpers are
  `pub(in crate::app)` and glob-imported in `mod.rs` (`use table::*;`). Keep files under ~800 lines.
- A new window → `src/app/dialogs/<name>.rs` and `mod <name>;` in `dialogs/mod.rs`.
- Pure logic goes into its own module with unit tests; UI code stays thin.
- Every user-visible string goes through `t!("key")`, and the key goes into all four `lang/*.lang`.
- Heavy work runs off the UI thread (a worker thread + a channel polled in the frame loop).
- New settings are fields of `Config` with a default.
- Code is `rustfmt`-clean and `clippy -D warnings`-clean; `#[allow(dead_code)]` only with a reason.
