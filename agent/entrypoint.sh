#!/bin/bash
# Entrypoint script: fix volume permissions then drop to ubuntu user
set -e

# Fix target directory ownership (volume may be created as root)
chown ubuntu:ubuntu /code/target 2>/dev/null || mkdir -p /code/target && chown ubuntu:ubuntu /code/target

# Fix docker socket permissions for ubuntu user
if [ -S /var/run/docker.sock ]; then
    chown root:ubuntu /var/run/docker.sock
    chmod g+rw /var/run/docker.sock
fi

# Set up SSH key for remote node access
if [ -f /tmp/claude-ssh/claude-code ]; then
    mkdir -p /home/ubuntu/.ssh
    cp /tmp/claude-ssh/claude-code /home/ubuntu/.ssh/claude-code
    cp /tmp/claude-ssh/claude-code.pub /home/ubuntu/.ssh/claude-code.pub
    chmod 700 /home/ubuntu/.ssh
    chmod 600 /home/ubuntu/.ssh/claude-code
    chmod 644 /home/ubuntu/.ssh/claude-code.pub
    chown -R ubuntu:ubuntu /home/ubuntu/.ssh
    # Configure SSH to use this key by default and auto-accept new host keys
    cat > /home/ubuntu/.ssh/config <<'SSHEOF'
Host *
    IdentityFile ~/.ssh/claude-code
    StrictHostKeyChecking accept-new
SSHEOF
    chmod 600 /home/ubuntu/.ssh/config
    chown ubuntu:ubuntu /home/ubuntu/.ssh/config
fi

# Clean old build artifacts (files not accessed in 1 day)
gosu ubuntu cargo sweep --time 1 --installed /code/target 2>/dev/null || true

# Sync Python dependencies from project if pyproject.toml exists
if [ -f /code/pyproject.toml ]; then
    gosu ubuntu uv sync --project /code 2>/dev/null || true
fi

# Execute command as ubuntu user
exec gosu ubuntu "$@"
