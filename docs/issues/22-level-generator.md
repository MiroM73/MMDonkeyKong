# Level generator and validator (game::levelgen)

`generate(type, seed, size_hint) -> Level` producing valid levels for every level type, plus `validate(&Level)` (goal reachable by graph search over girders and ladders, ladders connect girders, gaps within the jump arc, spawn points on girders, hammers reachable). Invalid results are re-rolled with the next sub-seed. The generator writes the ASCII level format of issue 06 (`cargo run --example genlevel -- <type> <seed>` or a test helper prints it) so a good seed can be saved as a hand-tunable file. Tests: several hundred seeds per type validate; the same seed gives the same level.

## Done when
- Tests, fmt and clippy clean on Windows, Linux and wasm; behaviour checked in the running program; docs and roadmap updated; `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
