# Contributing to Decent Cloud

👋 **See [Development Guide](docs/development.md) for comprehensive contribution guidelines.**

## Quick Links

- 📚 [Development Guide](docs/development.md) - Complete setup, testing, and contribution workflow
- 💬 [Discussions](https://github.com/orgs/decent-stuff/discussions) - Community discussions
- 🐛 [Issue Tracker](https://github.com/decent-stuff/decent-cloud/issues) - Report bugs or request features

## Prerequisites

Before contributing, ensure your development environment includes:

- **PostgreSQL 14+** - Required database for local development
  - Automatically started via `docker compose up -d postgres` (see Development Guide)
  - Connection URL: `postgres://test:test@localhost:5432/test`
- **Rust toolchain** - 1.70 or later
- **Node.js 18+** - For frontend development
- **Docker and Docker Compose** - Required for local PostgreSQL instance

See the [Development Guide](docs/development.md) for detailed setup instructions.

## Code Standards

Follow [Conventional Commits](https://www.conventionalcommits.org/):
```
type(scope): description
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

## Help

- 📝 [Open an Issue](https://github.com/decent-stuff/decent-cloud/issues)
- 💬 [Join Discussions](https://github.com/orgs/decent-stuff/discussions)