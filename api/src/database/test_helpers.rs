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
        include_str!("../../migrations/005_oauth_support.sql"),
        include_str!("../../migrations/006_gpu_fields.sql"),
        include_str!("../../migrations/007_username_case_sensitive.sql"),
        include_str!("../../migrations/008_example_offerings.sql"),
        include_str!("../../migrations/009_email_queue.sql"),
        include_str!("../../migrations/010_payment_methods.sql"),
        include_str!("../../migrations/011_payment_status.sql"),
        include_str!("../../migrations/012_refund_tracking.sql"),
        include_str!("../../migrations/013_contract_currency.sql"),
        include_str!("../../migrations/014_fix_contract_currency_data.sql"),
        include_str!("../../migrations/015_update_example_offering_currencies.sql"),
        include_str!("../../migrations/016_contract_currency.sql"),
        include_str!("../../migrations/017_drop_currency_default.sql"),
        include_str!("../../migrations/018_provider_trust_cache.sql"),
        include_str!("../../migrations/019_last_login_tracking.sql"),
        include_str!("../../migrations/020_email_verification.sql"),
        include_str!("../../migrations/021_admin_accounts.sql"),
        include_str!("../../migrations/022_messaging.sql"),
        include_str!("../../migrations/023_email_queue_time_based_retry.sql"),
    ];

    for migration in &migrations {
        sqlx::query(migration).execute(&pool).await.unwrap();
    }

    Database { pool }
}
