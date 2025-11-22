/// Shared test helpers for database tests
use super::Database;
use sqlx::SqlitePool;

/// Set up a test database with all migrations applied
pub async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    // Run all migrations in order
    // NOTE: When adding new migrations, add them here
    let migrations = [
        include_str!("../../migrations/001_original_schema.sql"),
        include_str!("../../migrations/002_account_profiles.sql"),
        include_str!("../../migrations/003_device_names.sql"),
        include_str!("../../migrations/004_account_profiles_fix.sql"),
    ];

    for migration in &migrations {
        sqlx::query(migration).execute(&pool).await.unwrap();
    }

    Database { pool }
}
