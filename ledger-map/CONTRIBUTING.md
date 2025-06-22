# Contributing to LedgerMap

Thank you for your interest in contributing to LedgerMap! This document provides guidelines and instructions for contributing to the project.

## Development Setup

1. **Prerequisites**

   - Rust (latest stable version)
   - Node.js (v16 or higher)
   - wasm-pack (for WebAssembly builds)
   - A C++ build environment (for native builds)

2. **Clone the Repository**

   ```bash
   git clone https://github.com/decent-cloud/ledger-map.git
   cd ledger-map
   ```

3. **Install Dependencies**

   ```bash
   # Install Rust dependencies
   cargo check

   # Install Node.js dependencies
   npm install
   ```

## Building

### Rust Library

```bash
# Build the Rust library
cargo build --release

# Build with specific features
cargo build --release --features browser
cargo build --release --features ic
```

### WebAssembly Package

```bash
# Build the WebAssembly package
npm run build

# Or using wasm-pack directly
wasm-pack build --target web
```

## Testing

LedgerMap has several test suites:

### Rust Tests

```bash
# Run Rust unit and integration tests
cargo test

# Run tests with specific features
cargo test --features browser
cargo test --features ic
```

### TypeScript/Node.js Tests

```bash
# Run Node.js tests
npm run test
```

### Browser Tests

```bash
# Run browser-specific tests
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' wasm-pack test --chrome --features browser
```

## Code Style

### Rust Code Style

- Follow the [Rust Style Guide](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for code formatting
- Run `cargo clippy` and address any warnings
- Document public APIs with rustdoc comments

### TypeScript Code Style

- Follow the project's TSConfig settings
- Use ESLint for code linting
- Document public APIs with JSDoc comments

## Pull Request Process

1. **Fork the Repository**

   - Create a fork of the repository
   - Clone your fork locally

2. **Create a Branch**

   - Create a new branch for your changes
   - Use a descriptive name (e.g., `feature/new-storage-backend` or `fix/memory-leak`)

3. **Make Your Changes**

   - Write clear, concise commit messages
   - Keep commits focused and atomic
   - Add tests for new functionality
   - Update documentation as needed

4. **Test Your Changes**

   - Run all test suites
   - Ensure no existing tests are broken
   - Add new tests for your changes

5. **Submit a Pull Request**

   - Push your changes to your fork
   - Create a pull request against the main repository
   - Fill out the pull request template
   - Link any related issues

6. **Code Review**
   - Address any review feedback
   - Make requested changes
   - Keep the PR up-to-date with the main branch

## Release Process

1. **Version Bump**

   - Update version in `Cargo.toml`
   - Update version in `package.json`
   - Update CHANGELOG.md

2. **Documentation**
   - Ensure README.md is up to date
   - Update API documentation if needed

## Questions or Problems?

If you have questions or run into problems, please:

1. Check existing issues
2. Create a new issue with a clear description
3. Provide reproduction steps if applicable

## License

By contributing to LedgerMap, you agree that your contributions will be licensed under its MIT OR Apache-2.0 license.
