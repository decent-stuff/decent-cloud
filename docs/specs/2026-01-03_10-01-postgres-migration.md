# PostgreSQL Migration: SQLite to PostgreSQL

**Status:** In Progress
**Created:** 2026-01-03
**Updated:** 2026-01-03
**Author:** Claude Code

## Summary

Migrate the entire codebase from SQLite to PostgreSQL for production readiness. Fix all cargo make warnings and errors, ensure complete schema migration with flattened design, implement automatic test postgres instance provisioning, and achieve solid test coverage for all database operations.

**Current State (2026-01-03 14:17 UTC):**
- Test run output available in `logs/nextest-run.txt`
- **2 test failures** (both `test_migrations_via_database_new`):
  - `api database::migration_tests::test_migrations_via_database_new`
  - `api::bin/api-server database::migration_tests::test_migrations_via_database_new`
- **Root cause:** Test tries to connect to postgres but fails with "failed to lookup address information: Name or service not known"
- **Immediate action needed:** Set up automatic postgres test instance provisioning for cargo make/cargo nextest

## Problem Statement

The codebase currently uses SQLite, which is insufficient for production deployment. Many things are broken due to the incomplete migration:

1. **Build failures**: `cargo make` fails with warnings and errors
2. **Missing test infrastructure**: No automatic postgres test instance setup
3. **Incomplete schema migration**: Original SQLite schema not fully migrated to postgres
4. **Missing test coverage**: Database operations lack comprehensive tests
5. **Schema flattening needed**: Complex sqlite schema needs to be flattened for postgres

## Requirements

### Must-have

- [ ] All `cargo make` commands pass without warnings or errors
- [ ] Automatic postgres test instance provisioning via `cargo make` (check agent/ for reference)
- [ ] Complete SQLite to PostgreSQL schema migration (flattened design)
- [ ] All existing data models work with postgres
- [ ] Comprehensive test coverage for all database operations
- [ ] Production-ready configuration for postgres connection
- [ ] Migration scripts from existing sqlite databases
- [ ] All warnings and errors resolved in clippy and tests

### Nice-to-have

- [ ] Connection pooling configuration
- [ ] Performance benchmarks comparing sqlite vs postgres
- [ ] Automatic migration on startup (with opt-out)
- [ ] Database health check endpoints
- [ ] Monitoring and observability for database operations

## Technical Design

### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    PostgreSQL Migration                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. Test Infrastructure                                       │
│     ├── docker-compose.yml for test postgres                 │
│     ├── cargo make tasks for provisioning                    │
│     └── automatic lifecycle management                       │
│                                                               │
│  2. Schema Migration                                          │
│     ├── analyze existing sqlite schema                       │
│     ├── flatten schema for postgres                          │
│     ├── create migration scripts                             │
│     └── data migration tooling                               │
│                                                               │
│  3. Code Updates                                              │
│     ├── update database connection layer                     │
│     ├── fix all sql queries for postgres syntax             │
│     ├── update ORM/query builder if applicable               │
│     └── handle postgres-specific features                   │
│                                                               │
│  4. Test Coverage                                             │
│     ├── unit tests for all db operations                     │
│     ├── integration tests with test postgres                 │
│     ├── migration tests                                      │
│     └── performance tests                                    │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

### Test Postgres Instance

Reference implementation exists in `agent/` directory. Use similar approach:

```toml
# Makefile.toml
[tasks.postgres-up]
command = "docker-compose"
args = ["-f", "docker-compose.test.yml", "up", "-d", "postgres"]

[tasks.postgres-down]
command = "docker-compose"
args = ["-f", "docker-compose.test.yml", "down"]

[tasks.postgres-logs]
command = "docker-compose"
args = ["-f", "docker-compose.test.yml", "logs", "postgres"]
```

```yaml
# docker-compose.test.yml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: decent_cloud_test
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U test"]
      interval: 1s
      timeout: 1s
      retries: 30
```

### Schema Migration Strategy

1. **Analyze existing schema**:
   - Extract all table definitions from sqlite
   - Identify foreign keys and constraints
   - Document indexes and unique constraints

2. **Flatten schema**:
   - Denormalize where appropriate for postgres performance
   - Use proper postgres types (BIGINT, TEXT, TIMESTAMP, JSONB, etc.)
   - Add proper foreign key constraints
   - Create appropriate indexes

3. **Migration script**:
   - SQL script to create postgres schema
   - Data migration script from sqlite to postgres
   - Validation script to verify migration success

4. **Rollback strategy**:
   - Keep sqlite backup
   - Provide migration rollback script
   - Document migration process

### Database Connection Layer

Update database connection code to:

```rust
// Example pattern
pub async fn create_postgres_connection(
    url: &str,
) -> Result<PgPool> {
    let config = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(30));

    let pool = config.connect(url).await
        .context("Failed to connect to postgres")?;

    Ok(pool)
}
```

Environment variables:
- `DATABASE_URL` - postgres connection string
- `DATABASE_MAX_CONNECTIONS` - connection pool size (default: 10)
- `DATABASE_ACQUIRE_TIMEOUT_SECS` - acquire timeout (default: 30)

## Implementation Plan

### Phase 1: Test Infrastructure

**Goal:** Automatic postgres test instance provisioning

1. Create `docker-compose.test.yml` with postgres service
2. Add cargo make tasks for postgres lifecycle:
   - `cargo make postgres-up` - start test postgres
   - `cargo make postgres-down` - stop test postgres
   - `cargo make postgres-logs` - view logs
   - `cargo make postgres-reset` - reset database
3. Integrate with existing test setup
4. Verify postgres starts/stops correctly
5. Add healthcheck to ensure postgres is ready before tests run

**Success criteria:**
- `cargo make postgres-up` starts postgres container
- `cargo make postgres-down` stops and removes container
- Healthcheck ensures postgres is ready before tests run
- Test database is created and accessible

### Phase 2: Schema Analysis and Design

**Goal:** Understand and document sqlite schema

1. Extract all sqlite table definitions
2. Document each table with:
   - Purpose
   - Columns and types
   - Indexes
   - Foreign keys
   - Constraints
3. Identify flattening opportunities
4. Design postgres schema with proper types
5. Document migration strategy

**Success criteria:**
- Complete documentation of sqlite schema
- Postgres schema design document
- Migration strategy approved

### Phase 3: Schema Migration

**Goal:** Create postgres schema from sqlite

1. Create postgres schema migration script:
   - All tables with proper postgres types
   - Primary keys and foreign keys
   - Indexes for performance
   - Constraints for data integrity
2. Create data migration script:
   - Export data from sqlite
   - Transform data for postgres schema
   - Import data to postgres
3. Create validation script:
   - Compare row counts between sqlite and postgres
   - Validate data integrity
   - Test queries return same results

**Success criteria:**
- Postgres schema created successfully
- All data migrated without loss
- Validation passes
- Migration script documented and tested

### Phase 4: Code Migration

**Goal:** Update all code to use postgres

1. Update database connection layer:
   - Replace sqlite connection with postgres connection
   - Add connection pooling
   - Add configuration for postgres URL
2. Update all database queries:
   - Fix sqlite-specific syntax
   - Use postgres-compatible queries
   - Update parameter binding if needed
3. Update migrations:
   - Use postgres migrations (e.g., sqlx, refinery)
   - Ensure migrations run on postgres
4. Fix all compilation errors and warnings

**Success criteria:**
- All code compiles without errors
- All clippy warnings resolved
- Database operations work with postgres
- No sqlite-specific code remains

### Phase 5: Test Coverage

**Goal:** Comprehensive test coverage for database operations

1. Unit tests for:
   - All database models
   - All database operations (CRUD)
   - Migration logic
2. Integration tests with test postgres:
   - Test all database operations end-to-end
   - Test transactions
   - Test error handling
3. Migration tests:
   - Test schema migration
   - Test data migration
   - Test rollback
4. Performance tests:
   - Benchmark critical database operations
   - Compare sqlite vs postgres performance

**Success criteria:**
- All database operations have unit tests
- Integration tests cover all major workflows
- Migration tests pass
- Test coverage > 90% for database code
- No test overlaps

### Phase 6: Production Readiness

**Goal:** Production-ready postgres configuration

1. Configuration:
   - Environment variables for postgres connection
   - Connection pooling configuration
   - Timeout configuration
2. Error handling:
   - Proper error messages for database failures
   - Retry logic for transient failures
   - Connection error handling
3. Monitoring:
   - Database health checks
   - Connection pool metrics
   - Query performance logging
4. Documentation:
   - Migration guide
   - Configuration guide
   - Troubleshooting guide

**Success criteria:**
- Production configuration documented
- Error handling is robust
- Health checks implemented
- Documentation is complete

## Steps

### Step 1: Create test infrastructure (docker-compose + make tasks)

**Success:** `cargo make postgres-up` starts test postgres; `cargo make postgres-down` stops it; healthcheck ensures postgres is ready

**Status:** **CRITICAL - Current blocker** (test failures due to missing postgres instance)

**Current failures:**
```
FAIL [   0.036s] ( 175/1486) api database::migration_tests::test_migrations_via_database_new
FAIL [   0.052s] ( 754/1486) api::bin/api-server database::migration_tests::test_migrations_via_database_new
```

Error: `failed to connect to setup test database: Io(Custom { kind: Uncategorized, error: "failed to lookup address information: Name or service not known" })`

**Action items:**
1. Create `docker-compose.test.yml` with postgres service (check `agent/` directory for reference)
2. Add postgres lifecycle tasks to `Makefile.toml`:
   - `postgres-up` - start test postgres
   - `postgres-down` - stop test postgres
   - `postgres-logs` - view logs
   - `postgres-reset` - reset database
3. Integrate postgres startup with `cargo make` and `cargo nextest run`
4. Add healthcheck to ensure postgres is ready before tests run
5. Update test environment variables to use test postgres instance

Files to create:
- `docker-compose.test.yml` - Postgres service definition
- Update `Makefile.toml` - Add postgres tasks

### Step 2: Analyze existing sqlite schema

**Success:** Complete documentation of all sqlite tables, columns, indexes, and foreign keys

**Status:** Pending

Files to create:
- `docs/sqlite-schema-analysis.md` - Schema documentation

### Step 2a: Critically review consolidated migrations

**Success:** New postgres migrations are objectively as good or better than old sqlite schema

**Status:** Pending

**Review criteria:**
1. **Completeness:** All tables from sqlite schema exist in postgres
2. **Flattening:** Schema is appropriately flattened (no unnecessary normalization)
3. **Types:** Proper postgres types used (BIGINT, TEXT, TIMESTAMP, JSONB, etc.)
4. **Constraints:** Foreign keys, unique constraints, and indexes preserved or improved
5. **Indexes:** Performance-critical indexes maintained
6. **Data integrity:** NOT NULL constraints, CHECK constraints preserved
7. **Defaults:** Default values preserved
8. **Migration path:** Clear upgrade path from sqlite to postgres

**Action items:**
1. Extract and document complete sqlite schema
2. Extract and document new postgres schema
3. Create comparison matrix: sqlite table → postgres table
4. Identify any missing tables or columns
5. Identify any broken constraints or relationships
6. Verify indexes are appropriate for postgres
7. Check for proper use of postgres-specific features (JSONB, arrays, etc.)
8. Validate migration scripts produce equivalent schema
9. Test data migration preserves all data and relationships

### Step 3: Design postgres schema

**Success:** Postgres schema design with flattened structure, proper types, and migration strategy

**Status:** Pending

Files to create:
- `docs/postgres-schema-design.md` - Postgres schema design
- `migrations/` directory structure

### Step 4: Create schema migration script

**Success:** SQL script creates postgres schema from sqlite schema

**Status:** Pending

Files to create:
- `migrations/001_initial_schema.up.sql` - Create postgres schema
- `migrations/001_initial_schema.down.sql` - Rollback schema

### Step 5: Create data migration script

**Success:** Script migrates all data from sqlite to postgres

**Status:** Pending

Files to create:
- `tools/migrate-sqlite-to-postgres/` - Migration tooling
- Migration script with validation

### Step 6: Update database connection layer

**Success:** Code connects to postgres instead of sqlite

**Status:** Pending

Files to modify:
- All crates that connect to database
- Update connection code to use postgres
- Add connection pooling

### Step 7: Update all database queries

**Success:** All queries work with postgres syntax

**Status:** Pending

Files to modify:
- All files containing SQL queries
- Fix sqlite-specific syntax
- Update parameter binding

### Step 8: Fix all compilation errors and warnings

**Success:** `cargo make` passes without errors or warnings

**Status:** Pending

Files to modify:
- All files with compilation errors or warnings
- Run `cargo clippy --tests` and fix all warnings

### Step 9: Write comprehensive tests

**Success:** All database operations have solid test coverage

**Status:** Pending

Files to create:
- Unit tests for all database operations
- Integration tests with test postgres
- Migration tests

### Step 10: Validate production readiness

**Success:** Production configuration, error handling, and monitoring in place

**Status:** Pending

Files to create:
- Configuration documentation
- Migration guide
- Troubleshooting guide

## Testing Strategy

### Unit Tests

- Test all database models
- Test all CRUD operations
- Test migrations
- Test error handling

### Integration Tests

- Test all database operations with real postgres
- Test transactions
- Test concurrent operations
- Test connection pooling

### Migration Tests

- Test schema migration
- Test data migration
- Test rollback
- Test data validation

### Performance Tests

- Benchmark critical operations
- Compare sqlite vs postgres performance
- Test connection pool efficiency

## Rollback Plan

If migration fails:

1. Keep sqlite database as backup
2. Document rollback procedure
3. Provide postgres-to-sqlite migration if needed
4. Ensure no data loss during migration

## Success Metrics

- [ ] `cargo make` passes without warnings or errors
- [ ] All tests pass (unit + integration)
- [ ] Test coverage > 90% for database code
- [ ] Postgres schema created successfully
- [ ] All data migrated without loss
- [ ] Validation tests pass
- [ ] Production configuration documented
- [ ] Migration guide documented

## Risks and Mitigations

### Risk 1: Data loss during migration

**Mitigation:**
- Backup sqlite database before migration
- Validate migration with test data first
- Provide rollback mechanism
- Test migration thoroughly in staging

### Risk 2: Performance regression

**Mitigation:**
- Benchmark critical operations
- Optimize queries and indexes
- Use connection pooling
- Monitor performance in staging

### Risk 3: Breaking existing functionality

**Mitigation:**
- Comprehensive test coverage
- Integration tests for all workflows
- Gradual rollout with feature flags
- Monitor for issues in production

## Definition of Done

- [ ] All `cargo make` commands pass without warnings or errors
- [ ] Automatic postgres test instance provisioning works
- [ ] Complete sqlite to postgres schema migration
- [ ] All database operations work with postgres
- [ ] Comprehensive test coverage achieved
- [ ] Production configuration implemented
- [ ] Documentation complete
- [ ] No sqlite-specific code remains
- [ ] All tests pass consistently
