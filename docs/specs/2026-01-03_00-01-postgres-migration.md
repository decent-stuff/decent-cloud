# PostgreSQL Migration: SQLite to PostgreSQL

**Status:** Draft
**Created:** 2026-01-03
**Author:** Claude Code

## Summary

Migrate the entire codebase from SQLite to PostgreSQL database engine. Fix all warnings and errors from `cargo make` that resulted from the incomplete migration. Add automated test PostgreSQL instance management via cargo make (following patterns from `agent/`).

## Problem Statement

The codebase has begun migrating from SQLite to PostgreSQL but the migration is fundamentally broken:

1. **Build failures**: `cargo make` produces warnings and errors
2. **No automated testing**: Test postgres instance not available via cargo make
3. **Incomplete migration**: Database code likely has sqlite-specific patterns that don't work with postgres
4. **Missing infrastructure**: No docker-compose or automated postgres setup for development/testing

This violates the project's fail-fast principle - the build system should not be in a broken state.

## Requirements

### Must-have

- [ ] All `cargo make` commands complete without errors or warnings
- [ ] PostgreSQL test instance starts via `cargo make db-start` (or similar)
- [ ] PostgreSQL test instance stops via `cargo make db-stop` (or similar)
- [ ] Database migrations work correctly with PostgreSQL
- [ ] All existing tests pass against PostgreSQL
- [ ] Follow existing patterns from `agent/` directory for test DB management
- [ ] Clear error messages if postgres dependencies are missing
- [ ] Documentation in README or developer docs on how to set up postgres locally

### Nice-to-have

- [ ] Docker Compose setup for postgres (if not already present)
- [ ] Health check endpoint for test postgres instance
- [ ] Automated schema validation
- [ ] Migration tooling (e.g., sqlx-cli or refinery)
- [ ] CI/CD pipeline updates for postgres

## Technical Design

### Discovery Phase

Before implementing, thoroughly investigate:

1. **Current state analysis**:
   - Run `cargo make` and capture all warnings/errors
   - Identify all sqlite-specific code patterns
   - Find all database connection setup code
   - Document what works and what's broken

2. **Reference implementation**:
   - Examine `agent/` directory for postgres test setup
   - Identify patterns used for automated test DB
   - Understand connection string management
   - Check migration strategy

3. **Dependency audit**:
   - Check `Cargo.toml` for sqlite vs postgres dependencies
   - Identify feature flags for database backends
   - Verify sqlx features (sqlite vs postgres)

### Implementation Plan

#### Phase 1: Investigation and Documentation (DO THIS FIRST)

1. Run `cargo make` and document all errors:
   ```bash
   cargo make 2>&1 | tee logs/cargo-make-errors.log
   ```

2. Catalog all database code:
   ```bash
   # Find all sqlite references
   rg -i "sqlite" --type rust
   rg -i "rusqlite" --type rust
   rg -i "sqlx.*sqlite" --type rust

   # Find database connection code
   rg "SqlitePool" --type rust
   rg "PgPool" --type rust
   rg "connect.*database" --type rust
   ```

3. Examine agent directory postgres setup:
   ```bash
   # Find test DB setup patterns
   rg "postgres" agent/ --type rust
   rg "docker" agent/ --type rust
   rg "test.*db" agent/ --type rust -i
   ```

4. Create detailed inventory in this spec document:
   - List all files that need changes
   - List all errors with root cause analysis
   - Document sqlite→postgres pattern mapping

#### Phase 2: Database Setup Automation

Add to `Makefile.toml`:

```toml
[tasks.db-start]
description = "Start PostgreSQL test instance"
command = "docker"
args = ["compose", "-f", "docker-compose.test.yml", "up", "-d", "postgres"]

[tasks.db-stop]
description = "Stop PostgreSQL test instance"
command = "docker"
args = ["compose", "-f", "docker-compose.test.yml", "down"]

[tasks.db-reset]
description = "Reset PostgreSQL test database"
dependencies = ["db-stop"]
command = "docker"
args = ["compose", "-f", "docker-compose.test.yml", "up", "-d", "postgres"]
```

If `agent/` already has this, reuse the exact same patterns.

#### Phase 3: Code Migration

Migration checklist (to be completed during Phase 1 discovery):

- [ ] Replace `rusqlite` with `sqlx::postgres`
- [ ] Update connection string format (sqlite: → postgres://)
- [ ] Replace sqlite-specific SQL with postgres-compatible SQL
- [ ] Update transaction patterns (BEGIN/COMMIT vs BEGIN TRANSACTION)
- [ ] Fix data type incompatibilities (e.g., BOOLEAN vs INTEGER)
- [ ] Update auto-increment/sequence patterns
- [ ] Replace `last_insert_rowid()` with `RETURNING id`
- [ ] Fix LIMIT/OFFSET syntax differences (if any)
- [ ] Update FTS (Full-Text Search) if used
- [ ] Replace ATTACH DATABASE usage (if any)
- [ ] Update PRAGMA statements (postgres doesn't support)
- [ ] Fix datetime handling differences

#### Phase 4: Testing

- [ ] Run `cargo clippy --tests` - must pass
- [ ] Run `cargo nextest run` - all tests pass
- [ ] Manual test: start postgres, run migrations, verify data
- [ ] Test rollback (can we migrate back to sqlite if needed?)

### Known SQLite → PostgreSQL Gotchas

**Data Types:**
- SQLite: `INTEGER` → PostgreSQL: `INTEGER` or `BIGINT`
- SQLite: `BOOLEAN` (stored as 0/1) → PostgreSQL: `BOOLEAN` (true/false)
- SQLite: `DATETIME` (strings or Unix timestamps) → PostgreSQL: `TIMESTAMP` or `TIMESTAMPTZ`

**SQL Dialect:**
- SQLite: `SELECT last_insert_rowid()` → PostgreSQL: `INSERT ... RETURNING id`
- SQLite: `INSERT OR IGNORE` → PostgreSQL: `INSERT ... ON CONFLICT DO NOTHING`
- SQLite: `CREATE TABLE IF NOT EXISTS` → PostgreSQL: Same (works)
- SQLite: `PRAGMA` statements → PostgreSQL: Not supported, remove
- SQLite: `ATTACH DATABASE` → PostgreSQL: Not supported, use schemas

**Connection Handling:**
- SQLite: File path (`sqlite://path/to/db.sqlite3`)
- PostgreSQL: Connection string (`postgresql://user:pass@host/db`)
- SQLite: Single-file, no server → PostgreSQL: Server process required

**Transactions:**
- SQLite: `BEGIN TRANSACTION` → PostgreSQL: `BEGIN` (both work, but be consistent)
- SQLite: Immediate locking → PostgreSQL: MVCC (different concurrency model)

## Implementation Notes

**IMPORTANT**: Do NOT make unnecessary changes elsewhere in the codebase. Focus ONLY on:

1. Database engine migration
2. Fixing cargo make warnings/errors caused by DB migration
3. Test postgres instance automation

**DO NOT** refactor unrelated code, "improve" other areas, or add new features. Stay minimal and focused.

## Verification Checklist

Before marking this spec complete:

- [ ] `cargo make` runs without errors or warnings
- [ ] `cargo make db-start` starts postgres test instance
- [ ] `cargo make db-stop` stops postgres test instance
- [ ] All tests pass: `cargo nextest run`
- [ ] Linter passes: `cargo clippy --tests`
- [ ] At least one manual migration test performed
- [ ] Documentation updated (README or developer docs)
- [ ] No sqlite references remain in production code
- [ ] Agent directory patterns reused where applicable

## Open Questions

To be answered during Phase 1 discovery:

1. What is the current state of the migration? Partially complete? Not started?
2. What specific errors does `cargo make` produce?
3. What patterns does `agent/` use for test postgres?
4. Are there migration scripts already written?
5. Is docker-compose already configured for postgres?
6. What is the migration strategy for existing data (if any)?
7. Are there environment variables or config files that need updating?
