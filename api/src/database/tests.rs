use crate::database::{Database, LedgerEntryData};
use sqlx::{Row, SqlitePool};

/// Test database setup - manual table creation for test isolation
///
/// NOTE: We manually create tables instead of using migrations because:
/// 1. sqlx migrations don't work well with in-memory SQLite databases
/// 2. File-based migrations cause test isolation issues and cleanup complexity  
/// 3. Manual creation gives us precise control over test schema
/// 4. The manual schema matches migration 001_flattened_schema.sql
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

    fn registration_entry(label: &str, pubkey_hash: &[u8]) -> LedgerEntryData {
        Self::ledger_entry(label, pubkey_hash, b"signature_data", 1234567890, 1)
    }
}

/// Test helper to count rows in a table
async fn count_table_rows(db: &Database, table: &str) -> i64 {
    sqlx::query(&format!("SELECT COUNT(*) as count FROM {}", table))
        .fetch_one(&db.pool)
        .await
        .unwrap()
        .get("count")
}

/// Test helper to verify sync position
async fn assert_sync_position(db: &Database, expected: u64) {
    assert_eq!(db.get_last_sync_position().await.unwrap(), expected);
}

/// Test helper to verify table contents
async fn assert_table_count(db: &Database, table: &str, expected: i64) {
    assert_eq!(count_table_rows(db, table).await, expected);
}

// Core functionality tests
// Test database setup - using in-memory for simplicity
///
/// NOTE: Using in-memory SQLite for fast, isolated tests.
/// While migrations would be ideal, they're complex with in-memory DBs.
/// For now, manual setup gives us reliable test isolation.
async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    // Load and execute migration SQL at runtime for production-like schema
    let migration_sql = include_str!("../../migrations/001_flattened_schema.sql");

    // Execute the entire migration as a single batch
    sqlx::query(migration_sql).execute(&pool).await.unwrap();

    // Create sync_state table and initialize
    sqlx::query("CREATE TABLE IF NOT EXISTS sync_state (id INTEGER PRIMARY KEY, last_position INTEGER, last_sync_at TIMESTAMP)")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT OR IGNORE INTO sync_state (id, last_position, last_sync_at) VALUES (1, 0, CURRENT_TIMESTAMP)")
        .execute(&pool).await.unwrap();

    Database { pool }
}

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
        // Registration entries (should use INSERT OR REPLACE)
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
    let timestamps: Vec<i64> = sqlx::query(
        "SELECT block_timestamp_ns FROM provider_check_ins ORDER BY block_timestamp_ns",
    )
    .fetch_all(&db.pool)
    .await
    .unwrap()
    .into_iter()
    .map(|row| row.get("block_timestamp_ns"))
    .collect();

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
