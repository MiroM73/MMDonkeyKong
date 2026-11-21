# Platforms: browser, mobile, desktop, offline

How one code base is built, published and run on every target.

[← Back to README](../README.md)

## Decision: one Rust code base, two build targets

The project already is a Rust / egui / eframe program. eframe compiles to **WebAssembly**, so the
**same game code** runs

| Target | Build | Where it runs |
|--------|-------|---------------|
| `x86_64-pc-windows-msvc` | `cargo build --release` | offline Windows exe (the game works without installation or internet) |
| `x86_64-unknown-linux-gnu` | same | CI checks it, optional Linux build |
| `wasm32-unknown-unknown` | `trunk build --release` | any modern browser – PC, tablet, phone – hosted on GitHub Pages |

Alternatives considered: a separate JavaScript / Canvas game (two code bases, the exe would need
Electron or a webview – big and not in the owner's Rust-based toolchain) or a game engine such as
Bevy / macroquad (a new dependency and a new way of working for a game this small). egui's
painter is enough for a 2-D pixel-art arcade game, the template (i18n, settings, CI, release
scripts) is reused unchanged, and the game logic stays a pure module testable on every platform.

## Code structure that makes this work

- `src/game/` – the **platform-independent game**: fixed-step simulation, levels, difficulty,
  scoring, progress. No egui, no I/O, no `std::time` – it gets `dt` and an `Input` struct, returns
  a state and a list of events (sounds). 100 % unit-tested natively.
- `src/app/` – egui front end: draws the game state with the painter, collects input (keyboard,
  touch, gamepad) into `Input`, menus, dialogs.
- `src/storage.rs` – the `Storage` trait: `FileStorage` (native, atomic write, existing `config.rs`
  code) and `WebStorage` (`localStorage` through `web-sys`).
- `src/audio.rs` – `Audio` trait: native backend (e.g. `rodio` / `cpal` or a tiny Win32 `waveOut`
  wrapper) and web backend (Web Audio through `web-sys`); sounds are synthesised, no files.
- Native-only code (Win32 FFI, `dirs`, `sys-locale`, the update check, windows as OS viewports,
  command line) is behind `#[cfg(not(target_arch = "wasm32"))]`; the browser build has one
  full-screen canvas and no separate windows. `Config::separate_windows` is not used in the web
  build (the convention of `CLAUDE.md` applies to the desktop build).
- `main.rs` has two entry points: native `fn main` and the wasm `start` through `eframe::WebRunner`.
- Dependencies that do not build for wasm are `cfg`-gated in `Cargo.toml`
  (`[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`).

## Web build and GitHub Pages

- Tool: **Trunk** (`cargo install trunk`, `rustup target add wasm32-unknown-unknown`) with an
  `index.html` at the repository root (canvas element, `<link data-trunk rel="rust" />`, manifest,
  icons). `trunk serve` for local development with live reload, `trunk build --release --public-url
  /MMDonkeyKong/` for the release.
- Hosting: **GitHub Pages** from a workflow `.github/workflows/pages.yml` (`actions/upload-pages-
  artifact` + `actions/deploy-pages`), triggered by a push to `master` (or by a release tag – to be
  decided). The game then lives at `https://mirom73.github.io/MMDonkeyKong/` (exact URL is written
  to the README once published). The Pages source in the repository settings is switched to
  "GitHub Actions" once, by the owner.
- Size: release wasm with `opt-level = "s"`/`"z"` for the web profile, `wasm-opt -Oz` (Trunk runs
  it), gzip is served by Pages. Target: under ~2 MB.
- The WebGL context comes from eframe's `glow` backend (works on all current mobile browsers); no
  WebGPU requirement.
- Tested browsers: Chrome / Edge / Firefox (PC), Safari iOS, Chrome Android. Touch input, the audio
  unlock on the first gesture and the viewport meta tag (`user-scalable=no`, `viewport-fit=cover`)
  are checked on a real phone and tablet before a release.

## Offline use

1. **Windows exe** – `mmdonkeykong.exe` from the GitHub release (existing `release.yml` /
   `scripts\release.ps1` / `package.ps1`). Static CRT, no installer, no internet; settings and saves in
   `%APPDATA%\MMDonkeyKong\`.
2. **Installable web app (PWA)** – a `manifest.webmanifest` and a service worker that caches the
   wasm, js and html. After the first visit the game starts without internet in any browser and
   can be "added to the home screen" on a phone / tablet, then looks like a native app (full
   screen, own icon). A version string in the cache name makes updates replace the old cache.
3. **Offline HTML bundle (optional)** – the Trunk output zipped as `mmdonkeykong-web.zip` in the
   release, openable from a local folder with a tiny static server (browsers block wasm over
   `file://`), so mostly useful for PC; the exe is the real offline version.

## Saves across platforms

Saves live in the platform storage (file / `localStorage`), so progress is **per device** unless the
owner wants more (see the open questions: a text export / import in the settings is the cheap
solution, cloud sync is out of scope).

## CI

- Existing `ci.yml` keeps fmt + clippy + tests on Windows and Linux.
- New job `wasm`: `cargo clippy --target wasm32-unknown-unknown -- -D warnings` and `trunk build
  --release` so the web build cannot rot.
- `pages.yml` deploys. The release workflow additionally attaches `mmdonkeykong-web.zip`.
