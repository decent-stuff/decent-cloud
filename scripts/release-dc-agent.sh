#!/bin/bash
# Build and release dc-agent locally via GitHub CLI.
# Usage: ./scripts/release-dc-agent.sh [VERSION]
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Preflight checks
command -v gh >/dev/null || { echo "ERROR: gh CLI not installed"; exit 1; }
gh auth status >/dev/null 2>&1 || { echo "ERROR: gh not authenticated (run: gh auth login)"; exit 1; }

cd "$PROJECT_ROOT"

# Check for uncommitted changes
if ! git diff --quiet HEAD; then
    echo "ERROR: Uncommitted changes. Commit or stash first."
    exit 1
fi

# Get version
CURRENT_VERSION=$(grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)
echo "Current version: $CURRENT_VERSION"
if [[ -n "${1:-}" ]]; then
    VERSION="$1"
else
    read -p "Release version: " -r VERSION
fi
[[ -n "$VERSION" ]] || { echo "ERROR: Version required"; exit 1; }
[[ "$VERSION" != v* ]] && VERSION="v$VERSION"
VERSION_NUM="${VERSION#v}"

# Check if tag exists
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    echo "ERROR: Tag $VERSION already exists"
    exit 1
fi

# Bump version in Cargo.toml files
echo ""
echo "==> Bumping version to $VERSION_NUM..."
for toml in $(find . -name "Cargo.toml" -not -path "./target/*"); do
    sed -i "s/^version = \"$CURRENT_VERSION\"/version = \"$VERSION_NUM\"/" "$toml"
done

# Build in Debian 11 container (updates Cargo.lock)
echo ""
"$SCRIPT_DIR/build-dc-agent.sh"

# Commit version bump (Cargo.toml + Cargo.lock)
echo "==> Committing version bump..."
git add -A
git commit -m "chore: bump version to $VERSION_NUM"

# Tag
echo "==> Creating tag $VERSION..."
git tag "$VERSION"

BINARY="$PROJECT_ROOT/target/release/dc-agent"
[[ -f "$BINARY" ]] || { echo "ERROR: Binary not found at $BINARY"; exit 1; }

# Confirm
echo ""
echo "Ready to release:"
echo "  Version: $VERSION"
echo "  Binary:  $BINARY ($(du -h "$BINARY" | cut -f1))"
echo ""
read -p "Push and create release? [y/N] " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted. Cleaning up..."
    git tag -d "$VERSION"
    git reset --hard HEAD~1
    exit 0
fi

# Push and release
echo "==> Pushing..."
git push origin main
git push origin "$VERSION"

echo "==> Creating GitHub release..."
gh release create "$VERSION" \
    --generate-notes \
    "$BINARY#dc-agent-linux-amd64"

echo ""
echo "Done! Release: https://github.com/decent-stuff/decent-cloud/releases/tag/$VERSION"
