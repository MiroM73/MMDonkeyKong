# Save export / import as text (merge) for moving progress between devices

Settings -> Export progress: versioned JSON text (optionally base64 + checksum), copy to clipboard and save as a file (desktop file dialog, browser download). Import: paste text or load a file, validate, then **merge**: completed levels = union; best score per level and difficulty = the higher; high scores = merged top 10; never lower progress; show a summary before applying. Pure functions with tests: round trip, merge is commutative and idempotent, garbage / newer-version input is rejected with a clear message.

## Done when
- Tests, fmt and clippy clean on Windows, Linux and wasm; behaviour checked in the running program; docs and roadmap updated; `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
