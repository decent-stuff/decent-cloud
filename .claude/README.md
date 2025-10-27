# Claude Code Sandbox Configuration

This directory contains the sandbox configuration for the **decent-cloud** repository, enabling Claude Code to run the full development and test cycle in an isolated environment.

## Overview

The decent-cloud project is a complex multi-language repository that includes:
- **Rust** components for Internet Computer canister development
- **Python** simulation tools
- **JavaScript/TypeScript** Next.js frontend
- **Internet Computer SDK** integration

## Sandbox Configuration Files

### `sandbox.json`
Main configuration file that defines:
- Container environment based on `rust:latest`
- Required system packages and tools
- Environment variables for IC development
- Build commands for each language
- Resource limits and mount points

### `Dockerfile.sandbox`
Custom Docker image that includes:
- Rust toolchain with WebAssembly target
- Python 3.10+ with UV
- Node.js 22 with npm
- Internet Computer SDK (dfx)
- Pocket IC for local canister testing
- All necessary build tools and dependencies

### `sandbox-init.sh`
Initialization script that:
- Sets up dfx environment
- Installs language-specific dependencies
- Verifies all tools are working correctly
- Provides usage information

## Key Features

### 🔧 Multi-Language Support
- **Rust**: Stable toolchain with `wasm32-unknown-unknown` target
- **Python**: UV-managed dependencies with testing tools
- **JavaScript**: Node.js 22 with TypeScript and modern tooling

### 🌐 Internet Computer Development
- **dfx SDK** version 0.24.2 for IC development
- **Pocket IC** for local canister testing
- **Candid** support for interface definition
- **WASM** compilation and testing

### 🧪 Complete Testing Environment
- **Rust**: `cargo-make` orchestration with `cargo-nextest`
- **Python**: `pytest` with coverage and linting
- **JavaScript**: Jest for unit and integration tests
- **Integration**: End-to-end testing across all components

### 📦 Build System Integration
- **Pants** for build orchestration
- **Bazel** for hermetic builds
- **Cargo** for Rust workspace management
- **UV** for Python dependency management
- **npm** for JavaScript packages

## Usage

### Automatic Setup
When you open the repository in Claude Code with sandbox enabled, it will:

1. Build the custom Docker image with all dependencies
2. Initialize the development environment
3. Install project dependencies
4. Verify all tools are working

### Manual Commands
Once inside the sandbox, you can run:

```bash
# Initialize the environment
./.claude/sandbox-init.sh

# Build and test Rust components
cargo make

# Run Python tests
cd simulator && uv run pytest

# Build and test the website
cd website && npm run build && npm test

# Run the full test suite
cargo make && cd simulator && uv run pytest && cd website && npm test
```

### Development Workflow
1. **Make changes** to any component
2. **Run tests** for the affected language
3. **Build** the entire project to verify integration
4. **Deploy** canisters locally for testing with Pocket IC

## Environment Variables

- `XDG_DATA_HOME`: Directory for dfx cache and configuration
- `POCKET_IC_BIN`: Path to Pocket IC binary
- `RUST_TOOLCHAIN`: Rust toolchain version (stable)
- `PYTHONPATH`: Python module path
- `NODE_ENV`: Node.js environment (development)

## Resource Limits

- **Memory**: 8GB (sufficient for large builds)
- **CPU**: 4 cores (parallel builds and testing)
- **Timeout**: 30 minutes (prevents hanging builds)

## Troubleshooting

### Common Issues

1. **Pocket IC not found**
   - Ensure the Docker image built successfully
   - Check that `pocket-ic` is in `/usr/local/bin/`

2. **dfx initialization fails**
   - Check `XDG_DATA_HOME` is set correctly
   - Ensure writable directory for cache

3. **Rust WASM target missing**
   - Run `rustup target add wasm32-unknown-unknown`
   - Verify toolchain installation

4. **UV dependencies fail**
   - Check Python version compatibility
   - Ensure virtual environment is properly created

### Debug Commands

```bash
# Check all tool versions
rustc --version && python3 --version && node --version && dfx --version

# Verify Pocket IC
pocket-ic --help

# Test Rust compilation
cargo check

# Test Python environment
uv run python --version

# Test Node.js environment
cd website && npm run build
```

## Maintenance

### Updating Dependencies
- Modify `Dockerfile.sandbox` for system dependencies
- Update `sandbox.json` for tool versions
- Rebuild the sandbox image when changes are made

### Performance Optimization
- The sandbox includes mount points for cargo, npm, and UV caches
- Build artifacts are preserved between sessions
- Resource limits can be adjusted based on project needs

## Security

- The sandbox runs without privileged access
- Network access is enabled for package downloads
- Docker socket is not mounted (cannot launch containers)
- All operations are contained within the sandbox environment

This configuration provides a complete, isolated development environment that matches the project's CI/CD pipeline while enabling interactive development and testing.