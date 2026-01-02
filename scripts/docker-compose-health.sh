#!/usr/bin/env bash
# docker-compose-health.sh - Wait for a Docker Compose service to become healthy
#
# Usage: docker-compose-health.sh <service-name> [timeout_seconds]
#
# Arguments:
#   service-name     Name of the Docker Compose service (e.g., "postgres")
#   timeout_seconds  Optional timeout in seconds (default: 60)
#
# Returns:
#   0 if service becomes healthy within timeout
#   1 if timeout is reached or service fails to start
#
# Example:
#   docker-compose-health.sh postgres 30

set -eEuo pipefail

# Script arguments
SERVICE_NAME="${1:-}"
TIMEOUT="${2:-60}"

# Validate arguments
if [[ -z "$SERVICE_NAME" ]]; then
    echo "✗ Error: Service name is required"
    echo "Usage: $0 <service-name> [timeout_seconds]"
    echo "Example: $0 postgres 30"
    exit 1
fi

if ! [[ "$TIMEOUT" =~ ^[0-9]+$ ]]; then
    echo "✗ Error: Timeout must be a positive integer"
    exit 1
fi

# Check if service is running
if ! docker compose ps "$SERVICE_NAME" --format '{{.State}}' 2>/dev/null | grep -q "running"; then
    echo "✗ Service '$SERVICE_NAME' is not running"
    echo "Start it with: docker compose up -d $SERVICE_NAME"
    exit 1
fi

echo "Waiting for '$SERVICE_NAME' to be healthy (timeout: ${TIMEOUT}s)..."

elapsed=0
while [[ $elapsed -lt $TIMEOUT ]]; do
    # Check health using docker compose exec
    # For postgres: pg_isready -U test -d test
    # For other services: adjust health check command accordingly
    case "$SERVICE_NAME" in
        postgres)
            if docker compose exec -T postgres pg_isready -U test -d test &>/dev/null; then
                echo "✓ '$SERVICE_NAME' is ready"
                exit 0
            fi
            ;;
        *)
            # Generic health check: container health status
            health_status=$(docker compose ps "$SERVICE_NAME" --format '{{.Health}}' 2>/dev/null || echo "")
            if [[ "$health_status" == "healthy" ]]; then
                echo "✓ '$SERVICE_NAME' is healthy"
                exit 0
            fi
            ;;
    esac

    sleep 1
    elapsed=$((elapsed + 1))
done

echo "✗ '$SERVICE_NAME' failed to become healthy within ${TIMEOUT}s"
echo "Check logs with: docker compose logs $SERVICE_NAME"
echo "Check status with: docker compose ps $SERVICE_NAME"
exit 1
