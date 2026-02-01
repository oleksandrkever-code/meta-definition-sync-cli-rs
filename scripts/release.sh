#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/release.sh 0.1.0
#
# What it does:
# - updates crates/mds_cli/Cargo.toml version
# - commits the change
# - creates git tag vX.Y.Z
# - pushes commit + tag to origin
#
# After that, GitHub Actions "Release" workflow builds and attaches binaries.

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <version> (example: 0.1.0)" >&2
  exit 2
fi

version="$1"
tag="v${version}"

file="crates/mds_cli/Cargo.toml"

if ! [[ -f "$file" ]]; then
  echo "Expected file not found: $file" >&2
  exit 2
fi

if git rev-parse -q --verify "refs/tags/${tag}" >/dev/null; then
  echo "Tag already exists: ${tag}" >&2
  exit 2
fi

perl -0777 -i -pe "s/(\\nversion\\s*=\\s*\")([0-9]+\\.[0-9]+\\.[0-9]+)(\"\\s*\\n)/\$1${version}\$3/s" "$file"

git add "$file"
git commit -m "release: ${tag}"
git tag "${tag}"
git push origin HEAD
git push origin "${tag}"

echo "Created release ${tag}. GitHub Actions should build binaries now."

