use crate::database::{Database, LedgerEntryData};
use sqlx::Row;

/// Test database setup - manual table creation for test isolation
///
/// NOTE: We manually create tables instead of using migrations because:
/// 1. Migrations add complexity to test setup/teardown
/// 2. Manual creation gives us precise control over test schema
/// 3. Each test gets a fresh, isolated schema
/// 4. The manual schema matches migration 001_original_schema.sql
///
/// Test data factory for creating consistent test entries
struct TestDataFactory;
impl TestDataFactory {
    fn ledger_entry(
        label: &str,
        key: &[u8],
        value: &[u8],
        timestamp: u64,
        offset: u64,
    ) -> LedgerEntryData {
        LedgerEntryData {
            label: label.to_string(),
            key: key.to_vec(),
            value: value.to_vec(),
            block_timestamp_ns: timestamp,
            block_hash: vec![offset as u8; 3],
            block_offset: offset,
        }
    }

    fn check_in_entries(provider_key: &[u8], base_timestamp: u64) -> Vec<LedgerEntryData> {
        vec![
            Self::ledger_entry(
                "ProvCheckIn",
                provider_key,
                &dcc_common::CheckInPayload::new("First".to_string(), vec![1, 2, 3, 4])
                    .to_bytes()
                    .unwrap(),
                base_timestamp,
                1,
            ),
            Self::ledger_entry(
                "ProvCheckIn",
                provider_key,
                &dcc_common::CheckInPayload::new("Second".to_string(), vec![5, 6, 7, 8])
                    .to_bytes()
                    .unwrap(),
                base_timestamp + 1000,
                2,
            ),
        ]
    }

    fn registration_entry(label: &str, pubkey: &[u8]) -> LedgerEntryData {
        Self::ledger_entry(label, pubkey, b"signature_data", 1234567890, 1)
    }
}

/// Test helper to count rows in a table, excluding example provider data
async fn count_table_rows(db: &Database, table: &str) -> i64 {
    // Example provider pubkey hash from migration 002
    let example_provider_pubkey =
        hex::decode("6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572")
            .unwrap();

    // For provider tables, exclude the example provider
    let query = if table.starts_with("provider_") {
        format!("SELECT COUNT(*) as count FROM {} WHERE pubkey != $1", table)
    } else {
        format!("SELECT COUNT(*) as count FROM {}", table)
    };

    let result = if table.starts_with("provider_") {
        sqlx::query(&query)
            .bind(example_provider_pubkey)
            .fetch_one(&db.pool)
            .await
            .unwrap()
    } else {
        sqlx::query(&query).fetch_one(&db.pool).await.unwrap()
    };

    result.get("count")
}

/// Test helper to verify sync position
async fn assert_sync_position(db: &Database, expected: u64) {
    assert_eq!(db.get_last_sync_position().await.unwrap(), expected);
}

/// Test helper to verify table contents
async fn assert_table_count(db: &Database, table: &str, expected: i64) {
    assert_eq!(count_table_rows(db, table).await, expected);
}

/// Delete example provider data so tests get clean counts.
/// Migration 054 creates pools for example provider, making example offerings visible in search.
async fn delete_example_data(db: &Database) {
    let example_pubkey = Database::example_provider_pubkey();
    // Delete in correct order to respect foreign key constraints
    sqlx::query("DELETE FROM provider_agent_delegations WHERE provider_pubkey = $1")
        .bind(&example_pubkey[..])
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM agent_pools WHERE provider_pubkey = $1")
        .bind(&example_pubkey[..])
        .execute(&db.pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM provider_offerings WHERE pubkey = $1")
        .bind(&example_pubkey[..])
        .execute(&db.pool)
        .await
        .unwrap();
}

// Core functionality tests
// Test database setup - using shared helper that runs all migrations
use crate::database::test_helpers::setup_test_db;

// Core functionality tests
#[tokio::test]
async fn test_database_basic_operations() {
    let db = setup_test_db().await;

    // Test initial sync state
    assert_sync_position(&db, 0).await;

    // Test empty entries insertion
    assert!(db.insert_entries(vec![]).await.is_ok());

    // Test sync position updates
    let test_positions = [42, 100, 999];
    for pos in test_positions {
        db.update_sync_position(pos).await.unwrap();
        assert_sync_position(&db, pos).await;
    }
}

// Test that all tables store entries properly
#[tokio::test]
async fn test_all_entry_types_storage() {
    let db = setup_test_db().await;
    let provider_key = b"provider_key_123";
    let user_key = b"user_key_456";
    let base_timestamp = 1234567890;

    let entries = vec![
        // Registration entries (use ON CONFLICT DO UPDATE for upsert)
        TestDataFactory::registration_entry("ProvRegister", provider_key),
        TestDataFactory::registration_entry("UserRegister", user_key),
    ]
    .into_iter()
    .chain(TestDataFactory::check_in_entries(
        provider_key,
        base_timestamp,
    ))
    .collect::<Vec<LedgerEntryData>>();

    assert!(db.insert_entries(entries).await.is_ok());

    // Verify all entries stored
    assert_table_count(&db, "provider_registrations", 1).await;
    assert_table_count(&db, "user_registrations", 1).await;
    assert_table_count(&db, "provider_check_ins", 2).await;
}

// Test the "store all entries" behavior for critical tables
#[tokio::test]
async fn test_historical_tables_store_all_entries() {
    let db = setup_test_db().await;
    let provider_key = b"provider_hist_123";

    // Create multiple check-ins over time with increasing timestamps
    let mut all_entries = Vec::new();
    for i in 0..5 {
        let timestamp = 1234567890 + (i * 2000); // 2 second intervals
        let check_ins = TestDataFactory::check_in_entries(provider_key, timestamp);
        all_entries.extend(check_ins);
    }

    assert!(db.insert_entries(all_entries).await.is_ok());

    // Should have all 10 check-ins stored
    assert_table_count(&db, "provider_check_ins", 10).await;

    // Verify timestamps are sequential and unique
    let timestamps: Vec<i64> = sqlx::query_scalar!(
        r#"SELECT block_timestamp_ns as "block_timestamp_ns!: i64" FROM provider_check_ins ORDER BY block_timestamp_ns"#,
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();

    // All timestamps should be unique and ordered
    assert_eq!(timestamps.len(), 10);
    // Test that timestamps are properly ordered
    for i in 1..timestamps.len() {
        assert!(
            timestamps[i] > timestamps[i - 1],
            "Timestamps should be increasing"
        );
    }
}

// Test that registration tables properly replace entries
#[tokio::test]
async fn test_unique_tables_replace_entries() {
    let db = setup_test_db().await;
    let provider_key = b"provider_unique_123";

    // Insert initial registration
    let entries1 = vec![TestDataFactory::registration_entry(
        "ProvRegister",
        provider_key,
    )];
    assert!(db.insert_entries(entries1).await.is_ok());
    assert_table_count(&db, "provider_registrations", 1).await;

    // Insert duplicate registration (should replace)
    let entries2 = vec![TestDataFactory::registration_entry(
        "ProvRegister",
        provider_key,
    )];
    assert!(db.insert_entries(entries2).await.is_ok());
    assert_table_count(&db, "provider_registrations", 1).await; // Still only 1 row
}

// Test that invalid labels are handled gracefully
#[tokio::test]
async fn test_unknown_label_handling() {
    let db = setup_test_db().await;

    let entries = vec![TestDataFactory::ledger_entry(
        "UnknownLabel",
        b"key",
        b"value",
        1234567890,
        1,
    )];

    // Should not fail, just log warning
    assert!(db.insert_entries(entries).await.is_ok());
}

// Performance test with many entries
#[tokio::test]
async fn test_bulk_insert_performance() {
    let db = setup_test_db().await;
    let provider_key = b"provider_bulk_123";

    let mut entries = Vec::new();
    for i in 0..100 {
        let timestamp = 1234567890 + (i * 100);
        entries.push(TestDataFactory::ledger_entry(
            "ProvCheckIn",
            provider_key,
            &dcc_common::CheckInPayload::new(
                format!("check_in_{}", i),
                vec![i as u8; 4], // simple signature
            )
            .to_bytes()
            .unwrap(),
            timestamp,
            i,
        ));
    }

    let start = std::time::Instant::now();
    assert!(db.insert_entries(entries).await.is_ok());
    let duration = start.elapsed();

    // Verify all entries stored
    assert_table_count(&db, "provider_check_ins", 100).await;

    // Performance assertion - should complete reasonably fast
    assert!(
        duration.as_secs() < 5,
        "Bulk insert took too long: {:?}",
        duration
    );
}

// Example offerings tests
#[tokio::test]
async fn test_get_example_offerings() {
    let db = setup_test_db().await;

    // Test retrieving example offerings
    let example_offerings = db.get_example_offerings().await.unwrap();

    // Should have exactly 10 example offerings from migration (2 per product type)
    assert_eq!(example_offerings.len(), 10);

    // Verify we have offerings for all product types
    let product_types: Vec<_> = example_offerings
        .iter()
        .map(|o| o.product_type.as_str())
        .collect();
    assert!(product_types.contains(&"compute"));
    assert!(product_types.contains(&"gpu"));
    assert!(product_types.contains(&"storage"));
    assert!(product_types.contains(&"network"));
    assert!(product_types.contains(&"dedicated"));

    // Find and verify a compute offering
    let compute_offering = example_offerings
        .iter()
        .find(|o| o.offering_id == "compute-001")
        .expect("Should have compute-001");
    assert_eq!(compute_offering.offer_name, "Basic VPS");
    assert_eq!(compute_offering.currency, "USD");
    assert_eq!(compute_offering.product_type, "compute");

    // Verify it has required data
    assert!(compute_offering.payment_methods.is_some());
    assert!(compute_offering.features.is_some());
    assert!(compute_offering.operating_systems.is_some());

    // Find and verify a GPU offering
    let gpu_offering = example_offerings
        .iter()
        .find(|o| o.offering_id == "gpu-001")
        .expect("Should have gpu-001");
    assert_eq!(gpu_offering.offer_name, "AI Training - RTX 4090");
    assert_eq!(gpu_offering.currency, "ICP");
    assert_eq!(gpu_offering.product_type, "gpu");
    assert!(gpu_offering.gpu_name.is_some());
    assert!(gpu_offering.gpu_count.is_some());
}

#[tokio::test]
async fn test_csv_template_data_retrieval() {
    let db = setup_test_db().await;

    // Verify we can retrieve all data needed for CSV template generation
    let example_offerings = db.get_example_offerings().await.unwrap();
    assert_eq!(example_offerings.len(), 10);

    // For each example offering, verify we can fetch all related data without errors
    for offering in &example_offerings {
        let payment_methods = offering.payment_methods.as_deref().unwrap_or("");
        assert!(
            !payment_methods.is_empty(),
            "Payment methods should not be empty for {}",
            offering.offering_id
        );

        let features = offering.features.as_deref().unwrap_or("");
        assert!(
            !features.is_empty(),
            "Features should not be empty for {}",
            offering.offering_id
        );

        // Operating systems are only required for compute, GPU, and dedicated offerings
        if matches!(
            offering.product_type.as_str(),
            "compute" | "gpu" | "dedicated"
        ) {
            let operating_systems = offering.operating_systems.as_deref().unwrap_or("");
            assert!(
                !operating_systems.is_empty(),
                "Operating systems should not be empty for {} offerings",
                offering.product_type
            );
        }
    }

    // Verify example offerings have correct visibility
    for offering in &example_offerings {
        assert_eq!(
            offering.visibility, "public",
            "Example offerings should have visibility='public'"
        );
    }
}

#[tokio::test]
async fn test_offerings_with_pools_included_in_search() {
    let db = setup_test_db().await;
    delete_example_data(&db).await;

    // Create a provider with pool (offerings without pools are filtered from marketplace)
    let pubkey = vec![1u8; 32];
    {
        let pubkey_ref: &[u8] = &pubkey;
        sqlx::query!(
            "INSERT INTO provider_registrations (pubkey, signature, created_at_ns) VALUES ($1, $2, 0)",
            pubkey_ref,
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Create a pool for US region
    sqlx::query(
        "INSERT INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns) VALUES ($1, $2, 'Test Pool', 'na', 'manual', 0)"
    )
    .bind("test-pool-na")
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    {
        let pubkey_ref: &[u8] = &pubkey;
        sqlx::query!(
            "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'test-public-001', 'Test Public Offering', 'USD', 99.99, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'Test City', FALSE, 0)",
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Search offerings - only offerings with matching pools appear
    // (Example offerings from migration have no pools, so they're excluded)
    let search_params = crate::database::offerings::SearchOfferingsParams {
        product_type: None,
        country: None,
        in_stock_only: false,
        min_price_monthly: None,
        max_price_monthly: None,
        limit: 100,
        offset: 0,
    };

    let search_results = db.search_offerings(search_params).await.unwrap();
    assert_eq!(
        search_results.len(),
        1,
        "Search should return only offerings with matching pools"
    );

    // Find our test offering
    let test_offering = search_results
        .iter()
        .find(|o| o.offering_id == "test-public-001");
    assert!(
        test_offering.is_some(),
        "Test offering should be in results"
    );
    assert_eq!(test_offering.unwrap().visibility, "public");

    // count_offerings counts all public offerings (example data was deleted)
    let total_count = db.count_offerings(None).await.unwrap();
    assert_eq!(total_count, 1, "Count should include all public offerings");
}

// Validator tests
#[tokio::test]
async fn test_get_active_validators() {
    let db = setup_test_db().await;
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    // Create test validators with different activity patterns
    let validator1 = b"validator_1_active_now";
    let validator2 = b"validator_2_active_week";
    let validator3 = b"validator_3_inactive";
    let validator1_hex = hex::encode(validator1);
    let validator2_hex = hex::encode(validator2);

    // Register all validators
    let registrations = vec![
        TestDataFactory::registration_entry("ProvRegister", validator1),
        TestDataFactory::registration_entry("ProvRegister", validator2),
        TestDataFactory::registration_entry("ProvRegister", validator3),
    ];
    db.insert_entries(registrations).await.unwrap();

    // Validator 1: Multiple recent check-ins (last 24h, 7d, 30d)
    let check_ins_v1 = vec![
        TestDataFactory::ledger_entry(
            "ProvCheckIn",
            validator1,
            &dcc_common::CheckInPayload::new("recent1".to_string(), vec![1, 2, 3, 4])
                .to_bytes()
                .unwrap(),
            (now_ns - 3600 * 1_000_000_000) as u64, // 1 hour ago
            1,
        ),
        TestDataFactory::ledger_entry(
            "ProvCheckIn",
            validator1,
            &dcc_common::CheckInPayload::new("recent2".to_string(), vec![5, 6, 7, 8])
                .to_bytes()
                .unwrap(),
            (now_ns - 12 * 3600 * 1_000_000_000) as u64, // 12 hours ago
            2,
        ),
        TestDataFactory::ledger_entry(
            "ProvCheckIn",
            validator1,
            &dcc_common::CheckInPayload::new("week_ago".to_string(), vec![9, 10, 11, 12])
                .to_bytes()
                .unwrap(),
            (now_ns - 3 * 24 * 3600 * 1_000_000_000) as u64, // 3 days ago
            3,
        ),
    ];

    // Validator 2: Only checked in 10 days ago (within 30d but not 7d)
    let check_ins_v2 = vec![TestDataFactory::ledger_entry(
        "ProvCheckIn",
        validator2,
        &dcc_common::CheckInPayload::new("ten_days_ago".to_string(), vec![13, 14, 15, 16])
            .to_bytes()
            .unwrap(),
        (now_ns - 10 * 24 * 3600 * 1_000_000_000) as u64, // 10 days ago
        4,
    )];

    // Validator 3: Checked in 35 days ago (inactive)
    let check_ins_v3 = vec![TestDataFactory::ledger_entry(
        "ProvCheckIn",
        validator3,
        &dcc_common::CheckInPayload::new("old".to_string(), vec![17, 18, 19, 20])
            .to_bytes()
            .unwrap(),
        (now_ns - 35 * 24 * 3600 * 1_000_000_000) as u64, // 35 days ago
        5,
    )];

    db.insert_entries(check_ins_v1).await.unwrap();
    db.insert_entries(check_ins_v2).await.unwrap();
    db.insert_entries(check_ins_v3).await.unwrap();

    // Test: Get validators active in last 30 days
    let validators_30d = db.get_active_validators(30).await.unwrap();
    assert_eq!(
        validators_30d.len(),
        2,
        "Should have 2 validators active in last 30 days"
    );

    // Find validator1 in results
    let v1 = validators_30d
        .iter()
        .find(|v| v.pubkey == validator1_hex)
        .expect("Validator 1 should be in results");

    assert_eq!(
        v1.total_check_ins, 3,
        "Validator 1 should have 3 total check-ins"
    );
    assert_eq!(
        v1.check_ins_24h, 2,
        "Validator 1 should have 2 check-ins in last 24h"
    );
    assert_eq!(
        v1.check_ins_7d, 3,
        "Validator 1 should have 3 check-ins in last 7d"
    );
    assert_eq!(
        v1.check_ins_30d, 3,
        "Validator 1 should have 3 check-ins in last 30d"
    );

    // Find validator2 in results
    let v2 = validators_30d
        .iter()
        .find(|v| v.pubkey == validator2_hex)
        .expect("Validator 2 should be in results");

    assert_eq!(
        v2.total_check_ins, 1,
        "Validator 2 should have 1 total check-in"
    );
    assert_eq!(
        v2.check_ins_24h, 0,
        "Validator 2 should have 0 check-ins in last 24h"
    );
    assert_eq!(
        v2.check_ins_7d, 0,
        "Validator 2 should have 0 check-ins in last 7d"
    );
    assert_eq!(
        v2.check_ins_30d, 1,
        "Validator 2 should have 1 check-in in last 30d"
    );

    // Test: Get validators active in last 7 days (should only have validator1)
    let validators_7d = db.get_active_validators(7).await.unwrap();
    assert_eq!(
        validators_7d.len(),
        1,
        "Should have 1 validator active in last 7 days"
    );
    assert_eq!(
        validators_7d[0].pubkey, validator1_hex,
        "Only validator 1 should be active in last 7 days"
    );

    // Test: Get validators active in last 1 day (should only have validator1)
    let validators_1d = db.get_active_validators(1).await.unwrap();
    assert_eq!(
        validators_1d.len(),
        1,
        "Should have 1 validator active in last 24 hours"
    );
    assert_eq!(
        validators_1d[0].check_ins_24h, 2,
        "Should have 2 check-ins in 24h"
    );
}

#[tokio::test]
async fn test_get_active_validators_with_profile() {
    let db = setup_test_db().await;
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    let validator_key = b"validator_with_profile";

    // Register validator
    db.insert_entries(vec![TestDataFactory::registration_entry(
        "ProvRegister",
        validator_key,
    )])
    .await
    .unwrap();

    // Add check-in
    db.insert_entries(vec![TestDataFactory::ledger_entry(
        "ProvCheckIn",
        validator_key,
        &dcc_common::CheckInPayload::new("test".to_string(), vec![1, 2, 3, 4])
            .to_bytes()
            .unwrap(),
        (now_ns - 3600 * 1_000_000_000) as u64,
        1,
    )])
    .await
    .unwrap();

    // Add profile for this validator
    {
        let validator_key_ref: &[u8] = &validator_key[..];
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            validator_key_ref,
            "Test Validator",
            Some("A test validator"),
            Some("https://example.com"),
            Some("https://example.com/logo.png"),
            Some("We're reliable!"),
            "v1",
            "0.1.0",
            now_ns
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Get active validators
    let validators = db.get_active_validators(1).await.unwrap();
    assert_eq!(validators.len(), 1);

    let validator = &validators[0];
    assert_eq!(validator.name, Some("Test Validator".to_string()));
    assert_eq!(validator.description, Some("A test validator".to_string()));
    assert_eq!(
        validator.website_url,
        Some("https://example.com".to_string())
    );
    assert_eq!(
        validator.logo_url,
        Some("https://example.com/logo.png".to_string())
    );
}

#[tokio::test]
async fn test_get_active_validators_without_profile() {
    let db = setup_test_db().await;
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    let validator_key = b"validator_no_profile";

    // Register validator (no profile)
    db.insert_entries(vec![TestDataFactory::registration_entry(
        "ProvRegister",
        validator_key,
    )])
    .await
    .unwrap();

    // Add check-in
    db.insert_entries(vec![TestDataFactory::ledger_entry(
        "ProvCheckIn",
        validator_key,
        &dcc_common::CheckInPayload::new("test".to_string(), vec![1, 2, 3, 4])
            .to_bytes()
            .unwrap(),
        (now_ns - 3600 * 1_000_000_000) as u64,
        1,
    )])
    .await
    .unwrap();

    // Get active validators - should still return the validator even without a profile
    let validators = db.get_active_validators(1).await.unwrap();
    assert_eq!(validators.len(), 1);

    let validator = &validators[0];
    assert_eq!(validator.name, None);
    assert_eq!(validator.description, None);
    assert_eq!(validator.website_url, None);
    assert_eq!(validator.logo_url, None);
    assert_eq!(validator.total_check_ins, 1);
    assert_eq!(validator.check_ins_24h, 1);
}
