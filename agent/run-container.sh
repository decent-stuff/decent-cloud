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

# Find repository root (directory containing .git)
find_repo_root() {
    local current_dir="$(pwd)"
    while [[ "$current_dir" != "/" ]]; do
        if [[ -d "$current_dir/.git" ]]; then
            echo "$current_dir"
            return 0
        fi
        current_dir="$(dirname "$current_dir")"
    done
    log_error "Repository root not found (no .git directory found)"
    exit 1
}

# Default values
REPO_ROOT="$(find_repo_root)"
COMPOSE_FILE="$REPO_ROOT/agent/docker-compose.yml"
SERVICE_NAME="agent"
TOOL=""
COMMAND=""
DETACH=false
REBUILD=true

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
Claude Code and Happy Coder Docker Wrapper - Safe containerized environment

USAGE:
    $0 [OPTIONS] TOOL [COMMAND]

OPTIONS:
    -h, --help          Show this help message
    -d, --detach        Run in detached mode
        --no-build      Skip building Docker image before running, run the existing image if available, without checking
    -f, --file FILE     Use specific docker-compose file (default: <repo-root>/agent/docker-compose.yml)

TOOLS:
    claude              Run Claude Code (with dangerously-skip-permissions)
    happy               Run Happy Coder
    bash OR shell       Run a plain bash shell

EXAMPLES:
    $0 claude                        # Start Claude Code with dangerously-skip-permissions
    $0 happy                         # Start Happy Coder
    $0 bash                          # Start a bash shell
    $0 claude --no-build             # Run Claude Code without rebuilding
    $0 claude "cargo test"           # Run cargo test in the container with Claude Code
    $0 happy --detach                # Start Happy Coder in background

REQUIREMENTS:
    - Docker and Docker Compose must be installed
    - Must specify either 'claude' or 'happy' as the tool

This wrapper provides a safe way to run Claude Code or Happy Coder with full access to the project
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
        --no-build)
            REBUILD=false
            shift
            ;;
        -f|--file)
            COMPOSE_FILE="$2"
            shift 2
            ;;
        claude|happy|shell|bash)
            if [[ -z "$TOOL" ]]; then
                TOOL="$1"
                shift
            else
                log_error "Multiple tools specified: $TOOL and $1"
                show_help
                exit 1
            fi
            ;;
        -*)
            log_error "Unknown option: $1"
            show_help
            exit 1
            ;;
        *)
            if [[ -z "$TOOL" ]]; then
                log_error "Must specify a tool: claude or happy"
                show_help
                exit 1
            fi
            COMMAND="$*"
            break
            ;;
    esac
done

# Check requirements
check_requirements() {
    log_info "Checking requirements..."

    # Check if tool is specified
    if [[ -z "$TOOL" ]]; then
        log_error "Must specify a tool: claude or happy"
        show_help
        exit 1
    fi

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
    if [[ "$REBUILD" == "true" ]]; then
        log_info "Rebuilding Docker image with BuildKit..."
        # Enable BuildX for faster parallel builds
        DOCKER_BUILDKIT=1 COMPOSE_DOCKER_CLI_BUILD=1 docker-compose -f "$COMPOSE_FILE" build --pull
    fi
}

# Run the specified tool or custom command
run_tool() {
    local docker_args=()
    local tool_command=""

    # Add detach flag if requested
    if [[ "$DETACH" == "true" ]]; then
        docker_args+=("-d")
    fi

    # Set up the command based on mode and tool
    if [[ "$SHELL_MODE" == "true" ]]; then
        log_info "Starting shell in container..."
        docker-compose -f "$COMPOSE_FILE" "${docker_args[@]}" exec "$SERVICE_NAME" bash
    elif [[ -n "$COMMAND" ]]; then
        log_info "Running command in container: $COMMAND"
        docker-compose -f "$COMPOSE_FILE" "${docker_args[@]}" exec "$SERVICE_NAME" bash -c "$COMMAND"
    else
        # Set up tool-specific command
        case "$TOOL" in
            claude)
                tool_command="claude --dangerously-skip-permissions"
                log_info "Starting Claude Code..."
                ;;
            happy)
                tool_command="happy --yolo"
                log_info "Starting Happy Coder..."
                ;;
            bash|shell)
                tool_command="bash"
                log_info "Starting bash shell..."
                ;;
        esac

        log_info "Container provides isolation while giving $TOOL full project access"
        log_warning "Press Ctrl+D to exit $TOOL"

        # Use docker-compose run for interactive session instead of up
        docker-compose -f "$COMPOSE_FILE" "${docker_args[@]}" run --rm "$SERVICE_NAME" $tool_command
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
    run_tool
}

# Run main function
main "$@"
