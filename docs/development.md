# Development

Tests, checks and build details for contributors.

[← Back to README](../README.md)

Tests: `cargo test`

Checks before a commit / pull request (the same ones the CI runs): `cargo fmt --check`,
`cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`.
The workflow `.github/workflows/ci.yml` runs them on every push to `master` and on every pull
request – on Windows (plus a release build, downloadable as the `mmdonkeykong-windows-<commit>`
artifact of the run) and on Linux (clippy and tests, so the build keeps working there).
Formatting-only commits are listed in `.git-blame-ignore-revs`
(`git config blame.ignoreRevsFile .git-blame-ignore-revs` makes a local `git blame` skip them).

Tests must not touch the user's settings (`%APPDATA%\MMDonkeyKong\config.json`): test pure
functions, not `Config::load` / `save`.

The C runtime is linked statically (`.cargo/config.toml`), so the exe runs without the
Visual C++ Redistributable. `build.rs` embeds the program icon (drawn in code by `src/icon.rs`) and
the version information into the exe; this uses the resource compiler `rc.exe` of the Windows SDK
(installed with the Visual Studio C++ build tools).

Renderer: OpenGL (glow) by default, wgpu (DX12 preferred) when OpenGL cannot start;
`MMDONKEYKONG_RENDERER=wgpu` forces wgpu.

## Web build (browser)

One-time setup: `rustup target add wasm32-unknown-unknown` and `cargo install trunk --locked`.

- `trunk serve` - builds the game for the browser and serves it at <http://127.0.0.1:8080/> with
  live reload (`Trunk.toml`, `index.html`).
- `trunk build --release --public-url /MMDonkeyKong/` - the release build into `dist/` (what
  GitHub Pages serves, issue #4).
- `cargo check --target wasm32-unknown-unknown` / `cargo clippy --target wasm32-unknown-unknown
  -- -D warnings` - quick checks without Trunk. Keep them clean: code that only exists on the
  desktop is behind `#[cfg(not(target_arch = "wasm32"))]` (`src/native.rs`, `src/icon.rs`, the File
  menu, the `dirs` dependency).
- Do not remove `data-wasm-opt-params="--all-features"` from `index.html`: current Rust emits
  bulk-memory instructions and Trunk's `wasm-opt` rejects them without it.
- Size of the release bundle (2026-11-21): 4.1 MB wasm, 1.6 MB gzipped (what GitHub Pages sends),
  above the 2 MB goal only uncompressed. A `release-web` profile (opt-level `s`) is not needed
  yet; revisit when the game grows (#20).
- Installing Trunk with `cargo install trunk --locked` takes ~9 minutes; CI uses a prebuilt binary (#4).
- In the browser `std::time::Instant` / `SystemTime` panic: take time from egui
  (`ctx.input(|i| i.time)`), never from `std::time`.
- Settings are not stored in the browser yet (`Config::path()` is `None` there, saves are
  silently skipped); the browser storage backend comes with #11.
