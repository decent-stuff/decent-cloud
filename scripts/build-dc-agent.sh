#!/bin/bash
# Build dc-agent in a Debian 11 container for glibc compatibility.
# Used by both local dev and CI.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

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

echo "==> Binary: $PROJECT_ROOT/target/release/dc-agent"
