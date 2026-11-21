# Reusing code from the sibling projects

The template contains only what every project needs. Everything below already exists in
**RusTrace** (`D:\Rust\RusTrace`, `MiroM73/Rustrace`) or **MM PortLens** (`D:\Rust\MMPortLens`,
`MiroM73/MMPortLens`) – copy the files, rename `RusTrace` / `MMPortLens` to this project, add the
`t!` keys to all four `lang/*.lang`, and read the feature's section in the sibling's
`docs/architecture.md` first (the feature index is in the sibling's `CLAUDE.md`).

[← Back to README](../README.md)

| Feature | Files | Notes |
|---------|-------|-------|
| Update check, in-place update, retry / explained failures | RusTrace `src/update.rs`, `src/http.rs`, `src/app/dialogs/update.rs` (MM PortLens: `app/update_window.rs`) | needs `sha2`, `flate2`, `crc32fast`; the update window shows the GitHub release notes; keeps window / session state across the restart |
| Command line (`--help`, `--version`, `--lang`, invalid options) | RusTrace `src/cli.rs`, `src/console.rs`, `src/app/cli.rs`; MM PortLens `src/cli.rs`, `src/clihelp.rs`, `src/console.rs`, `src/bin/mmportlens-com.rs` | `console.rs` handles console encoding and attaching to the parent console; the `-com` companion binary is shipped as `.com` for cmd / PowerShell |
| Start-up splash | RusTrace `src/splash.rs` | Win32, drawn before the window exists |
| File association (`.log` …) | RusTrace `src/fileassoc.rs`, `src/app/dialogs/assoc.rs` | |
| Column filters (text, regex, ranges) | RusTrace `src/colfilter.rs`; MM PortLens `src/colfilter.rs` (+ CIDR, address classes), `src/app/colfilters.rs` | |
| Search / filter with background re-filter | RusTrace `src/filter.rs`, `src/app/filter.rs` | |
| Highlight rules with per-theme palettes | RusTrace `src/config.rs` (`HighlightRule`, `Palette`), `src/app/dialogs/rules.rs`; MM PortLens `src/rules.rs`, `src/app/rules.rs` | |
| Table export CSV / Excel / JSON Lines / HTML | RusTrace `src/tabexport/` (+ `src/batch.rs` for `--export`); MM PortLens `src/tabexport/` | own tiny `zip.rs` + `xlsx.rs`, no extra crates |
| Settings shared between running instances | RusTrace `src/config.rs` (`write_value`, merge), `src/app/sync.rs` | |
| Table with horizontal scrolling, column layout | RusTrace `src/app/table.rs`; MM PortLens `src/app/columnlayout.rs`, `src/columns.rs` | |
| Window title, default window state | RusTrace `src/app/title.rs`; `Startup` / `run` in `src/main.rs` | |
| Status bar, keyboard shortcuts window | RusTrace `src/app/statusbar.rs`, `src/app/dialogs/help.rs`, `src/app/input.rs` | |
| Guides / Help menu pointing to the docs | RusTrace `src/guides.rs`, `src/app/menu.rs` | |
| Background polling thread + change tracking | MM PortLens `src/poller.rs`, `src/model.rs` | the pattern for any periodic data source |
| Recording to SQLite and replay | MM PortLens `src/recording/`, `src/app/record.rs`, `src/app/replay.rs` | needs `rusqlite` (bundled) |
| **Separate OS windows for tool windows / dialogs (default for every new window)** | MM PortLens `src/app/native_window.rs` (`ToolWindow`, `ToolSpec`), `src/helpwindow.rs` (zoom), `Config::separate_windows`, `ToolWindowState`; example use `src/app/stats.rs` | immediate viewport, falls back to `egui::Window` when `ctx.embed_viewports()`; the convention for all new windows, see `CLAUDE.md` → Conventions; RusTrace migration: issues #68, #69 |
| Charts / statistics | MM PortLens `src/stats.rs`, `src/app/stats.rs`, `src/app/properties_graph.rs` | needs `egui_plot` |
| Watch rules + notification icon | MM PortLens `src/watch.rs`, `src/notify.rs`, `src/app/watch.rs` | needs `raw-window-handle` |
| Administrator mode (restart elevated) | MM PortLens `src/elevation.rs`, `src/app/admin.rs` | |
| Win32 FFI without a binding crate | MM PortLens `src/net/windows.rs`, `src/winproc.rs`, `src/gpu.rs` | `unsafe extern "system"` + `// SAFETY:` comments |
| Compact string store / memory-mapped reading of big files | RusTrace `src/store.rs`, `src/parser.rs` | needs `memmap2`, `encoding_rs` |

Dependency versions used by both projects are listed (commented) in `Cargo.toml`.
