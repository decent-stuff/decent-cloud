use crate::database::{Database, LedgerEntryData};
use sqlx::{Row, SqlitePool};

/// Test database setup - manual table creation for test isolation
///
/// NOTE: We manually create tables instead of using migrations because:
/// 1. sqlx migrations don't work well with in-memory SQLite databases
/// 2. File-based migrations cause test isolation issues and cleanup complexity
/// 3. Manual creation gives us precise control over test schema
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
    let migration_sql_001 = include_str!("../../migrations/001_original_schema.sql");
    let migration_sql_002 = include_str!("../../migrations/002_user_profiles.sql");

    // Execute migrations in order
    sqlx::query(migration_sql_001).execute(&pool).await.unwrap();
    sqlx::query(migration_sql_002).execute(&pool).await.unwrap();

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

// User profile tests
#[tokio::test]
async fn test_user_profile_storage_and_retrieval() {
    let db = setup_test_db().await;
    let user_key = b"user_profile_test_123";
    let timestamp = 1234567890;

    // First, register the user
    let registration_entry = TestDataFactory::registration_entry("UserRegister", user_key);
    assert!(db.insert_entries(vec![registration_entry]).await.is_ok());

    // Insert user profile
    sqlx::query(
        "INSERT INTO user_profiles (pubkey_hash, display_name, bio, avatar_url, updated_at_ns)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&user_key[..])
    .bind("Test User")
    .bind("A test user bio")
    .bind("https://example.com/avatar.png")
    .bind(timestamp as i64)
    .execute(&db.pool)
    .await
    .unwrap();

    // Retrieve user profile
    let profile = db.get_user_profile(user_key).await.unwrap();
    assert!(profile.is_some());

    let profile = profile.unwrap();
    assert_eq!(profile.pubkey_hash, user_key);
    assert_eq!(profile.display_name, Some("Test User".to_string()));
    assert_eq!(profile.bio, Some("A test user bio".to_string()));
    assert_eq!(
        profile.avatar_url,
        Some("https://example.com/avatar.png".to_string())
    );
    assert_eq!(profile.updated_at_ns, timestamp as i64);
}

#[tokio::test]
async fn test_user_profile_not_found() {
    let db = setup_test_db().await;
    let nonexistent_key = b"nonexistent_user";

    let profile = db.get_user_profile(nonexistent_key).await.unwrap();
    assert!(profile.is_none());
}

#[tokio::test]
async fn test_user_contacts_storage_and_retrieval() {
    let db = setup_test_db().await;
    let user_key = b"user_contacts_test_123";
    let timestamp = 1234567890;

    // Register user first
    let registration_entry = TestDataFactory::registration_entry("UserRegister", user_key);
    assert!(db.insert_entries(vec![registration_entry]).await.is_ok());

    // Insert multiple contacts
    let contacts = vec![
        ("email", "test@example.com", true),
        ("telegram", "@testuser", false),
        ("phone", "+1234567890", true),
    ];

    for (contact_type, contact_value, verified) in &contacts {
        sqlx::query(
            "INSERT INTO user_contacts (user_pubkey_hash, contact_type, contact_value, verified, created_at_ns)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user_key[..])
        .bind(contact_type)
        .bind(contact_value)
        .bind(verified)
        .bind(timestamp as i64)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Retrieve contacts
    let retrieved = db.get_user_contacts(user_key).await.unwrap();
    assert_eq!(retrieved.len(), 3);

    // Verify all contacts are present
    assert!(retrieved
        .iter()
        .any(|c| c.contact_type == "email" && c.contact_value == "test@example.com" && c.verified));
    assert!(retrieved
        .iter()
        .any(|c| c.contact_type == "telegram" && c.contact_value == "@testuser" && !c.verified));
    assert!(retrieved
        .iter()
        .any(|c| c.contact_type == "phone" && c.contact_value == "+1234567890" && c.verified));
}

#[tokio::test]
async fn test_user_socials_storage_and_retrieval() {
    let db = setup_test_db().await;
    let user_key = b"user_socials_test_123";
    let timestamp = 1234567890;

    // Register user first
    let registration_entry = TestDataFactory::registration_entry("UserRegister", user_key);
    assert!(db.insert_entries(vec![registration_entry]).await.is_ok());

    // Insert social accounts
    let socials = vec![
        ("twitter", "testuser", Some("https://twitter.com/testuser")),
        ("github", "testuser", Some("https://github.com/testuser")),
        ("discord", "testuser#1234", None),
    ];

    for (platform, username, profile_url) in &socials {
        sqlx::query(
            "INSERT INTO user_socials (user_pubkey_hash, platform, username, profile_url, created_at_ns)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&user_key[..])
        .bind(platform)
        .bind(username)
        .bind(profile_url)
        .bind(timestamp as i64)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Retrieve socials
    let retrieved = db.get_user_socials(user_key).await.unwrap();
    assert_eq!(retrieved.len(), 3);

    // Verify all socials are present
    assert!(retrieved.iter().any(|s| s.platform == "twitter"
        && s.username == "testuser"
        && s.profile_url == Some("https://twitter.com/testuser".to_string())));
    assert!(retrieved.iter().any(|s| s.platform == "github"
        && s.username == "testuser"
        && s.profile_url == Some("https://github.com/testuser".to_string())));
    assert!(retrieved.iter().any(|s| s.platform == "discord"
        && s.username == "testuser#1234"
        && s.profile_url.is_none()));
}

#[tokio::test]
async fn test_user_public_keys_storage_and_retrieval() {
    let db = setup_test_db().await;
    let user_key = b"user_keys_test_123";
    let timestamp = 1234567890;

    // Register user first
    let registration_entry = TestDataFactory::registration_entry("UserRegister", user_key);
    assert!(db.insert_entries(vec![registration_entry]).await.is_ok());

    // Insert public keys
    let keys = vec![
        (
            "ssh-ed25519",
            "AAAAC3NzaC1lZDI1NTE5AAAAI...",
            Some("SHA256:abc123"),
            Some("Work laptop"),
        ),
        (
            "ssh-rsa",
            "AAAAB3NzaC1yc2EAAAADAQAB...",
            Some("SHA256:def456"),
            Some("Home desktop"),
        ),
        ("gpg", "-----BEGIN PGP PUBLIC KEY BLOCK-----...", None, None),
    ];

    for (key_type, key_data, fingerprint, label) in &keys {
        sqlx::query(
            "INSERT INTO user_public_keys (user_pubkey_hash, key_type, key_data, key_fingerprint, label, created_at_ns)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&user_key[..])
        .bind(key_type)
        .bind(key_data)
        .bind(fingerprint)
        .bind(label)
        .bind(timestamp as i64)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Retrieve keys
    let retrieved = db.get_user_public_keys(user_key).await.unwrap();
    assert_eq!(retrieved.len(), 3);

    // Verify all keys are present
    assert!(retrieved.iter().any(|k| k.key_type == "ssh-ed25519"
        && k.key_fingerprint == Some("SHA256:abc123".to_string())
        && k.label == Some("Work laptop".to_string())));
    assert!(retrieved.iter().any(|k| k.key_type == "ssh-rsa"
        && k.key_fingerprint == Some("SHA256:def456".to_string())
        && k.label == Some("Home desktop".to_string())));
    assert!(retrieved
        .iter()
        .any(|k| k.key_type == "gpg" && k.key_fingerprint.is_none() && k.label.is_none()));
}

#[tokio::test]
async fn test_user_contacts_empty_for_nonexistent_user() {
    let db = setup_test_db().await;
    let nonexistent_key = b"nonexistent_user";

    let contacts = db.get_user_contacts(nonexistent_key).await.unwrap();
    assert_eq!(contacts.len(), 0);
}

#[tokio::test]
async fn test_user_socials_empty_for_nonexistent_user() {
    let db = setup_test_db().await;
    let nonexistent_key = b"nonexistent_user";

    let socials = db.get_user_socials(nonexistent_key).await.unwrap();
    assert_eq!(socials.len(), 0);
}

#[tokio::test]
async fn test_user_keys_empty_for_nonexistent_user() {
    let db = setup_test_db().await;
    let nonexistent_key = b"nonexistent_user";

    let keys = db.get_user_public_keys(nonexistent_key).await.unwrap();
    assert_eq!(keys.len(), 0);
}
