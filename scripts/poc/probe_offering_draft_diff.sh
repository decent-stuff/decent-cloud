#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/../.." && pwd)"

echo "[PoC] Draft diff utility happy + error paths"
npm --prefix "$ROOT_DIR/website" run test -- src/lib/utils/offering-draft-diff.test.ts
