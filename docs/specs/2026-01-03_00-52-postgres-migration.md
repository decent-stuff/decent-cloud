# SQLite to PostgreSQL Migration

**Status:** Draft
**Created:** 2026-01-03
**Author:** Claude Code

## Summary

Migrate the entire codebase from SQLite to PostgreSQL as the primary database engine. Fix all compilation warnings and errors, ensure automatic test PostgreSQL instance management via `cargo make`, preserve the complete SQLite schema (flattened) in PostgreSQL, and achieve production-ready status with comprehensive test coverage.

## Problem Statement

The codebase is currently broken due to an in-progress migration from SQLite to PostgreSQL:

1. **Build failures**: `cargo make` fails with compilation errors and warnings
2. **Missing test infrastructure**: No automatic PostgreSQL test instance setup
3. **Incomplete schema migration**: SQLite schema not fully migrated to PostgreSQL
4. **No automated testing**: Tests cannot run without database setup
5. **Production not ready**: Code needs to be production-ready with PostgreSQL

## Current State

### Database Configuration
- **Current**: SQLite with database files in `data/api-data-dev/ledger.db` and `data/api-data-prod/ledger.db`
- **Target**: PostgreSQL with Docker-based test instances
- **Reference**: `agent/docker-compose.yml` already has PostgreSQL setup for testing

### Build System
- `Makefile.toml` has `postgres-start` task but integration incomplete
- `SQLX_OFFLINE=true` suggests offline mode is being used
- Agent directory has working PostgreSQL docker-compose setup

### Schema
- Migrations exist in `api/migrations/` (17 migration files)
- Original schema: `001_original_schema.sql` (~15KB)
- Need to verify all SQLite-specific syntax is converted to PostgreSQL

## Requirements

### Must-have

- [ ] `cargo make` runs without errors or warnings
- [ ] Automatic PostgreSQL test instance management (start/stop via cargo make)
- [ ] Complete SQLite schema flattened and migrated to PostgreSQL
- [ ] All database queries compatible with PostgreSQL (no SQLite-specific syntax)
- [ ] Environment variable configuration for PostgreSQL connection
- [ ] Comprehensive test coverage for all database operations
- [ ] Production-ready error handling and connection pooling
- [ ] Migration path for existing SQLite data (if needed)
- [ ] Documentation for local development setup

### Nice-to-have

- [ ] Automatic database migration on startup
- [ ] Connection pooling configuration
- [ ] Database health check endpoints
- [ ] Performance benchmarks comparing SQLite vs PostgreSQL
- [ ] CI/CD integration for PostgreSQL testing

## Technical Design

### Build System Integration

**Reference Implementation**: `agent/docker-compose.yml`

The agent directory already has a working PostgreSQL setup:
```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: test
      POSTGRES_PASSWORD: test
      POSTGRES_DB: test
    networks:
      - decent-cloud-network
```

**Tasks**:
1. Move PostgreSQL service to root `docker-compose.yml` (create if needed)
2. Update `Makefile.toml` to:
   - Start PostgreSQL via docker compose on `postgres-start` task
   - Run health checks using existing `scripts/docker-compose-health.sh`
   - Set `DATABASE_URL` environment variable
   - Stop PostgreSQL on `cleanup` task

### Database Schema Migration

**Migration Files**: `api/migrations/*.sql`

**Steps**:
1. Audit all 17 migration files for SQLite-specific syntax:
   - `INTEGER PRIMARY KEY` → PostgreSQL `SERIAL` or `BIGSERIAL`
   - `WITHOUT ROWID` → Not applicable, remove
   - SQLite-specific functions → PostgreSQL equivalents
   - Type conversions (BLOB, TEXT, etc.)

2. Key conversions:
   ```
   SQLite → PostgreSQL
   ------------------
   INTEGER → INTEGER or BIGINT
   REAL → DOUBLE PRECISION
   TEXT → TEXT or VARCHAR
   BLOB → BYTEA
   DATETIME → TIMESTAMP
   ```

3. Verify schema flattening:
   - Check for nested structures or JSON fields
   - Ensure all foreign keys are properly defined
   - Index creation for PostgreSQL

### Code Changes

**Configuration**:
```rust
// Use environment variable for database URL
const DATABASE_URL: &str = env::var("DATABASE_URL")
    .unwrap_or_else(|_| "postgres://test:test@localhost:5432/test".to_string());

// Sqlx connection pool
let pool = PgPoolOptions::new()
    .max_connections(5)
    .connect(&DATABASE_URL)
    .await?;
```

**Query Updates**:
1. Replace `rusqlite` with `sqlx` (PostgreSQL features)
2. Update `SQLX_OFFLINE` mode - may need to regenerate `sqlx-data.json`
3. Check for LIMIT/OFFSET syntax differences
4. Verify datetime handling
5. Update transaction syntax

**Files to Audit**:
```
api/src/db/
api/src/models/
api/src/openapi/
api/src/lib.rs
api/Cargo.toml (dependencies)
```

### Test Infrastructure

**Requirements**:
1. Tests should use separate test database
2. Each test should run in isolation (transactions with rollback)
3. Fixtures for common test data
4. Migration runner for test setup

**Test Database Configuration**:
```toml
[env]
RUST_BACKTRACE = "1"
SQLX_OFFLINE = "false"  # Enable online mode for testing
DATABASE_URL = "postgres://test:test@localhost:5432/test"
```

**Test Setup Pattern**:
```rust
#[sqlx::test]
async fn test_user_creation(pool: PgPool) {
    // pool is automatically created and cleaned up
    let user = create_user(&pool, "test@example.com").await?;
    assert_eq!(user.email, "test@example.com");
}
```

### Data Migration (Optional)

**If existing production data needs migration**:

1. Export SQLite to SQL dump:
   ```bash
   sqlite3 data/api-data-prod/ledger.db .dump > dump.sql
   ```

2. Convert dump to PostgreSQL syntax:
   - Remove SQLite-specific pragmas
   - Convert data types
   - Fix transaction syntax

3. Import to PostgreSQL:
   ```bash
   psql -U postgres -d decent_cloud < dump.sql
   ```

**Alternative**: Write Rust migration tool that reads SQLite and writes to PostgreSQL.

## Implementation Plan

### Phase 1: Build System & Database Setup
- [ ] Create root `docker-compose.yml` with PostgreSQL service
- [ ] Update `Makefile.toml` for automatic PostgreSQL management
- [ ] Add health check verification
- [ ] Verify `cargo make postgres-start` works
- [ ] Add `cargo make postgres-stop` task

### Phase 2: Schema Migration
- [ ] Audit all migration files for SQLite-specific syntax
- [ ] Convert `001_original_schema.sql` to PostgreSQL
- [ ] Convert remaining 16 migration files
- [ ] Test schema creation on clean PostgreSQL database
- [ ] Verify all foreign keys and indexes

### Phase 3: Code Updates
- [ ] Update database configuration to use `DATABASE_URL`
- [ ] Replace SQLite-specific dependencies with PostgreSQL
- [ ] Update all database queries for PostgreSQL compatibility
- [ ] Fix compilation errors
- [ ] Regenerate `sqlx-data.json` if needed

### Phase 4: Test Coverage
- [ ] Set up test database infrastructure
- [ ] Write tests for all database operations
- [ ] Add integration tests for migrations
- [ ] Ensure tests run in `cargo make test`
- [ ] Verify tests pass

### Phase 5: Production Readiness
- [ ] Add connection pooling configuration
- [ ] Implement proper error handling
- [ ] Add database health checks
- [ ] Document local development setup
- [ ] Run `cargo clippy --tests` and fix all warnings
- [ ] Verify production deployment configuration

### Phase 6: Verification
- [ ] Full test suite passes: `cargo make test`
- [ ] No warnings: `cargo clippy --tests`
- [ ] Build succeeds: `cargo make build`
- [ ] Manual testing of API endpoints
- [ ] Performance testing (if applicable)

## Acceptance Criteria

1. **Build System**: `cargo make` completes successfully without errors or warnings
2. **Database Setup**: PostgreSQL test instance starts automatically via `cargo make postgres-start`
3. **Schema Complete**: All 17 migrations run successfully on PostgreSQL
4. **Code Quality**: `cargo clippy --tests` passes with zero warnings
5. **Test Coverage**: All database operations have tests, tests pass
6. **Production Ready**: Code handles connection errors, implements connection pooling, and has proper logging
7. **Documentation**: Local development setup documented in README or similar

## Rollout Plan

1. **Development**: Merge to main after all tests pass
2. **Staging**: Deploy to staging environment with PostgreSQL
3. **Data Migration**: Migrate staging data from SQLite to PostgreSQL (if applicable)
4. **Testing**: Comprehensive testing on staging
5. **Production**: Deploy to production with PostgreSQL
6. **Monitoring**: Monitor database performance and errors

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Data loss during migration | HIGH | Comprehensive backups, test migration on staging first |
| Performance degradation | MEDIUM | Benchmark PostgreSQL, add indexes as needed |
| Connection pool exhaustion | MEDIUM | Proper pool sizing, monitoring, alerts |
| Syntax compatibility issues | MEDIUM | Comprehensive testing, audit all queries |
| Downtime during migration | MEDIUM | Plan maintenance window, use read replicas if needed |

## References

- Agent PostgreSQL setup: `/home/sat/projects/decent-cloud/agent/docker-compose.yml`
- Migration files: `/home/sat/projects/decent-cloud/api/migrations/`
- Makefile: `/home/sat/projects/decent-cloud/Makefile.toml`
- SQLite to PostgreSQL guide: https://www.postgresql.org/docs/current/datatype.html
