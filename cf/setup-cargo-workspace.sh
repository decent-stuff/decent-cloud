#!/bin/sh
# Auto-discover and setup cargo workspace for caching
# This script is used in Dockerfile to future-proof cargo dependency caching

set -e

# Find all workspace members from root Cargo.toml
members=$(grep -A 100 '\[workspace\]' Cargo.toml | grep 'members = \[' -A 100 | sed -n '/members = \[/,/\]/p' | grep '"' | sed 's/.*"\([^"]*\)".*/\1/')

echo "Setting up workspace members..."
for member in $members; do
    if [ -f "$member/Cargo.toml" ]; then
        echo "  - $member"
        mkdir -p "$member/src"

        # Detect if it's a binary or library crate
        if grep -q '\[\[bin\]\]' "$member/Cargo.toml" 2>/dev/null || \
           grep -q '^path.*main\.rs' "$member/Cargo.toml" 2>/dev/null; then
            echo "fn main() {}" > "$member/src/main.rs"
        else
            echo "pub fn dummy() {}" > "$member/src/lib.rs"
        fi
    fi
done

echo "Workspace setup complete"
