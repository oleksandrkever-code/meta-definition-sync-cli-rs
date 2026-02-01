# Changelog

This project follows **Keep a Changelog** and **Semantic Versioning**.

## [Unreleased]

### Added

### Changed

### Fixed

## [0.1.1] - 2026-02-01

### Added
- Rustdoc publishing to GitHub Pages (CI workflow) and documentation links in README.

### Fixed
- Metaobjects: make cross-environment metaobject reference validations portable by converting
  `metaobject_definition_ids` ⇄ `metaobject_definition_types` during export/import and failing fast
  when configs contain store-specific definition IDs.

## [0.1.0] - 2026-02-01

### Added
- `mdsr-cli` command-line tool for syncing Shopify **metafield definitions** and **metaobject definitions** via the Admin GraphQL API.
- Metafield definitions:
  - Export to `definitions/metafields/<owner>.json` (filters out `shopify` namespaces)
  - Import from JSON with create/update/recreate/no-change planning
  - Cross-environment metaobject reference handling (`metaobject_definition_type` ⇄ `metaobject_definition_id`)
  - Import reports written under `reports/metafield-definitions:import/`
- Metaobject definitions:
  - Export to `definitions/metaobjects.json` (filters out system types starting with `shopify--`)
  - Import with dependency planning (prints a dependency tree) and level-by-level execution
  - Import reports written under `reports/metaobject-definitions:import/`
- CI and release automation:
  - GitHub Actions CI on Linux + macOS (arm64)
  - Tag-based GitHub Releases with prebuilt binaries and stable “latest download” asset names
- Version and update UX:
  - `mdsr-cli --version`
  - `mdsr-cli version --check` (checks GitHub Releases for updates)
  - Best-effort startup update check (cached; configurable via env)

### Changed
- Default import behavior is non-destructive (no deletions by default); destructive actions require explicit flags (planned).

### Fixed
- Improved developer ergonomics with Lefthook `pre-commit` hooks (fmt/clippy/tests) and clippy-clean codebase.

