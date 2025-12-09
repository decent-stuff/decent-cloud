#!/bin/bash
# Entrypoint script: fix volume permissions then drop to ubuntu user
set -e

# Fix target directory ownership (volume may be created as root)
chown ubuntu:ubuntu /code/target 2>/dev/null || mkdir -p /code/target && chown ubuntu:ubuntu /code/target

# Clean old build artifacts (files not accessed in 1 day)
gosu ubuntu cargo sweep --time 1 --installed /code/target 2>/dev/null || true

# Sync Python dependencies from project if pyproject.toml exists
if [ -f /code/pyproject.toml ]; then
    gosu ubuntu uv sync --project /code 2>/dev/null || true
fi

# Execute command as ubuntu user
exec gosu ubuntu "$@"
