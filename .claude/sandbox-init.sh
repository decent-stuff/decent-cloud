#!/bin/bash

# Claude Code Sandbox Initialization Script for decent-cloud

set -e

echo "🚀 Initializing decent-cloud development sandbox..."

# Set up environment (matching Dockerfile)
export XDG_DATA_HOME=/usr/local
export PYTHONPATH=/code

# Initialize dfx if not already done
if [ ! -d "$XDG_DATA_HOME/dfinity" ]; then
    echo "📦 Initializing Internet Computer SDK..."
    dfx cache install
fi

# Check Rust toolchain
echo "🔧 Checking Rust toolchain..."
rustup show
rustup target list --installed | grep wasm32-unknown-unknown || rustup target add wasm32-unknown-unknown

# Verify Poetry dependencies
if [ -f "pyproject.toml" ]; then
    echo "🐍 Installing Python dependencies..."
    poetry install
fi

# Verify npm dependencies for website
if [ -d "website" ] && [ -f "website/package.json" ]; then
    echo "📱 Installing website dependencies..."
    cd website
    npm install
    cd ..
fi

# Check Pocket IC
if command -v pocket-ic &> /dev/null; then
    echo "✅ Pocket IC is available at: $(which pocket-ic)"
else
    echo "❌ Pocket IC not found"
    exit 1
fi

# Verify build tools
echo "🔍 Verifying build tools..."
cargo make --version || echo "⚠️  cargo-make not available"
pytest --version || echo "⚠️  pytest not available"

echo "✅ Sandbox initialization complete!"
echo ""
echo "Available commands:"
echo "  - Rust build/test: cargo make"
echo "  - Python tests: poetry run pytest"
echo "  - Website build: cd website && npm run build"
echo "  - Full test suite: cargo make && cd simulator && poetry run pytest && cd website && npm test"
echo ""
echo "Environment variables:"
echo "  - XDG_DATA_HOME: $XDG_DATA_HOME"
echo "  - POCKET_IC_BIN: $POCKET_IC_BIN"
echo "  - RUST_TOOLCHAIN: $RUST_TOOLCHAIN"