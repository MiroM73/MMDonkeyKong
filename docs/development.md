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
