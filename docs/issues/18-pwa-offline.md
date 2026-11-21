# Installable offline web app (PWA)

`manifest.webmanifest` (name, icons 192 / 512 / maskable, `display: fullscreen`, orientation any), a service worker caching html / js / wasm with a versioned cache name and a cache-first-then-update strategy, registration in `index.html`, an 'update available' hint. Verify: load once, switch the network off, reload; add to the home screen on Android and iOS.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
