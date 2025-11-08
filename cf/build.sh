#!/bin/bash

# Build script for Decent Cloud containers with Docker BuildKit caching
# Uses inline cache to speed up subsequent builds

set -e

echo "🚀 Building Decent Cloud containers with Docker BuildKit caching..."

# Enable Docker BuildKit for better caching
export DOCKER_BUILDKIT=1
export COMPOSE_DOCKER_CLI_BUILD=1

echo "📦 Building API service..."
docker compose -f cf/docker-compose.yml build api

echo "🌐 Building website service..."  
docker compose -f cf/docker-compose.yml build website

echo "✅ Build completed successfully!"
echo ""
echo "💡 Docker BuildKit caching is enabled."
echo "   Subsequent builds will be significantly faster when dependencies haven't changed."
