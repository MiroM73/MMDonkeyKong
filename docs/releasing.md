# Making a release

How the Windows release archives are built and published (maintainer information).

[← Back to README](../README.md)

- Locally: `cargo build --release`, then `scripts\package.ps1` (needs 7-Zip) →
  `dist\MMDonkeyKong-v<version>-windows-x64.exe` (self-extracting) and `.zip`, each with a `.sha256`.
- On GitHub, in one step: `scripts\release.ps1 0.2.0` (add `-Publish` to publish right after the
  build, `-DryRun` to only run the checks and tests, `-SkipTests` when the local tests cannot pass
  for a reason outside the code). It checks the repository (master, clean, not behind, tag not used
  yet), sets the version in `Cargo.toml`, runs the tests (backing up
  `%APPDATA%\MMDonkeyKong\config.json`), asks for confirmation, commits, pushes, tags, waits for the
  workflow and shows the draft. Runs under PowerShell 7 (`pwsh`) and Windows PowerShell 5.1. Run
  `Get-Help scripts\release.ps1 -Full` for the details.
- On GitHub by hand: bump `version` in `Cargo.toml`, commit, then push a tag, e.g.
  `git tag v0.1.0 && git push origin v0.1.0`. The workflow `.github/workflows/release.yml` runs the
  tests, builds, packages and creates a **draft** release with the archives attached; review it and
  publish. A manual run of the workflow (Actions → Release → Run workflow) only builds the archives as
  a downloadable artifact.
- **Release notes** are written by hand for every release (sections *What's new*, *What's changed*,
  *What's fixed*, *Removed*, then the *Full Changelog* link) from `docs/history.md` and
  `git log v<prev>..v<new>`: `gh release edit v<version> --notes-file <file>`.

## The web version (GitHub Pages)

`.github/workflows/pages.yml` builds the browser version with Trunk (a prebuilt, checksum-verified
Trunk binary, pinned in the workflow) and deploys it to <https://mirom73.github.io/MMDonkeyKong/>
on **every push to `master`** (not tied to the Windows releases). One-time setting by the owner:
repository Settings -> Pages -> Source = "GitHub Actions". The URL path is case-sensitive
(`/MMDonkeyKong/`, passed to `trunk build --public-url`). To update the pinned Trunk, change
`TRUNK_VERSION` and `TRUNK_SHA256` (the `.sha256` file of the release asset) in `pages.yml` and in
the `web` job of `ci.yml`. CI (`ci.yml`, job `web`) checks clippy for wasm32 and a Trunk build on
every push and pull request, so a broken web build is found before the deployment.

When the program ships more files than the exe (language files, data files), add them to
`scripts\package.ps1`.
