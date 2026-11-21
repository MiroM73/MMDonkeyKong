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

**Play in the browser:** <https://mirom73.github.io/MMDonkeyKong/> (rebuilt on every push to `master`)

## At a glance

- five difficulty steps, progress saved after every level, replay any completed level
- keyboard, touch (phone / tablet) and gamepad controls (gamepad planned)
- playable in the browser and installable for offline use (gamepad planned), Windows exe (planned)
- light / dark / system theme, UI zoom (Ctrl + plus / minus)
- [translatable user interface](docs/languages.md) (English, Slovenčina, Čeština, Deutsch) – no recompiling
- settings in `%APPDATA%\MMDonkeyKong\config.json`

## Documentation

- [Game design](docs/game-design.md) · [Platforms: browser, mobile, offline exe](docs/platforms.md)
- [Languages and translations](docs/languages.md)
- [Development: tests, checks, CI](docs/development.md)
- [Making a release](docs/releasing.md)
- [Source layout and conventions](docs/source-layout.md)
- [Reusing code from the sibling projects](docs/reuse.md)
- [Architecture](docs/architecture.md) · [History](docs/history.md) · [Roadmap](docs/roadmap.md)

## Build

```
cargo build --release      # target\release\mmdonkeykong.exe
cargo test
```

Needs the Rust toolchain; on Windows the Visual Studio C++ build tools and the Windows SDK
(`rc.exe` embeds the icon). The C runtime is linked statically, so the exe runs without the
Visual C++ Redistributable.

## License

[The Unlicense](LICENSE) – public domain.
