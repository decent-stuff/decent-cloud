#!/bin/bash

# YAGNI wrapper script for running Claude Code in a safe container
# This script provides a simple interface to run Claude Code with full project access
# while keeping your host system safe through containerization

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default values
COMPOSE_FILE="docker-compose.yml"
SERVICE_NAME="claude"
COMMAND=""
DETACH=false
REBUILD=false
SHELL_MODE=false

# Helper functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Show usage
show_help() {
    cat << EOF
Claude Code Docker Wrapper - Safe containerized Claude Code environment

USAGE:
    $0 [OPTIONS] [COMMAND]

OPTIONS:
    -h, --help          Show this help message
    -d, --detach        Run in detached mode
    -r, --(re)build     Rebuild the Docker image before running
    -s, --shell         Start a shell in the container instead of Claude Code
    -f, --file FILE     Use specific docker-compose file (default: docker-compose.yml)

EXAMPLES:
    $0                              # Start Claude Code with dangerously-skip-permissions
    $0 --rebuild                    # Rebuild image and start Claude Code
    $0 --build                      # Same as above
    $0 --shell                      # Start a bash shell in the container
    $0 "cargo test"                 # Run cargo test in the container
    $0 --detach                     # Start Claude Code in background

REQUIREMENTS:
    - Docker and Docker Compose must be installed

This wrapper provides a safe way to run Claude Code with full access to the project
while keeping your host system isolated through containerization.
EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -d|--detach)
            DETACH=true
            shift
            ;;
        -r|--rebuild|--build)
            REBUILD=true
            shift
            ;;
        -s|--shell)
            SHELL_MODE=true
            shift
            ;;
        -f|--file)
            COMPOSE_FILE="$2"
            shift 2
            ;;
        -*)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
        *)
            COMMAND="$*"
            break
            ;;
    esac
done

# Check requirements
check_requirements() {
    log_info "Checking requirements..."

    # Check if Docker is running
    if ! docker info >/dev/null 2>&1; then
        log_error "Docker is not running. Please start Docker daemon."
        exit 1
    fi

    # Check if docker-compose file exists
    if [[ ! -f "$COMPOSE_FILE" ]]; then
        log_error "Docker compose file not found: $COMPOSE_FILE"
        exit 1
    fi

    log_success "Requirements check passed"
}

# Build or rebuild the image
build_image() {
    # Enable BuildX for faster parallel builds
    DOCKER_BUILDKIT=1 COMPOSE_DOCKER_CLI_BUILD=1

    if [[ "$REBUILD" == "true" ]]; then
        log_info "Rebuilding Docker image with BuildKit (preserving cache where possible)..."
        DOCKER_BUILDKIT=1 COMPOSE_DOCKER_CLI_BUILD=1 docker-compose -f "$COMPOSE_FILE" build --pull
    else
        log_info "Building Docker image with BuildKit (faster parallel builds)..."
        DOCKER_BUILDKIT=1 COMPOSE_DOCKER_CLI_BUILD=1 docker-compose -f "$COMPOSE_FILE" build
    fi
}

# Run Claude Code or custom command
run_claude() {
    local docker_args=()

    # Add detach flag if requested
    if [[ "$DETACH" == "true" ]]; then
        docker_args+=("-d")
    fi

    # Set up the command based on mode
    if [[ "$SHELL_MODE" == "true" ]]; then
        log_info "Starting shell in container..."
        docker-compose -f "$COMPOSE_FILE" "${docker_args[@]}" exec "$SERVICE_NAME" bash
    elif [[ -n "$COMMAND" ]]; then
        log_info "Running command in container: $COMMAND"
        docker-compose -f "$COMPOSE_FILE" "${docker_args[@]}" exec "$SERVICE_NAME" bash -c "$COMMAND"
    else
        log_info "Starting Claude Code with dangerously-skip-permissions..."
        log_info "Container provides isolation while giving Claude full project access"
        log_warning "Press Ctrl+D to exit Claude Code"

        # Use docker-compose run for interactive session instead of up
        docker-compose -f "$COMPOSE_FILE" "${docker_args[@]}" run --rm "$SERVICE_NAME" claude --dangerously-skip-permissions
    fi
}

# Cleanup function
cleanup() {
    if [[ "$DETACH" == "true" ]]; then
        log_info "Stopping detached container..."
        docker-compose -f "$COMPOSE_FILE" down
    fi
}

# Set up signal handlers
trap cleanup EXIT INT TERM

# Main execution
main() {
    check_requirements
    build_image
    run_claude
}

# Run main function
main "$@"
