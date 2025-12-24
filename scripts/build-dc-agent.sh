#!/bin/bash
# Build dc-agent in a Debian 11 container for glibc compatibility.
# Usage:
#   ./scripts/build-dc-agent.sh          # Docker-based build (for local dev)
#   ./scripts/build-dc-agent.sh --direct # Direct build (for CI or testing)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${OUTPUT_DIR:-$PROJECT_ROOT/target/release}"

# Direct build mode (CI or when deps are pre-installed)
if [[ "${1:-}" == "--direct" ]]; then
    echo "==> Building dc-agent (direct mode)..."
    cd "$PROJECT_ROOT"
    cargo build --release --bin dc-agent
    echo "==> Binary: $OUTPUT_DIR/dc-agent"
    exit 0
fi

# Docker-based build for local dev
echo "==> Building dc-agent in Debian 11 container..."

docker run --rm \
    -v "$PROJECT_ROOT":/build \
    -w /build \
    debian:11 \
    bash -c '
        set -euo pipefail
        apt-get update -qq
        apt-get install -y -qq curl gcc clang pkg-config libssl-dev ca-certificates >/dev/null
        curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable -q
        . "$HOME/.cargo/env"
        cargo build --release --bin dc-agent
    '

echo "==> Binary: $OUTPUT_DIR/dc-agent"
