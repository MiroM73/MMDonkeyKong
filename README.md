# MMDonkeyKong – Donkey Kong style platform game

> **This program was created entirely by AI.** Everything in this repository – the source code,
> program logic, architecture, user interface design, translations, tests, build and release
> scripts and the documentation – was created by **Claude Opus 5.5** (model ID `claude-opus-5-5`),
> a large language model by [Anthropic](https://www.anthropic.com). The project owner only
> specified the requirements and gave feedback; he did not write the code.
> The code was compiled and tested (unit tests and checks in the running application) during
> generation. Review it like any other third-party code before using it in production.

An original single-player arcade platformer in the spirit of the classic 1981 ape-and-barrels cabinet: climb ladders, jump over rolling barrels, grab the hammer, reach the top. Written in Rust (egui/eframe); the same code runs **in a web browser** (PC, tablet, phone, hosted on GitHub Pages) and **offline as a Windows exe**. _Work in progress: the game is being designed, see the [game design](docs/game-design.md) and the [roadmap](docs/roadmap.md)._

[![CI](https://github.com/MiroM73/MMDonkeyKong/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/MiroM73/MMDonkeyKong/actions/workflows/ci.yml)
[![Pages](https://github.com/MiroM73/MMDonkeyKong/actions/workflows/pages.yml/badge.svg?branch=master)](https://github.com/MiroM73/MMDonkeyKong/actions/workflows/pages.yml)


## At a glance

- five difficulty steps, progress saved after every level, replay any completed level, top-ten high scores per difficulty
- keyboard, touch (phone / tablet) and gamepad controls
- playable in the browser, installable for offline use (web app), and a Windows exe
- light / dark / system theme, UI zoom (Ctrl + plus / minus)
- [translatable user interface](docs/languages.md) (English, Slovenčina, Čeština, Deutsch) – no recompiling
- settings in `%APPDATA%\MMDonkeyKong\config.json`, progress in `save.json` next to it (`localStorage` in the browser)

## Documentation

- [Game design](docs/game-design.md) · [Platforms: browser, mobile, offline exe](docs/platforms.md)
- [Languages and translations](docs/languages.md)
- [Development: tests, checks, CI](docs/development.md)
- [Making a release](docs/releasing.md)
- [Source layout and conventions](docs/source-layout.md)
- [Reusing code from the sibling projects](docs/reuse.md)
- [Architecture](docs/architecture.md) · [History](docs/history.md) · [Roadmap](docs/roadmap.md)

## Play online

<https://mirom73.github.io/MMDonkeyKong/> – PC, tablet and phone, nothing to install (rebuilt on every
push to `master`). Use the browser's *Install app* / *Add to home screen* to get an icon and offline
play; when a new version is published the menu bar shows *Update available - reload*.

## Play offline

- **Windows:** download `MMDonkeyKong-v<version>-windows-x64.exe` (self-extracting) or the `.zip`
  from the [latest release](https://github.com/MiroM73/MMDonkeyKong/releases/latest) and run
  `mmdonkeykong.exe`. No installer, no internet; settings and progress live in `%APPDATA%\MMDonkeyKong\`.
- **Any system with a browser:** the installed web app above works without internet after the first
  visit, or unzip `mmdonkeykong-web.zip` from the release and serve the folder with any static server
  (`python -m http.server 8080`, then open <http://localhost:8080/>; browsers do not run WebAssembly
  from `file://`).

## Build

```
cargo build --release      # targetelease\mmdonkeykong.exe
cargo test
```

Needs the Rust toolchain; on Windows the Visual Studio C++ build tools and the Windows SDK
(`rc.exe` embeds the icon). The C runtime is linked statically, so the exe runs without the
Visual C++ Redistributable.

### Build on Linux / macOS

Only the Windows exe is released, but the code builds elsewhere (CI builds and tests it on Ubuntu).
Install [Rust](https://rustup.rs), then:

```
# Debian / Ubuntu: gamepad support needs libudev; the window needs the usual X11 / Wayland / GL libraries
sudo apt-get install -y build-essential pkg-config libudev-dev libx11-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev
cargo run --release
```

On macOS install the Xcode command line tools (`xcode-select --install`) and run `cargo run --release`
(macOS is not covered by CI). The browser version of the game needs no build at all.

### Build the web version

```
rustup target add wasm32-unknown-unknown
cargo install trunk
trunk serve                                        # http://127.0.0.1:8080 with live reload
trunk build --release --public-url "./"            # dist\ = what mmdonkeykong-web.zip contains
```

## License

[The Unlicense](LICENSE) – public domain.
