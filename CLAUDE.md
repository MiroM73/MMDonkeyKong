# MMDonkeyKong – project memory for Claude

> Template note: this file comes from the Rust project template. Fill in **"What the application is"** and
> the feature index, then delete this note.

## Working with the owner
- The owner (Miroslav, GitHub `MiroM73`) talks to Claude in **Slovak, informal "ty" form**. Answer in Slovak.
- Everything inside the repository is in **English**: code, comments, README, docs, commit messages,
  GitHub issues (with detailed descriptions) – except the translations in `lang/`.
- After every finished change: run the tests, verify the behaviour (in the running program when it
  is UI), update the documentation (`README.md` is a short overview; feature details go into the
  documents in `docs/` it links, internals into `docs/architecture.md`, a line per issue / release
  into `docs/history.md`), and make a commit with a **detailed English commit message**
  (what changed, why, how it was verified).
- **Close issues from the commit:** the commit message of a change that finishes an issue ends with a
  line `Closes #N` (one per issue: `Closes #12`, `Closes #13`) – GitHub then closes the issue by itself
  when the commit reaches `master` (after the owner's push). `(#N)` in the subject line is only a
  link and does **not** close anything. Use `Refs #N` for a commit that only partly solves an issue.
  Never close issues by hand with `gh issue close` for work that is committed.
- When Claude runs into something broken, a performance problem or an improvement idea while
  working on the project, it creates a new GitHub issue for it.
- The **first block of the README** states that everything (code, logic, architecture, UI design,
  translations, tests, scripts, docs) was created by Claude (Claude Opus 5.5, `claude-opus-5-5`,
  Anthropic) and the owner only specified requirements. Keep it at the top when editing the README.
- License: **the Unlicense** (public domain, `LICENSE`, `license = "Unlicense"` in Cargo.toml).
  `LICENSE` is also shipped in the release archive.
- This file (`CLAUDE.md`) and `docs/` are committed in the repository.
- Do not push or rewrite published history without asking. Nothing is released without asking.

## Naming
- All programs of the owner start with **MM**: display name with a space (**MMDonkeyKong**),
  technical name without (`MMDonkeyKong` repository and settings folder, `mmdonkeykong` crate / exe).
  Avoid names that collide with existing projects or carry unwanted associations.

## What the application is
_(Describe the application here: what it does, what it is modelled on, the target platform.
Main target is Windows; the code should keep building on Linux, CI checks it.)_

Siblings with the same structure and conventions: **RusTrace** (`MiroM73/Rustrace`, a CMTrace-style
log viewer) and **MM PortLens** (`MiroM73/MMPortLens`, a TCPView-style connection viewer). When a
feature exists there (i18n extras, column filters, auto refresh, table export, highlight rules,
updates, command line, splash, file association, …), take its code and adapt it instead of writing
it again – `docs/reuse.md` says where each feature lives.

- **`docs/architecture.md`** – how every part works inside. Read the section of a feature before
  changing it, and update it with the change.
- **`docs/history.md`** – what was done when (a line per issue / release), open ideas of the owner.
- **`docs/source-layout.md`** – the files of `src/` and the conventions of the code.

## Code layout
- `src/main.rs` – window start-up, renderer choice (OpenGL, wgpu fallback).
- `src/app/` – the whole UI (`App`), split by area: `mod.rs` holds the `App` struct, its helper
  types / consts, `App::new` and the frame loop; every other file is an `impl App` block for one
  area. Each submodule starts with `use crate::app::*;` (gets the parent's imports, private types
  and consts – child modules may use private items of their ancestors); methods used outside their
  own file are `pub(in crate::app) fn`; shared free helpers are `pub(in crate::app)` and glob-imported
  in `mod.rs`. A new window → `dialogs/<name>.rs` + `mod <name>;` in `dialogs/mod.rs`; keep files
  under ~800 lines.
- `src/config.rs` – `Config`, serde defaults / migration, atomic write.
- `src/i18n.rs` – `t!()`, language files; `lang/*.lang` – built-in translations (embedded with
  `include_str!`, shipped next to the exe).
- `src/icon.rs` – the icon drawn in code; `build.rs` embeds it and the version information into
  the exe (Windows, `winresource`).

## Conventions
- **Every user-visible string goes through `t!("key")`** (placeholders `t!("key", name = value)`).
  A new key must be added to **all four** `lang/*.lang` files with the same placeholders –
  the test `i18n::tests::builtin_languages_are_complete` enforces this.
- Keep heavy work (file reading, network, export writing) off the UI thread.
- New settings: add to `Config` with a default (`#[serde(default)]` keeps old config files loading).
- Prefer pure, unit-tested functions in modules, thin UI code in `src/app/`.
- egui 0.37 API notes: `eframe::App::ui(&mut self, ui, frame)`, `on_exit(&mut self, gl)`,
  `egui::Panel::top/bottom(..).show(ui, ..)`, `egui::CentralPanel::default().show(ui, ..)`,
  `egui::MenuBar::new().ui(..)`, `ui.close()`, `egui::Id::unique(..)` (not `Id::new`).
- Win32 calls are declared directly (`unsafe extern "system"` + `#[link(name = "...")]`) with a
  `// SAFETY:` comment; no binding crate unless the project really needs one.
- Idle CPU matters: repaint only when something changes (`request_repaint_after`), never every frame.
- **Separately opened windows (tool windows, dialogs, lists, statistics …) are movable outside the
  main window by default** – every new window of this kind is built like MM PortLens'
  `src/app/native_window.rs` (`ToolWindow` / `ToolSpec`): an OS window (egui immediate viewport)
  when `Config::separate_windows` is on (default on, View → *Open windows separately*) and the
  backend supports it (`!ctx.embed_viewports()`), otherwise an `egui::Window` inside the main
  window. Keep position / size (and zoom) in a `ToolWindowState` in `Config`. Do **not** add a plain
  `egui::Window` that is stuck inside the main window. Reference: MM PortLens issue #42,
  `docs/architecture.md` → Zoomable tool windows; RusTrace issues of the same kind: see
  `docs/reuse.md`.

## Commands
- `cargo build --release` → `target\release\mmdonkeykong.exe`; `cargo test`.
- `cargo fmt` + `cargo clippy --all-targets --locked -- -D warnings` – the code is rustfmt-clean and
  clippy-clean; CI (`.github/workflows/ci.yml`) fails otherwise and also runs clippy + tests on
  **Linux** – keep the non-Windows build working. Formatting-only commits go into `.git-blame-ignore-revs`.
- Do not leave test runs touching `%APPDATA%\MMDonkeyKong\config.json`: back it up and restore it
  (or delete it if it did not exist before).
- `cargo` is in `%USERPROFILE%\.cargo\bin` – add it to PATH in shells started before the Rust installation.
- Releases: `scripts\release.ps1 <version> [-Publish] [-DryRun] [-NoWatch] [-Yes] [-SkipTests]`; the
  owner runs it himself to save tokens. Details: `docs/releasing.md`.
- **Every release gets written release notes** on GitHub (the workflow only creates a draft with
  generated notes): `## What's new`, `## What's changed`, `## What's fixed`, `## Removed`
  ("Nothing." when empty), one item per issue with its number, user-facing (menu paths, short
  examples), then the `**Full Changelog**` compare link; source = `docs/history.md` and
  `git log v<prev>..v<new>`. Set them with `gh release edit v<ver> --notes-file <file>`.
- Line endings: `.gitattributes` forces LF for all text files (`* text=auto eol=lf`); write new
  files with LF.
- **Heredocs in the Bash tool break often**: a longer `python - <<'EOF' … EOF` (or `cat <<'EOF'`)
  with many quotes / triple quotes / backticks fails with `unexpected EOF while looking for
  matching '`. Do not retry the heredoc – write the script (or commit message) to a file in the
  scratchpad with the Write tool and run it (`python "<scratchpad>/x.py"`, `git commit -F
  "<scratchpad>/msg.txt"`). Short heredocs without quotes are fine.
- Windows PowerShell 5.1 drops double quotes inside native arguments (a `gh --jq '... "x" ...'`
  filter breaks): filter the JSON in PowerShell instead. Scripts must run under `pwsh` and 5.1.

## Working economically
- `docs/architecture.md` and `docs/history.md` grow large – never read them whole. Grep for the
  section heading and read that section. For the new `docs/history.md` line read only the place
  where it goes.
- Large sources (over ~800 lines): find the function with grep and read its range.
- When taking a feature from a sibling project, read only the files of that feature there (see the
  feature index in its `CLAUDE.md`), not the whole project.
- While iterating use `cargo check -q --message-format=short`. Run the full set once, when the
  change is finished: `cargo test -q`, `cargo fmt`,
  `cargo clippy -q --all-targets --locked -- -D warnings` (and the Linux clippy when platform code changed).
- When a test fails, rerun only that test (`cargo test -q <name>`) until it passes, then run the
  whole suite once.
- When an advisor is configured: consult it before finalizing a plan that touches more than one
  module or any `unsafe` Win32 code, when the same compile or test error appears a second time,
  and before the final commit of such a change.
