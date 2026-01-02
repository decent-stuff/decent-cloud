# Scripts

This directory contains development utility scripts (Python and Bash).

## Available Scripts

### `setup-python-env.py`
Sets up the Python virtual environment and installs project dependencies.

Usage:
```bash
python3 scripts/setup-python-env.py
```

This script will:
- Check Python version (requires 3.10+)
- Create a `.venv` virtual environment if it doesn't exist
- Install project dependencies from `pyproject.toml`
- Print activation instructions

### `install-pocket-ic.py`
Downloads and installs the pocket-ic server binary for testing.

Usage:
```bash
python3 scripts/install-pocket-ic.py
```

This script will:
- Detect your platform (Linux, macOS, Windows)
- Download the appropriate pocket-ic binary from GitHub releases
- Install it to `~/.local/bin/pocket-ic`
- Make it executable

### `docker-compose-health.sh`
Waits for a Docker Compose service to become healthy.

Usage:
```bash
./scripts/docker-compose-health.sh <service-name> [timeout_seconds]
```

Example:
```bash
./scripts/docker-compose-health.sh postgres 30  # Wait 30s for postgres
```

This script will:
- Validate service is running
- Poll for service health with configurable timeout (default 60s)
- PostgreSQL-specific: uses `pg_isready -U test -d test`
- Generic fallback: checks container health status
- Return 0 on healthy, 1 on timeout/failure with helpful error messages

## Usage with Cargo Make

You can also run these scripts using cargo make:

```bash
# Set up Python environment
cargo make setup-python

# Install pocket-ic server
cargo make install-pocket-ic

# Build the whitepaper (automatically sets up environment if needed)
cargo make build-whitepaper
```

