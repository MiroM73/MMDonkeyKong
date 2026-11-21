# Phone / browser sync through a private GitHub Gist or WebDAV

A browser cannot use an arbitrary file path, so sync through the network: a private GitHub Gist with a personal access token (scope `gist`) entered in the settings, or a WebDAV URL with credentials. Same merge rule as issue 21. The token is stored only on the device (never in the export, never committed) and the settings explain what it allows. Decide Gist vs WebDAV when the issue starts; CORS must be checked from the Pages origin.

## Done when
- Tests, fmt and clippy clean on Windows, Linux and wasm; behaviour checked in the running program; docs and roadmap updated; `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
