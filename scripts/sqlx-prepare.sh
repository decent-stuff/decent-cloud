#!/usr/bin/env bash
# Regenerate the committed sqlx offline cache the RIGHT way.
#
# The build reads query plans from the workspace-ROOT .sqlx/ (sqlx walks up from
# the api/ package dir). The only way to write there is `cargo sqlx prepare
# --workspace` from the repo root — which `cargo make sqlx-prepare` already does
# (temp DB + migrations + `--workspace -- -p api --tests`).
#
# The footgun this script exists to prevent: a bare `cargo sqlx prepare` run from
# api/ writes to the gitignored per-package api/.sqlx/, which the build then
# reads preferentially while the committed root cache silently goes stale — and
# the next fresh CI clone fails with "no cached data".
#
# After this finishes, commit the resulting root .sqlx/ changes.
set -eEuo pipefail

# Self-locate the repo root from this script's path (works from any CWD).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${ROOT_DIR}"

# Pre-flight: refuse to run while the footgun dir exists, so we never merge a
# split cache. (The api test sqlx_offline_cache_has_single_committed_source
# enforces the same invariant in CI.)
if [ -d api/.sqlx ]; then
    echo "✗ api/.sqlx/ exists — removing the stray per-package cache before regenerating." >&2
    echo "  (A bare 'cargo sqlx prepare' from api/ created it; only the workspace-root" >&2
    echo "   .sqlx/ is committed.)" >&2
    rm -rf api/.sqlx
fi

echo "→ Regenerating workspace-root .sqlx/ via 'cargo make sqlx-prepare' …"
cargo make sqlx-prepare

echo
echo "✓ Done. Review and commit root .sqlx/ changes:"
echo "    git status .sqlx"
