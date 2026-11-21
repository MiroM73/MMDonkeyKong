# Build the game for the browser (wasm32 + Trunk)

Make the project build for `wasm32-unknown-unknown` with eframe's web runner.

- `cfg(not(target_arch = "wasm32"))` for native-only code: `dirs`, `sys-locale` (use `web-sys` `navigator.language` in the browser), renderer fallback, console handling, separate OS windows, any Win32 FFI. Target-specific dependency tables in `Cargo.toml`.
- Two entry points in `main.rs`: native `main`, and a wasm `start` with `eframe::WebRunner` on a canvas.
- `index.html` for Trunk (canvas, viewport meta with `user-scalable=no`, `viewport-fit=cover`, `touch-action: none`, full-screen canvas, loading text), `Trunk.toml`.
- A `[profile.release-web]` (opt-level `s`, LTO) if the size needs it; target under ~2 MB.
- Document the setup (`rustup target add wasm32-unknown-unknown`, `cargo install trunk`, `trunk serve`) in `docs/development.md`.
Verify: `trunk serve`, open in Chrome and Firefox, the window shows and resizes.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
