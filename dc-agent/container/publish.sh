#!/usr/bin/env bash
set -euo pipefail

REGISTRY="${1:-ghcr.io}"
ORG="${2:-decent-stuff}"
IMAGE_NAME="dc-agent-ssh"

LOCAL_TAG="${IMAGE_NAME}:latest"
REMOTE_TAG="${REGISTRY}/${ORG}/${IMAGE_NAME}:latest"

echo "Publishing ${LOCAL_TAG} -> ${REMOTE_TAG}"
docker tag "${LOCAL_TAG}" "${REMOTE_TAG}"
docker push "${REMOTE_TAG}"

echo "Done: ${REMOTE_TAG}"
