# Source layout

The files of `src/` and the conventions of the code.

[← Back to README](../README.md)

| File | Purpose |
|------|---------|
| `src/main.rs` | window start-up, renderer choice (OpenGL, wgpu fallback with DX12 preference) |
| `src/app/mod.rs` | the `App` struct, helper types / consts, `App::new`, the frame loop (`eframe::App::ui`) |
| `src/app/menu.rs` | menu bar: File, View (theme, language), Help |
| `src/app/game_view.rs` | the game view: fixed-step loop from the frame time, keyboard input, scaled playfield drawing, pause |
| `src/app/dialogs/` | one file per window (`about.rs`) |
| `src/game/` | platform-independent game: `mod.rs` (`GameState`, `step`, `Event`), `input.rs`, `rng.rs`, `timestep.rs` (fixed step); later levels, difficulty, progress |
| `src/storage.rs` | `Storage` trait + `NullStorage` / `MemoryStorage` (real backends: #11) |
| `src/audio.rs` | `Audio` trait + `NullAudio` (real backends: #16) |
| `levels/*.txt` | _(planned)_ level data, embedded with `include_str!` |
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
