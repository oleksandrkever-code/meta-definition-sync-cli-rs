# Releasing (`mdsr-cli`)

## Tag & version

- Version lives in `crates/mds_cli/Cargo.toml` (`version = "X.Y.Z"`).
- Release binaries are built when you push a tag like `v0.1.0`.
- GitHub Actions will build and attach binaries for:
  - Linux
  - macOS (Apple Silicon / arm64)

### One-command release (recommended)

From repo root:

```bash
chmod +x ./scripts/release.sh
./scripts/release.sh 0.1.0
```

This updates `crates/mds_cli/Cargo.toml`, commits, tags `v0.1.0`, and pushes. The GitHub Actions release workflow will then publish binaries.

## What gets built

The release workflow builds the `mds_cli` package in `--release` mode and publishes the `mdsr-cli` binary as archives on the GitHub Release.

## Changelog convention

- Update `CHANGELOG.md` before tagging.
- Keep changes under `[Unreleased]` until you cut a release, then move them to a new version section (e.g. `[0.1.0] - 2026-02-01`).

## Smoke checklist (minimum)

- `mdsr-cli --help`
- `mdsr-cli metafield export --owner-type PRODUCT`
- `mdsr-cli metaobject export`
- Import smoke (non-destructive by default):
  - `mdsr-cli metafield import --owner-type PRODUCT`
  - `mdsr-cli metaobject import` (ensure dependency tree prints; you can skip in non-CI)

