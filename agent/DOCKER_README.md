# AI Coding Agent Docker Setup

This directory contains a complete Docker setup for running AI coding agents (Claude Code, Happy Coder, OpenCode) safely with the Decent Cloud project. The container provides isolation while giving the AI agent full access to the project.

## Quick Start

**Run Claude Code:**
```bash
./run-container.sh claude
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

- **`Dockerfile`** - Based on `.github/container/Dockerfile` with Claude Code, Happy Coder, and OpenCode additions
- **`docker-compose.yml`** - Container orchestration with volumes and networking
- **`run-container.sh`** - Wrapper script for easy usage
- **`entrypoint.sh`** - Container entrypoint with permission fixes
- **`.dockerignore`** - Optimizes build context

## Architecture

This setup leverages the existing CI Dockerfile (`.github/container/Dockerfile`) as a base, adding:
- **Node.js 22** and npm for website development
- **Claude Code** installed globally via npm
- **Happy Coder** installed globally via npm
- **OpenCode** installed via official installer
- **Non-root user** for security
- **Proper volume mounting** at `/code` (matching CI Dockerfile structure)

## Usage Examples

### Basic Usage
```bash
# Start Claude Code with full project access
./run-container.sh claude

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
./run-container.sh happy "cd website && npm run dev"
./run-container.sh bash "cargo make build"
```

## What's Included in the Container

### Development Tools
- **Rust** - Latest stable with wasm32 target
- **Node.js 22** - With npm
- **Python 3** - With pip, venv, and UV package manager
- **Claude Code** - Installed globally via npm
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
- **Config mounts** - `~/.claude`, `~/.happy`, `~/.opencode` for AI tool configs

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

## Running Multiple Agents Concurrently

The script supports running multiple agents in parallel using unique project names:

```bash
# Terminal 1: Start Claude Code
./run-container.sh claude

# Terminal 2: Start OpenCode (will use dc-agent-2)
./run-container.sh opencode

# Or explicitly name them:
./run-container.sh -n claude1 claude
./run-container.sh -n opencode1 opencode
```

## Security Notes

- Container runs as non-root user
- Only project directory is mounted
- Network access limited to bridge network
- No access to host system files or credentials
- Container can be easily recreated if compromised
- AI tool configs are mounted read-only from host
