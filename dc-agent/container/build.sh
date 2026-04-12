#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME="${1:-dc-agent-ssh}"
IMAGE_TAG="${2:-latest}"
FULL_TAG="${IMAGE_NAME}:${IMAGE_TAG}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
DOCKERFILE="${SCRIPT_DIR}/Dockerfile"

echo "Building ${FULL_TAG} ..."
docker build -t "${FULL_TAG}" -f "${DOCKERFILE}" "${SCRIPT_DIR}"

echo ""
echo "Verifying sshd starts ..."
CONTAINER_ID=$(docker run -d --rm "${FULL_TAG}")
sleep 2

if docker exec "${CONTAINER_ID}" pgrep -x sshd > /dev/null 2>&1; then
    echo "OK: sshd is running inside ${CONTAINER_ID}"
    docker stop "${CONTAINER_ID}" > /dev/null
    echo "Container stopped."
else
    echo "FAIL: sshd not found"
    docker logs "${CONTAINER_ID}" 2>&1 || true
    docker stop "${CONTAINER_ID}" > /dev/null 2>&1 || true
    exit 1
fi

echo ""
echo "Image size:"
docker images "${FULL_TAG}" --format "{{.Repository}}:{{.Tag}}  {{.Size}}"
