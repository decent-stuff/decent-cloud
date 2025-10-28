# Claude Code Docker Setup

This directory contains a complete Docker setup for running Claude Code safely with the Decent Cloud project. The container provides isolation while giving Claude full access to the project, effectively replacing the need for `--dangerously-skip-permissions` on the host system.

## Quick Start

**Run Claude Code:**
   ```bash
   ./claude-docker.sh
   ```

## Files Overview

- **`Dockerfile`** - Based on `.github/container/Dockerfile` with Claude Code additions
- **`docker-compose.yml`** - Container orchestration with volumes and networking
- **`claude-docker.sh`** - YAGNI wrapper script for easy usage
- **`.dockerignore`** - Optimizes build context

## Architecture

This setup leverages the existing CI Dockerfile (`.github/container/Dockerfile`) as a base, adding:
- **Node.js 22** and npm for website development
- **Claude Code** installed globally
- **Non-root user** for security
- **Proper volume mounting** at `/code` (matching CI Dockerfile structure)

## Usage Examples

### Basic Usage
```bash
# Start Claude Code with full project access
./claude-docker.sh

# Rebuild the image first
./claude-docker.sh --rebuild

# Run in background
./claude-docker.sh --detach

# Alternative: Run directly with docker-compose (if script permissions fail)
export ANTHROPIC_API_KEY=your_key_here
docker-compose up decent-cloud-claude
```

### Custom Commands
```bash
# Run a shell in the container
./claude-docker.sh --shell

# Run specific commands
./claude-docker.sh "cargo test"
./claude-docker.sh "cd website && npm run dev"
./claude-docker.sh "cargo make build"
```

## What's Included in the Container

### Development Tools
- **Rust** - Latest stable with wasm32 target
- **Node.js 22** - With npm
- **Python 3** - With UV package manager
- **Claude Code** - Installed globally via npm

### Project-Specific Tools
- **Internet Computer SDK** - dfx for ICP development
- **Pocket IC** - Local ICP testing
- **Cargo tools** - make, nextest, wasm-pack
- **Project dependencies** - Pre-built and cached

### Safety Features
- **Non-root user** - Container runs as 'developer' user
- **Isolated filesystem** - Only project directory is mounted
- **Network isolation** - Bridge network only
- **Cached volumes** - Separate caches for dependencies

## Benefits vs Host `--dangerously-skip-permissions`

| Feature | Host Dangerous Mode | Docker Container |
|---------|-------------------|------------------|
| **Safety** | ❌ Full host access | ✅ Container isolation |
| **Cleanup** | ❌ Manual cleanup | ✅ Delete container |
| **Reproducibility** | ❌ Host-dependent | ✅ Consistent environment |
| **Resource Limits** | ❌ Unlimited | ✅ Configurable |
| **Networking** | ❌ Full access | ✅ Bridge network |

## Volumes

The setup uses several volumes for caching and persistence:

- **`cargo-cache`** - Cargo registry cache
- **`npm-cache`** - Node.js modules cache
- **`uv-cache`** - Python package cache
- **Project mount** - Your entire project directory at `/workspace`

## Troubleshooting

### Common Issues

1. **Permission denied on script**
   ```bash
   chmod +x claude-docker.sh
   # If that fails due to sandbox restrictions, run:
   bash claude-docker.sh
   ```

2. **Docker not running**
   ```bash
   # Start Docker daemon
   sudo systemctl start docker
   ```

3. **Port conflicts**
   ```bash
   # Check what's using port 3000
   lsof -i :3000
   # Or modify docker-compose.yml to use different port
   ```

### Debug Mode

To run the container with more debugging:
```bash
docker-compose -f docker-compose.yml up decent-cloud-claude
```

### Rebuilding

If you make changes to the project:
```bash
./claude-docker.sh --rebuild
```

Or completely rebuild without cache:
```bash
docker-compose build --no-cache
```

## Development Workflow

1. **Daily usage:**
   ```bash
   ./claude-docker.sh
   ```

2. **Running tests:**
   ```bash
   ./claude-docker.sh "cargo test"
   ./claude-docker.sh "cd website && npm test"
   ```

3. **Building:**
   ```bash
   ./claude-docker.sh "cargo make build"
   ```

4. **Cleanup:**
   ```bash
   # Stop and remove container
   docker-compose down

   # Remove cached volumes (if needed)
   docker volume rm decent-cloud_cargo-cache decent-cloud_npm-cache decent-cloud_uv-cache
   ```

## Security Notes

- ✅ Container runs as non-root user
- ✅ Only project directory is mounted
- ✅ Network access limited to bridge network
- ✅ No access to host system files or credentials
- ✅ Container can be easily recreated if compromised

## Advanced Usage

### Custom Docker Compose Files

```bash
# Use different compose file
./claude-docker.sh -f docker-compose.dev.yml

# Production setup
./claude-docker.sh -f docker-compose.prod.yml
```

### Running Multiple Services

The docker-compose setup can be extended to include additional services like databases, redis, etc.

### Resource Limits

Add resource limits to docker-compose.yml:
```yaml
services:
  decent-cloud-claude:
    deploy:
      resources:
        limits:
          cpus: '2.0'
          memory: 4G
        reservations:
          cpus: '1.0'
          memory: 2G
```

This Docker setup provides a safe, reproducible environment for running Claude Code with full project access while maintaining security through container isolation.
