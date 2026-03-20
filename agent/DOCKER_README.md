# AI Coding Agent Docker Setup

This directory contains a complete Docker setup for running AI coding agents (Claude Code, Codex, Happy Coder, OpenCode) safely with the Decent Cloud project. The container provides isolation while giving the AI agent full access to the project.

## Quick Start

**Run Claude Code:**
```bash
./run-container.sh claude
```

**Run Codex:**
```bash
./run-container.sh codex
```

**Run Happy Coder:**
```bash
./run-container.sh happy
```

**Run OpenCode:**
```bash
./run-container.sh opencode
```

## Files Overview

- **`Dockerfile`** - Based on `.github/container/Dockerfile` with Claude Code, Codex, Happy Coder, and OpenCode additions
- **`docker-compose.yml`** - Container orchestration with volumes and networking
- **`run-container.sh`** - Wrapper script for easy usage
- **`entrypoint.sh`** - Container entrypoint with permission fixes
- **`.dockerignore`** - Optimizes build context

## Architecture

This setup leverages the existing CI Dockerfile (`.github/container/Dockerfile`) as a base, adding:
- **Node.js 22** and npm for website development
- **Claude Code** installed globally via npm
- **Codex** installed globally via npm
- **Happy Coder** installed globally via npm
- **OpenCode** installed via official installer
- **Non-root user** for security
- **Proper volume mounting** at `/code` (matching CI Dockerfile structure)

## Usage Examples

### Basic Usage
```bash
# Start Claude Code with full project access
./run-container.sh claude

# Start Codex
./run-container.sh codex

# Start Happy Coder
./run-container.sh happy

# Start OpenCode
./run-container.sh opencode

# Start a bash shell
./run-container.sh bash

# Run without rebuilding (--no-build)
./run-container.sh claude --no-build

# Run in background
./run-container.sh claude --detach

# Run with a specific name (for concurrent agents)
./run-container.sh -n agent1 claude
```

### Custom Commands
```bash
# Run specific commands in the container
./run-container.sh claude "cargo test"
./run-container.sh codex "cargo test -p api"
./run-container.sh happy "cd website && npm run dev"
./run-container.sh bash "cargo make build"
```

## What's Included in the Container

### Development Tools
- **Rust** - Latest stable with wasm32 target
- **Node.js 22** - With npm
- **Python 3** - With pip, venv, and UV package manager
- **Claude Code** - Installed globally via npm
- **Codex** - Installed globally via npm
- **Happy Coder** - Installed globally via npm
- **OpenCode** - Installed via official installer
- **Docker CLI & Compose** - For running containers from within the container

### Project-Specific Tools
- **Internet Computer SDK** - dfx for ICP development
- **Pocket IC** - Local ICP testing
- **Cargo tools** - make, nextest
- **Project dependencies** - Pre-built and cached

### Safety Features
- **Non-root user** - Container runs as 'ubuntu' user
- **Isolated filesystem** - Only project directory is mounted
- **Network isolation** - Bridge network only
- **Cached volumes** - Separate caches for dependencies

## Volumes

The setup uses several volumes for caching and persistence:

- **`cargo-cache`** - Cargo registry cache
- **`rustup-cache`** - Rustup toolchain cache
- **`home-cache`** - Home directory cache (npm, uv, etc.)
- **`target-cache`** - Build artifacts (per-project)
- **Project mount** - Your entire project directory at `/code`
- **Docker socket** - Mounted at `/var/run/docker.sock` for Docker-in-Docker access
- **Config mounts** - `~/.claude`, `~/.codex`, `~/.happy`, `~/.opencode` for AI tool configs

## Troubleshooting

### Common Issues

1. **Permission denied on script**
   ```bash
   chmod +x run-container.sh
   # If that fails due to sandbox restrictions, run:
   bash run-container.sh claude
   ```

2. **Docker not running**
   ```bash
   # Start Docker daemon
   sudo systemctl start docker
   ```

3. **Port conflicts**
   ```bash
   # Check what's using ports 59010/59011
   lsof -i :59010
   # Or modify docker-compose.yml to use different ports
   ```

### Rebuilding

If you make changes to the project:
```bash
./run-container.sh claude  # Rebuilds by default
```

Or completely rebuild without cache:
```bash
docker-compose build --no-cache
```

## Development Workflow

1. **Daily usage:**
   ```bash
   ./run-container.sh claude
   ```

2. **Running tests:**
   ```bash
   ./run-container.sh claude "cargo test"
   ./run-container.sh codex "cargo clippy --tests"
   ./run-container.sh happy "cd website && npm test"
   ```

3. **Building:**
   ```bash
   ./run-container.sh claude "cargo make build"
   ```

4. **Cleanup:**
   ```bash
   # Stop and remove container
   docker-compose -p dc-agent-1 down

   # Remove cached volumes (if needed)
   docker volume rm dc-agent-1_cargo-cache dc-agent-1_rustup-cache dc-agent-1_home-cache
   ```

## Prove Browser Runs Inside Container

If you want hard evidence that Playwright/Chromium is running inside the `agent` container (not on host), run:

```bash
# 1) Start Codex in the agent container
./run-container.sh codex
```

Then inside Codex (same container shell context), run:

```bash
cd /code/decent-cloud

# Show container identity
echo "HOSTNAME=$(hostname)"
cat /proc/1/cgroup | head -n 5

# Launch browser tooling (starts/stops Chromium)
node scripts/browser.js eval https://dev.decent-cloud.org "navigator.userAgent"

# Capture Chromium PID while it is active
(node scripts/browser.js snap https://dev.decent-cloud.org >/tmp/browser-snap.log 2>&1 &) ; \
sleep 1; pgrep -af "chrome|chromium|playwright" | head -n 20
```

What to look for:
- `hostname` is the container hostname, not your host machine name.
- `/proc/1/cgroup` output is container-scoped.
- `pgrep` shows Chromium/Playwright process started by the in-container `node` command.

### Browser Engine Fallback

The browser helper defaults to Chromium. You can switch to Firefox if Chromium is unstable in your environment:

```bash
cd /code/decent-cloud
BROWSER_ENGINE=firefox node scripts/browser.js snap https://dev.decent-cloud.org/dashboard/saved
BROWSER_ENGINE=firefox node scripts/browser.js errs https://dev.decent-cloud.org/dashboard/saved
```

Supported values:
- `BROWSER_ENGINE=chromium` (default)
- `BROWSER_ENGINE=firefox`

## Running Multiple Agents Concurrently

The script supports running multiple agents in parallel using unique project names:

```bash
# Terminal 1: Start Claude Code
./run-container.sh claude

# Terminal 2: Start Codex (will use dc-agent-2)
./run-container.sh codex

# Terminal 3: Start OpenCode (will use dc-agent-3)
./run-container.sh opencode

# Or explicitly name them:
./run-container.sh -n claude1 claude
./run-container.sh -n codex1 codex
./run-container.sh -n opencode1 opencode
```

## Credential Management (dc-secrets)

Single source of truth for all credentials. Uses SOPS with age encryption — keys are visible in git diffs, values are encrypted. The `secrets/` directory is bind-mounted into all containers.

### How It Works

```
secrets/
  .sops.yaml          # SOPS config (committed to git)
  .age-identity       # Private key (NEVER committed — in .gitignore)
  shared/*.yaml       # SOPS-encrypted shared creds (committed to git)
  agents/*.yaml       # SOPS-encrypted per-agent creds (committed to git)
```

### First-Time Setup (on host)

```bash
scripts/dc-secrets init                          # Generate age keypair + .sops.yaml
scripts/dc-secrets set shared/env DATABASE_URL=postgres://test:test@postgres:5432/test
scripts/dc-secrets set shared/env TEST_DATABASE_URL=postgres://test:test@postgres:5432
# ... set other required credentials
```

### Adding/Editing Credentials

```bash
# CLI set (flock-protected for concurrent agents)
scripts/dc-secrets set shared/cloudflare CF_API_TOKEN=xxx CF_ZONE_ID=yyy

# Per-agent
scripts/dc-secrets set agents/agent-1 HETZNER_TOKEN=secret_1

# Interactive edit (opens $EDITOR with decrypted YAML, re-encrypts on save)
scripts/dc-secrets edit shared/api
```

### How Agents Load Credentials

On container startup, `entrypoint.sh` automatically runs `dc-secrets export --agent $AGENT_NAME` and exports all shared + agent-specific credentials as environment variables. No manual sourcing needed.

### CLI Reference

```bash
dc-secrets init                          # Initialize store
dc-secrets get <path> <key>              # Read one credential
dc-secrets set <path> <key>=<value> ...  # Write credentials (flock-protected)
dc-secrets delete <path> <key>           # Remove a credential
dc-secrets export [--agent <name>]       # Print all creds as key=value
dc-secrets list [<path>]                 # List files or keys
dc-secrets import <env-file> <path>      # Import a .env file
dc-secrets edit <path>                   # Open in $EDITOR via sops
```

Paths: `shared/<name>` for shared, `agents/<name>` for per-agent.

## Security Notes

- Container runs as non-root user
- Only project directory is mounted
- Network access limited to bridge network
- Credentials are SOPS-encrypted (AES-256-GCM + age) and committed to git
- Private key (`.age-identity`) is gitignored and never leaves the host
- Concurrent writes are flock-protected
- Container can be easily recreated if compromised
- AI tool configs are mounted read-only from host
