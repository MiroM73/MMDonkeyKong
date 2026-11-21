# Keyboard and touch controls, portrait and landscape layouts

- Keyboard as in the design table; input collected into `game::Input` once per frame (key down state, jump as an edge so it is not lost between fixed steps).
- On-screen d-pad (left) and jump button (right), multi-touch (move + jump together), larger than ~9 mm hit areas, semi-transparent, no accidental scrolling or zoom; shown when a touch screen is detected or forced in the settings; size and side (left-handed) configurable.
- Portrait: playfield at the top, controls below; landscape: controls over the side edges.
- Vibration on hit through the web Vibration API where available (optional setting).
Verify on a real phone and a tablet, and with the browser's device emulation.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.

## Findings from #3 / #1 to handle here
- Directions are sampled once per frame as *held* state, so a tap shorter than one frame is lost
  (seen with scripted key taps in the browser). Latch direction presses like the jump edge, so every
  press moves at least one step.
- `GameSession` pauses on a click on the whole view (desktop convenience); replace it with a pause
  button, because on a touch screen the d-pad and jump taps would toggle the pause.
