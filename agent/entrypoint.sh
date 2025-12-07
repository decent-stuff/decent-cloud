#!/bin/bash
# Entrypoint script: fix volume permissions then drop to ubuntu user
set -e

# Fix target directory ownership (volume may be created as root)
chown ubuntu:ubuntu /code/target 2>/dev/null || mkdir -p /code/target && chown ubuntu:ubuntu /code/target

# Clean old build artifacts (files not accessed in 7 days)
gosu ubuntu cargo sweep --time 7 --installed /code/target 2>/dev/null || true

# Execute command as ubuntu user
exec gosu ubuntu "$@"
