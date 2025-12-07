#!/bin/bash
# Entrypoint script: fix volume permissions then drop to ubuntu user
set -e

# Fix target directory ownership (volume may be created as root)
chown ubuntu:ubuntu /code/target 2>/dev/null || mkdir -p /code/target && chown ubuntu:ubuntu /code/target

# Execute command as ubuntu user
exec gosu ubuntu "$@"
