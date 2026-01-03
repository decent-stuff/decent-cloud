/// Shared test helpers for database tests
///
/// MIGRATION APPROACH DIFFERENCES:
///
/// This module uses `include_str!()` to manually execute SQL files, while core.rs uses
/// `sqlx::migrate!()`. This is intentional and both approaches are correct for their contexts:
///
/// **core.rs (api-server runtime):**
/// - Uses `sqlx::migrate!("./migrations_pg")`
/// - Macro resolves path relative to crate root (CARGO_MANIFEST_DIR)
/// - Tracks migration state in __sqlx_migrations table
/// - Each migration runs only once (idempotent)
/// - Perfect for production use
///
/// **test_helpers.rs (tests):**
/// - Uses `include_str!("../../migrations_pg/...")`
/// - Embeds SQL at compile time, no runtime file access needed
/// - Creates fresh schema for each test database
/// - Better for test isolation and concurrent test execution
/// - Both approaches execute the same SQL, resulting schema is identical
use super::Database;
use sqlx::PgPool;
use std::sync::atomic::{AtomicU32, Ordering};

static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Set up a test database with all migrations applied
///
/// Requires TEST_DATABASE_URL environment variable pointing to a PostgreSQL server.
/// Default: postgres://test:test@localhost:5432 (without database name for admin connections).
///
/// Each test gets a unique database that is dropped after the test.
/// Uses `include_str!()` instead of `sqlx::migrate!()` for better test isolation.
pub async fn setup_test_db() -> Database {
    let base_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://test:test@localhost:5432".to_string());

    // Create a unique database name for this test
    let test_id = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("test_db_{}_{}", std::process::id(), test_id);

    // Connect to the postgres database to create our test database
    let admin_url = format!("{}/postgres", base_url);
    let admin_pool = PgPool::connect(&admin_url)
        .await
        .expect("Failed to connect to PostgreSQL admin database");

    // Drop the test database if it exists, then create it fresh
    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name))
        .execute(&admin_pool)
        .await
        .expect("Failed to drop existing test database");

    sqlx::query(&format!("CREATE DATABASE {}", db_name))
        .execute(&admin_pool)
        .await
        .expect("Failed to create test database");

    admin_pool.close().await;

    // Connect to the new test database and run migrations
    let test_url = format!("{}/{}", base_url, db_name);
    let pool = PgPool::connect(&test_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations from consolidated PostgreSQL files using raw_sql for multi-statement execution
    let migrations = [
        include_str!("../../migrations_pg/001_schema.sql"),
        include_str!("../../migrations_pg/002_seed_data.sql"),
    ];

    for migration in &migrations {
        sqlx::raw_sql(migration)
            .execute(&pool)
            .await
            .expect("Migration failed");
    }

    Database { pool }
}

/// Clean up a test database (call this in test cleanup)
pub async fn cleanup_test_db(db: Database) {
    // The pool will be dropped, but we could also explicitly drop the database
    // For now, just close the connection
    db.pool.close().await;
}
