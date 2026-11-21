# Test on real devices and browsers; performance and battery

Checklist: Chrome / Edge / Firefox on PC, Safari on iPhone / iPad, Chrome on Android phone and tablet, in portrait and landscape; touch latency, multi-touch, audio unlock, saves surviving a reload, PWA offline start, frame rate and battery use over 10 minutes. Record results in `docs/history.md`, create an issue per defect.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
