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
        include_str!("../../migrations/024_chatwoot_tracking.sql"),
        include_str!("../../migrations/025_icpay_rename.sql"),
        include_str!("../../migrations/026_sla_tracking.sql"),
        include_str!("../../migrations/027_chatwoot_user_id.sql"),
        include_str!("../../migrations/028_provider_notification_config.sql"),
        include_str!("../../migrations/029_telegram_message_tracking.sql"),
        include_str!("../../migrations/030_icpay_escrow.sql"),
        include_str!("../../migrations/031_notification_usage.sql"),
        include_str!("../../migrations/032_user_notification_config.sql"),
        include_str!("../../migrations/033_remove_messaging.sql"),
        include_str!("../../migrations/034_provider_onboarding.sql"),
        include_str!("../../migrations/035_external_providers.sql"),
        include_str!("../../migrations/036_reseller_infrastructure.sql"),
        include_str!("../../migrations/037_chatwoot_provider_resources.sql"),
        include_str!("../../migrations/038_receipt_tracking.sql"),
        include_str!("../../migrations/039_invoices.sql"),
        include_str!("../../migrations/040_tax_tracking.sql"),
        include_str!("../../migrations/041_buyer_address.sql"),
        include_str!("../../migrations/042_billing_settings.sql"),
        include_str!("../../migrations/043_drop_chatwoot_portal_slug_from_user_notification.sql"),
        include_str!("../../migrations/044_stripe_invoice_id.sql"),
        include_str!("../../migrations/045_pending_stripe_receipts.sql"),
        include_str!("../../migrations/046_remove_invoice_pdf_blob.sql"),
        include_str!("../../migrations/047_agent_delegations.sql"),
        include_str!("../../migrations/048_auto_accept_rentals.sql"),
        include_str!("../../migrations/049_auto_accept_default_on.sql"),
        include_str!("../../migrations/050_account_based_identification.sql"),
        include_str!("../../migrations/051_termination_tracking.sql"),
    ];

    for migration in &migrations {
        sqlx::query(migration).execute(&pool).await.unwrap();
    }

    Database { pool }
}
