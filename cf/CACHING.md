# Rust Build Caching

This directory contains Docker build optimizations for faster Rust compilation using Docker BuildKit.

## Problem

Without caching, Docker builds Rust applications from scratch every time, re-downloading and re-compiling all dependencies. This can take several minutes per build.

## Solution

We use Docker BuildKit with inline caching to speed up builds:

- **BuildKit**: Docker's enhanced build system with improved layer caching
- **Inline Cache**: Cache metadata is embedded directly in the image for faster rebuilds
- **Layer Reuse**: Unchanged Docker layers are reused across builds

## Usage

### Building with Cache

Docker BuildKit caching is automatically enabled in all builds:

```bash
# Using the build script (recommended)
./cf/build.sh

# Manual build with BuildKit
DOCKER_BUILDKIT=1 COMPOSE_DOCKER_CLI_BUILD=1 docker compose -f cf/docker-compose.yml build

# Development deployment
./cf/deploy.py deploy dev

# Production deployment
./cf/deploy.py deploy prod
```

## Benefits

- **First Build**: Same time as normal (downloads dependencies)
- **Subsequent Builds**: 2-10x faster when dependencies unchanged
- **Development**: Rapid iteration when only application code changes
- **No Manual Setup**: Works automatically with Docker BuildKit
- **Shared Cache**: Cache works across different machines with image registry

## Cache Management

To clear cache and rebuild from scratch:

```bash
# Remove Docker build cache
docker builder prune -f

# Or rebuild without cache
docker compose -f cf/docker-compose.yml build --no-cache
```

## Technical Details

- **BUILDKIT_INLINE_CACHE**: Build argument enables inline cache metadata in images
- **Layer Caching**: Unchanged layers are reused automatically
- **Cache Export**: Cache metadata is stored in the built image
- **Docker BuildKit**: Required for advanced caching features (enabled by default in modern Docker)

## Verification

You can verify caching is working by looking for "CACHED" steps in build output:

```bash
# First build (will show many steps running)
./cf/build.sh

# Second build (will show many "CACHED" steps)
./cf/build.sh
```
