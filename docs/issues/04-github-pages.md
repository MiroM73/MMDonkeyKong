# Deploy the web build to GitHub Pages and check it in CI

- `.github/workflows/pages.yml`: install the wasm target and Trunk, `trunk build --release --public-url /MMDonkeyKong/`, `actions/upload-pages-artifact`, `actions/deploy-pages` (permissions `pages: write`, `id-token: write`); triggered by **every push to `master`** + manual.
- CI job `wasm` in `ci.yml`: clippy for the wasm target and a Trunk build.
- The owner switches Settings -> Pages -> Source to 'GitHub Actions' once (describe it in `docs/releasing.md`).
- README: play link `https://mirom73.github.io/MMDonkeyKong/` and a badge.
Verify: the page opens from the public URL on a PC and on a phone.

## Done when
- `cargo test`, `cargo fmt`, `cargo clippy --all-targets --locked -- -D warnings` are clean (Linux build keeps working; after issue 03 also the wasm build).
- Behaviour checked in the running program (desktop and, from issue 03 on, the browser).
- README, `docs/architecture.md`, `docs/history.md` and `docs/roadmap.md` updated; new `t!` keys in all four `lang/*.lang`.

See `docs/game-design.md` and `docs/platforms.md`.
