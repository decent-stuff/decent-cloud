# CI/CD Setup

## Overview

This document explains the CI/CD pipeline configuration for Decent Cloud, including PostgreSQL database setup for automated testing.

## GitHub Actions Workflow

The project uses GitHub Actions for continuous integration. The workflow is defined in `.github/workflows/build-and-test.yml`.

### PostgreSQL Service Configuration

The CI workflow automatically starts **PostgreSQL 16** as a service container:

```yaml
services:
  postgres:
    image: postgres:16
    env:
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
      POSTGRES_DB: test
    ports:
      - 5432:5432
    options: >-
      --health-cmd pg_isready
      --health-interval 10s
      --health-timeout 5s
      --health-retries 5
```

### Database Connection Strings

The CI environment provides the following environment variables to the test container:

- `DATABASE_URL=postgres://test:test@localhost:5432/test`
- `TEST_DATABASE_URL=postgres://test:test@localhost:5432/test`

### Network Configuration

The test container uses `--network host` to connect to the PostgreSQL service via `localhost:5432`.

## CI Docker Image

The project uses a custom CI Docker image defined in `.github/container/Dockerfile`.

### Base Image

```dockerfile
FROM rust:1.91
```

### PostgreSQL Client Tools

The CI image includes `postgresql-client` package for database interaction during tests:

```dockerfile
RUN apt update && \
    apt install -y libunwind-dev curl libssl-dev pkg-config gzip build-essential ca-certificates less jq clang postgresql-client
```

This provides tools like `psql` for debugging and manual database operations.

### Building the CI Image Locally

To replicate the CI environment locally:

```bash
docker build -f .github/container/Dockerfile --tag ghcr.io/decent-stuff/decent-cloud/ci-image:latest .
```

### Pushing to Registry

```bash
docker push ghcr.io/decent-stuff/decent-cloud/ci-image:latest
```

If push fails with "denied", generate a new GitHub token from [GitHub settings/tokens](https://github.com/settings/tokens?page=1) with `read:packages` and `write:packages` scopes.

## Running Tests Locally with PostgreSQL

To mimic the CI environment locally:

### Option 1: Using Docker Compose

```bash
# Start PostgreSQL
docker compose up -d db

# Run tests with CI environment variables
export DATABASE_URL="postgres://decent_cloud:decent_cloud@localhost:5432/decent_cloud"
export TEST_DATABASE_URL="postgres://decent_cloud:decent_cloud@localhost:5432/decent_cloud_test"
cargo make
```

### Option 2: Using the CI Container

```bash
# Build the CI image
docker build -f .github/container/Dockerfile --tag decent-cloud-ci:latest .

# Start PostgreSQL separately
docker compose up -d db

# Run tests in CI container
docker run --rm \
  -v $(pwd):/workspace \
  -w /workspace \
  -e DATABASE_URL="postgres://test:test@host.docker.internal:5432/test" \
  -e TEST_DATABASE_URL="postgres://test:test@host.docker.internal:5432/test" \
  decent-cloud-ci:latest \
  cargo make
```

## Database Migrations in CI

The CI pipeline runs all migrations from the `api/migrations_pg/` directory using `sqlx-cli`:

```bash
sqlx database create
sqlx migrate run
```

Migrations are verified to ensure they apply cleanly to a fresh PostgreSQL 16 database.

## sqlx Offline Mode

The project uses sqlx's offline mode for compile-time verification of SQL queries. Pre-generated query metadata is stored in `.sqlx/*.json` files.

### Updating sqlx Metadata

When adding or modifying database queries:

```bash
# With a running database
cargo install sqlx-cli
sqlx database setup
cargo check
```

This updates `.sqlx/*.json` files with query metadata for CI to use.

## Troubleshooting CI Failures

### PostgreSQL Connection Issues

If tests fail with "connection refused" to PostgreSQL:

1. Verify the health check is passing: check logs for `pg_isready` output
2. Ensure `--network host` is set in the docker run command
3. Verify port 5432 is not already in use

### sqlx Query Verification Failures

If sqlx complains about outdated `.sqlx/*.json` files:

```bash
# Locally regenerate with correct database URL
DATABASE_URL="postgres://test:test@localhost:5432/test" cargo sqlx prepare
```

Commit the updated `.sqlx/*.json` files.

### Migration Failures

If migrations fail in CI but work locally:

1. Verify PostgreSQL version: CI uses PostgreSQL 16
2. Check migration order: migrations must be numbered sequentially
3. Ensure no PostgreSQL-specific syntax issues (check `sqlx` version)

## CI Pipeline Stages

The GitHub Actions workflow includes:

1. **Free disk space**: Removes unnecessary tools from GitHub Actions runner
2. **Checkout code**: Clones the repository
3. **Login to registry**: Authenticates with GitHub Container Registry
4. **Build and test**: Runs `cargo make` in CI container with PostgreSQL

All stages must pass for a merge to `main`.

## Version Requirements

- **PostgreSQL**: 16 (same as production)
- **Rust**: 1.91 (via rust:1.91 base image)
- **sqlx-cli**: Latest (installed in CI image)
- **docker compose**: 2.x+ (for local testing)

## Security Considerations

- CI uses `test` credentials for PostgreSQL (not production credentials)
- No secrets are exposed in logs (environment variables are masked)
- Database connection strings use localhost (not accessible from outside CI)
- CI image is built from official `rust:1.91` image (trusted base)

## Related Documentation

- [Development Setup](./development.md) - Local development environment setup
- [Migration Guide](./migrations.md) - Database migration procedures
- [Environment Variables](../api/.env.example) - Complete configuration reference
