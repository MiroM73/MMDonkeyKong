# Cloud-folder sync of the save on the desktop

Setting: sync folder path (e.g. a OneDrive / Google Drive / Dropbox folder). The game reads / writes `mmdonkeykong-save.json` there, merging with the local save using the rule of issue 21, on start, after every level and on exit; handles a missing / unreadable folder and conflicting copies from cloud clients without losing progress (never overwrite with a worse state). Desktop only.

## Done when
- Tests, fmt and clippy clean on Windows, Linux and wasm; behaviour checked in the running program; docs and roadmap updated; `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
