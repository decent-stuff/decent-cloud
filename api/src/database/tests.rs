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

/// Test helper to count rows in a table, excluding example provider data
async fn count_table_rows(db: &Database, table: &str) -> i64 {
    // Example provider pubkey hash from migration 002
    let example_pubkey_hash =
        hex::decode("6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572")
            .unwrap();

    // For provider tables, exclude the example provider
    let query = if table.starts_with("provider_") {
        format!(
            "SELECT COUNT(*) as count FROM {} WHERE pubkey_hash != ?",
            table
        )
    } else {
        format!("SELECT COUNT(*) as count FROM {}", table)
    };

    let result = if table.starts_with("provider_") {
        sqlx::query(&query)
            .bind(example_pubkey_hash)
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

// Core functionality tests
// Test database setup - using in-memory for simplicity and fast, isolated tests.
async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect(":memory:").await.unwrap();

    // Load and execute migrations SQL at runtime for production-like schema
    let migration1_sql = include_str!("../../migrations/001_original_schema.sql");
    sqlx::query(migration1_sql).execute(&pool).await.unwrap();
    // Add more migrations below as needed

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

// Tests for automatic user registration
#[tokio::test]
async fn test_upsert_profile_auto_creates_registration() {
    let db = setup_test_db().await;
    let new_user_key = b"new_user_profile";

    // Upsert profile without prior registration - should auto-create registration
    let result = db
        .upsert_user_profile(
            new_user_key,
            Some("Test User"),
            Some("Bio"),
            Some("https://example.com/avatar.png"),
        )
        .await;

    assert!(result.is_ok());

    // Verify profile was created
    let profile = db.get_user_profile(new_user_key).await.unwrap();
    assert!(profile.is_some());
    assert_eq!(profile.unwrap().display_name, Some("Test User".to_string()));

    // Verify registration was auto-created
    let reg_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_registrations WHERE pubkey_hash = ?")
            .bind(&new_user_key[..])
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(reg_count.0, 1);
}

#[tokio::test]
async fn test_upsert_contact_auto_creates_registration() {
    let db = setup_test_db().await;
    let new_user_key = b"new_user_contact";

    let result = db
        .upsert_user_contact(new_user_key, "email", "test@example.com", false)
        .await;

    assert!(result.is_ok());

    // Verify registration was auto-created
    let reg_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_registrations WHERE pubkey_hash = ?")
            .bind(&new_user_key[..])
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(reg_count.0, 1);
}

#[tokio::test]
async fn test_upsert_social_auto_creates_registration() {
    let db = setup_test_db().await;
    let new_user_key = b"new_user_social";

    let result = db
        .upsert_user_social(
            new_user_key,
            "twitter",
            "@testuser",
            Some("https://twitter.com/testuser"),
        )
        .await;

    assert!(result.is_ok());

    // Verify registration was auto-created
    let reg_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_registrations WHERE pubkey_hash = ?")
            .bind(&new_user_key[..])
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(reg_count.0, 1);
}

#[tokio::test]
async fn test_add_public_key_auto_creates_registration() {
    let db = setup_test_db().await;
    let new_user_key = b"new_user_key";

    let result = db
        .add_user_public_key(
            new_user_key,
            "ssh-ed25519",
            "AAAAC3NzaC1lZDI1NTE5...",
            Some("SHA256:abc123"),
            Some("Test key"),
        )
        .await;

    assert!(result.is_ok());

    // Verify registration was auto-created
    let reg_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM user_registrations WHERE pubkey_hash = ?")
            .bind(&new_user_key[..])
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(reg_count.0, 1);
}

// Example offerings tests
#[tokio::test]
async fn test_get_example_offerings() {
    let db = setup_test_db().await;

    // Test retrieving example offerings
    let example_offerings = db.get_example_offerings().await.unwrap();

    // Should have exactly 2 example offerings from migration
    assert_eq!(example_offerings.len(), 2);

    // Verify first example offering (ds-premium-002, comes first alphabetically)
    let ds_offering = &example_offerings[0];
    assert_eq!(ds_offering.offering_id, "ds-premium-002");
    assert_eq!(ds_offering.offer_name, "Premium Dedicated Server");
    assert_eq!(ds_offering.monthly_price, 299.99);
    assert_eq!(ds_offering.currency, "USD");
    assert_eq!(ds_offering.product_type, "dedicated");

    // Verify second example offering (vm-basic-001)
    let vm_offering = &example_offerings[1];
    assert_eq!(vm_offering.offering_id, "vm-basic-001");
    assert_eq!(vm_offering.offer_name, "Basic Virtual Machine");
    assert_eq!(vm_offering.monthly_price, 29.99);
    assert_eq!(vm_offering.currency, "USD");
    assert_eq!(vm_offering.product_type, "compute");

    // Test that related data exists for each offering
    let vm_payment_methods: Vec<&str> = vm_offering
        .payment_methods
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(vm_payment_methods.len(), 2);
    assert!(vm_payment_methods.contains(&"Credit Card"));
    assert!(vm_payment_methods.contains(&"PayPal"));

    let vm_features: Vec<&str> = vm_offering
        .features
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(vm_features.len(), 3);
    assert!(vm_features.contains(&"Auto Backup"));
    assert!(vm_features.contains(&"SSH Access"));
    assert!(vm_features.contains(&"Root Access"));

    let vm_os: Vec<&str> = vm_offering
        .operating_systems
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(vm_os.len(), 3);
    assert!(vm_os.contains(&"Ubuntu 22.04"));
    assert!(vm_os.contains(&"Debian 11"));
    assert!(vm_os.contains(&"CentOS 8"));

    let ds_payment_methods: Vec<&str> = ds_offering
        .payment_methods
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(ds_payment_methods.len(), 3);
    assert!(ds_payment_methods.contains(&"BTC"));
    assert!(ds_payment_methods.contains(&"Bank Transfer"));
    assert!(ds_payment_methods.contains(&"Credit Card"));

    let ds_features: Vec<&str> = ds_offering
        .features
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(ds_features.len(), 4);
    assert!(ds_features.contains(&"RAID 1"));
    assert!(ds_features.contains(&"IPMI Access"));
    assert!(ds_features.contains(&"DDoS Protection"));
    assert!(ds_features.contains(&"24/7 Support"));

    let ds_os: Vec<&str> = ds_offering
        .operating_systems
        .as_deref()
        .unwrap_or("")
        .split(',')
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(ds_os.len(), 4);
    assert!(ds_os.contains(&"Ubuntu 22.04"));
    assert!(ds_os.contains(&"CentOS 8"));
    assert!(ds_os.contains(&"Windows Server 2022"));
    assert!(ds_os.contains(&"Debian 11"));
}

#[tokio::test]
async fn test_csv_template_data_retrieval() {
    let db = setup_test_db().await;

    // Verify we can retrieve all data needed for CSV template generation
    let example_offerings = db.get_example_offerings().await.unwrap();
    assert_eq!(example_offerings.len(), 2);

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

        let operating_systems = offering.operating_systems.as_deref().unwrap_or("");
        assert!(
            !operating_systems.is_empty(),
            "Operating systems should not be empty for {}",
            offering.offering_id
        );
    }

    // Verify example offerings have correct visibility
    for offering in &example_offerings {
        assert_eq!(
            offering.visibility, "example",
            "Example offerings should have visibility='example'"
        );
    }
}

#[tokio::test]
async fn test_example_offerings_excluded_from_search() {
    let db = setup_test_db().await;

    // Create a regular public offering
    let pubkey_hash = vec![1u8; 32];
    sqlx::query("INSERT INTO provider_registrations (pubkey_hash, pubkey_bytes, signature, created_at_ns) VALUES (?, ?, ?, 0)")
        .bind(&pubkey_hash).bind(&pubkey_hash).bind(&pubkey_hash).execute(&db.pool).await.unwrap();

    sqlx::query("INSERT INTO provider_offerings (pubkey_hash, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'test-public-001', 'Test Public Offering', 'USD', 99.99, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'Test City', 0, 0)")
        .bind(&pubkey_hash).execute(&db.pool).await.unwrap();

    // Search offerings - should only return the public offering, not examples
    let search_params = crate::database::offerings::SearchOfferingsParams {
        product_type: None,
        country: None,
        in_stock_only: false,
        limit: 10,
        offset: 0,
    };

    let search_results = db.search_offerings(search_params).await.unwrap();
    assert_eq!(
        search_results.len(),
        1,
        "Search should only return 1 public offering, not example offerings"
    );
    assert_eq!(search_results[0].offering_id, "test-public-001");
    assert_eq!(search_results[0].visibility, "public");

    // Verify count_offerings also excludes examples
    let total_count = db.count_offerings(None).await.unwrap();
    assert_eq!(
        total_count, 1,
        "Count should only include public offerings, not examples"
    );
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
            (now_ns - 1 * 3600 * 1_000_000_000) as u64, // 1 hour ago
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
        .find(|v| v.pubkey_hash == validator1)
        .expect("Validator 1 should be in results");

    assert_eq!(v1.total_check_ins, 3, "Validator 1 should have 3 total check-ins");
    assert_eq!(v1.check_ins_24h, 2, "Validator 1 should have 2 check-ins in last 24h");
    assert_eq!(v1.check_ins_7d, 3, "Validator 1 should have 3 check-ins in last 7d");
    assert_eq!(v1.check_ins_30d, 3, "Validator 1 should have 3 check-ins in last 30d");

    // Find validator2 in results
    let v2 = validators_30d
        .iter()
        .find(|v| v.pubkey_hash == validator2)
        .expect("Validator 2 should be in results");

    assert_eq!(v2.total_check_ins, 1, "Validator 2 should have 1 total check-in");
    assert_eq!(v2.check_ins_24h, 0, "Validator 2 should have 0 check-ins in last 24h");
    assert_eq!(v2.check_ins_7d, 0, "Validator 2 should have 0 check-ins in last 7d");
    assert_eq!(v2.check_ins_30d, 1, "Validator 2 should have 1 check-in in last 30d");

    // Test: Get validators active in last 7 days (should only have validator1)
    let validators_7d = db.get_active_validators(7).await.unwrap();
    assert_eq!(
        validators_7d.len(),
        1,
        "Should have 1 validator active in last 7 days"
    );
    assert_eq!(
        validators_7d[0].pubkey_hash, validator1,
        "Only validator 1 should be active in last 7 days"
    );

    // Test: Get validators active in last 1 day (should only have validator1)
    let validators_1d = db.get_active_validators(1).await.unwrap();
    assert_eq!(
        validators_1d.len(),
        1,
        "Should have 1 validator active in last 24 hours"
    );
    assert_eq!(validators_1d[0].check_ins_24h, 2, "Should have 2 check-ins in 24h");
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
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey_hash, name, description, website_url, logo_url, why_choose_us, api_version, profile_version, updated_at_ns)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&validator_key[..])
    .bind("Test Validator")
    .bind(Some("A test validator"))
    .bind(Some("https://example.com"))
    .bind(Some("https://example.com/logo.png"))
    .bind(Some("We're reliable!"))
    .bind("v1")
    .bind("0.1.0")
    .bind(now_ns)
    .execute(&db.pool)
    .await
    .unwrap();

    // Get active validators
    let validators = db.get_active_validators(1).await.unwrap();
    assert_eq!(validators.len(), 1);

    let validator = &validators[0];
    assert_eq!(validator.name, Some("Test Validator".to_string()));
    assert_eq!(validator.description, Some("A test validator".to_string()));
    assert_eq!(validator.website_url, Some("https://example.com".to_string()));
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
