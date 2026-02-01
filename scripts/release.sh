#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./scripts/release.sh 0.1.0
#
# What it does:
# - updates crates/mds_cli/Cargo.toml version
# - regenerates Cargo.lock (required for --locked builds in CI)
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
lockfile="Cargo.lock"
changelog="CHANGELOG.md"

if ! [[ -f "$file" ]]; then
  echo "Expected file not found: $file" >&2
  exit 2
fi

if git rev-parse -q --verify "refs/tags/${tag}" >/dev/null; then
  echo "Tag already exists: ${tag}" >&2
  exit 2
fi

# Refuse to run with a dirty working tree (avoids accidentally committing unrelated changes).
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Working tree has local changes. Please commit/stash them before release." >&2
  git status --porcelain=v1 >&2 || true
  exit 2
fi

# Also refuse untracked files (release commit should be deterministic).
if git status --porcelain=v1 | grep -q '^\?\?'; then
  echo "Untracked files present. Please clean them up before release." >&2
  git status --porcelain=v1 >&2 || true
  exit 2
fi

# Update version in-place (safe quoting; fail if nothing changed).
VERSION="$version" perl -i -pe 'BEGIN{$v=$ENV{VERSION}; $n=0} $n += s/^version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+"/version = "$v"/; END{exit 2 if $n==0}' "$file"

# Ensure Cargo.lock matches the updated Cargo.toml.
# IMPORTANT: do NOT use --locked here (we want Cargo to refresh the lockfile deterministically).
cargo build -p mds_cli --release

# Sanity: make sure CI command will succeed locally too.
cargo build -p mds_cli --release --locked

# Stage all relevant tracked changes (not just Cargo.toml).
git add "$file" "$lockfile"
if [[ -f "$changelog" ]]; then
  git add "$changelog"
fi

git commit -m "release: ${tag}"
git tag "${tag}"
git push origin HEAD
git push origin "${tag}"

echo "Created release ${tag}. GitHub Actions should build binaries now."

