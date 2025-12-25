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
        apt-get install -y -qq curl gcc pkg-config libssl-dev ca-certificates >/dev/null
        curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable -q
        . "$HOME/.cargo/env"
        # Remove mold linker config (not available in container), restore after build
        cp .cargo/config.toml /tmp/cargo-config-backup.toml
        trap "mv /tmp/cargo-config-backup.toml .cargo/config.toml" EXIT
        rm -f .cargo/config.toml
        cargo build --release --bin dc-agent
    '

echo "==> Binary: $PROJECT_ROOT/target/release/dc-agent"
