# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Primary Reference

@AGENTS.md contains the detailed development rules and project memory. This file serves as a quick reference for common workflows.

## Project Structure

Multi-repository project - see [Development Guide](docs/development.md#project-structure) for details.

## Key Development Commands

### Testing and Building
```bash
# Run all tests (unit + canister tests)
cargo make

# Build binary only
cargo build --release --bin dc

# Run unit tests only
cargo test
```

### Website Development

See [Development Guide](docs/development.md#building-the-website) for:
- Standard development (`npm run dev`)
- WASM auto-rebuild (`npm run dev:watch`)
- Production build (`npm run build`)

### Documentation

See [Development Guide](docs/development.md#documentation) for:
- Building whitepaper (Python venv required)
- API docs with `cargo doc --no-deps --open`

## Documentation Guidelines

When working with this project:

- **Update existing AGENTS.md files** when you learn implementation details, debugging insights, or architectural patterns
- **Create new AGENTS.md files** in relevant directories for areas lacking documentation
- **Add valuable insights** such as common pitfalls, debugging techniques, dependency relationships

This helps build a comprehensive knowledge base for the codebase over time.

## Quality Standards

Follow the detailed development rules and quality standards specified in @AGENTS.md, including:
- Running `cargo make` after changes to ensure tests pass
- Only committing changes when `cargo make` exits cleanly with no errors
- Adding comprehensive tests for new functionality

## Getting Help

- Reference [Development Guide](docs/development.md) for detailed setup instructions
- Use project AGENTS.md files for component-specific guidance
- Check existing documentation before making assumptions
