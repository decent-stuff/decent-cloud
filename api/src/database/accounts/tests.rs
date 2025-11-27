use super::*;
use sqlx::SqlitePool;

async fn create_test_db() -> Database {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();
    Database { pool }
}

#[tokio::test]
async fn test_create_account() {
    let db = create_test_db().await;
    let username = "alice";
    let public_key = [1u8; 32];

    let account = db.create_account(username, &public_key).await.unwrap();

    assert_eq!(account.username, username);
    assert_eq!(account.id.len(), 16);
}

#[tokio::test]
async fn test_get_account_by_username() {
    let db = create_test_db().await;
    let username = "bob";
    let public_key = [2u8; 32];

    db.create_account(username, &public_key).await.unwrap();

    let fetched = db.get_account_by_username(username).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().username, username);
}

#[tokio::test]
async fn test_get_account_with_keys() {
    let db = create_test_db().await;
    let username = "charlie";
    let public_key = [3u8; 32];

    db.create_account(username, &public_key).await.unwrap();

    let account_with_keys = db.get_account_with_keys(username).await.unwrap();
    assert!(account_with_keys.is_some());

    let account = account_with_keys.unwrap();
    assert_eq!(account.username, username);
    assert_eq!(account.public_keys.len(), 1);
    assert!(account.public_keys[0].is_active);
}

#[tokio::test]
async fn test_add_account_key() {
    let db = create_test_db().await;
    let username = "dave";
    let public_key1 = [4u8; 32];
    let public_key2 = [5u8; 32];

    let account = db.create_account(username, &public_key1).await.unwrap();

    let new_key = db.add_account_key(&account.id, &public_key2).await.unwrap();
    assert_eq!(new_key.public_key, public_key2);

    let keys = db.get_account_keys(&account.id).await.unwrap();
    assert_eq!(keys.len(), 2);
}

#[tokio::test]
async fn test_max_keys_limit() {
    let db = create_test_db().await;
    let username = "eve";
    let public_key1 = [6u8; 32];

    let account = db.create_account(username, &public_key1).await.unwrap();

    // Add 9 more keys (total 10)
    for i in 0..9 {
        let mut pk = [7u8; 32];
        pk[0] = i;
        db.add_account_key(&account.id, &pk).await.unwrap();
    }

    // Try to add 11th key - should fail
    let public_key11 = [11u8; 32];
    let result = db.add_account_key(&account.id, &public_key11).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_disable_account_key() {
    let db = create_test_db().await;
    let username = "frank";
    let public_key1 = [8u8; 32];
    let public_key2 = [9u8; 32];

    let account = db.create_account(username, &public_key1).await.unwrap();
    let key2 = db.add_account_key(&account.id, &public_key2).await.unwrap();

    let keys = db.get_account_keys(&account.id).await.unwrap();
    let key1_id = keys[0].id.clone();

    // Disable key2 using key1
    db.disable_account_key(&key2.id, &key1_id).await.unwrap();

    let keys_after = db.get_account_keys(&account.id).await.unwrap();
    assert_eq!(keys_after.len(), 2);

    // Find the keys by their IDs
    let key1_after = keys_after.iter().find(|k| k.id == key1_id).unwrap();
    let key2_after = keys_after.iter().find(|k| k.id == key2.id).unwrap();

    assert_eq!(key1_after.is_active, 1, "Key1 should still be active");
    assert_eq!(key2_after.is_active, 0, "Key2 should be disabled");
    assert!(
        key2_after.disabled_at.is_some(),
        "Key2 should have disabled_at set"
    );
    assert_eq!(
        key2_after.disabled_by_key_id,
        Some(key1_id.clone()),
        "Key2 should be disabled by key1"
    );
}

#[tokio::test]
async fn test_cannot_disable_last_key() {
    let db = create_test_db().await;
    let username = "grace";
    let public_key = [10u8; 32];

    let account = db.create_account(username, &public_key).await.unwrap();
    let keys = db.get_account_keys(&account.id).await.unwrap();
    let key_id = keys[0].id.clone();

    // Try to disable the only key - should fail
    let result = db.disable_account_key(&key_id, &key_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_check_nonce_exists() {
    let db = create_test_db().await;
    let nonce = uuid::Uuid::new_v4();

    // Initially nonce should not exist
    let exists = db.check_nonce_exists(&nonce, 10).await.unwrap();
    assert!(!exists);

    // Insert audit record with nonce
    let public_key = [11u8; 32];
    let signature = [0u8; 64];
    let timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();

    db.insert_signature_audit(
        None,
        "test_action",
        "{}",
        &signature,
        &public_key,
        timestamp,
        &nonce,
        false,
    )
    .await
    .unwrap();

    // Now nonce should exist
    let exists = db.check_nonce_exists(&nonce, 10).await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_signature_audit_cleanup() {
    let db = create_test_db().await;
    let public_key = [22u8; 32];
    let signature = [0u8; 64];
    let old_nonce = uuid::Uuid::new_v4();
    let recent_nonce = uuid::Uuid::new_v4();

    // Calculate timestamps
    let old_created_at =
        chrono::Utc::now().timestamp_nanos_opt().unwrap() - (200 * 24 * 60 * 60 * 1_000_000_000);
    let recent_created_at =
        chrono::Utc::now().timestamp_nanos_opt().unwrap() - (10 * 24 * 60 * 60 * 1_000_000_000);
    let client_timestamp = chrono::Utc::now().timestamp_nanos_opt().unwrap();

    // Insert old audit record directly with SQL to control created_at
    sqlx::query(
        "INSERT INTO signature_audit
         (account_id, action, payload, signature, public_key, timestamp, nonce, is_admin_action, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(None::<&[u8]>)
    .bind("old_action")
    .bind("{}")
    .bind(&signature[..])
    .bind(&public_key[..])
    .bind(client_timestamp)
    .bind(&old_nonce.as_bytes()[..])
    .bind(0)
    .bind(old_created_at)
    .execute(&db.pool)
    .await
    .unwrap();

    // Insert recent audit record directly with SQL to control created_at
    sqlx::query(
        "INSERT INTO signature_audit
         (account_id, action, payload, signature, public_key, timestamp, nonce, is_admin_action, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(None::<&[u8]>)
    .bind("recent_action")
    .bind("{}")
    .bind(&signature[..])
    .bind(&public_key[..])
    .bind(client_timestamp)
    .bind(&recent_nonce.as_bytes()[..])
    .bind(0)
    .bind(recent_created_at)
    .execute(&db.pool)
    .await
    .unwrap();

    // Clean up records older than 180 days
    let deleted_count = db.cleanup_signature_audit(180).await.unwrap();
    assert_eq!(deleted_count, 1, "Should delete 1 old record");

    // Verify old nonce no longer exists
    let old_exists = db
        .check_nonce_exists(&old_nonce, 365 * 24 * 60)
        .await
        .unwrap();
    assert!(!old_exists, "Old nonce should not exist after cleanup");

    // Verify recent nonce still exists
    let recent_exists = db
        .check_nonce_exists(&recent_nonce, 365 * 24 * 60)
        .await
        .unwrap();
    assert!(recent_exists, "Recent nonce should still exist");
}

#[tokio::test]
async fn test_create_oauth_account() {
    let db = create_test_db().await;

    // Create a regular account first
    let account = db.create_account("testuser", &[0u8; 32]).await.unwrap();

    // Create OAuth link
    let oauth_acc = db
        .create_oauth_account(
            &account.id,
            "google_oauth",
            "google_user_123",
            Some("test@example.com"),
        )
        .await
        .unwrap();

    assert_eq!(oauth_acc.provider, "google_oauth");
    assert_eq!(oauth_acc.external_id, "google_user_123");
    assert_eq!(oauth_acc.email, Some("test@example.com".to_string()));
    assert_eq!(oauth_acc.account_id, account.id);
}

#[tokio::test]
async fn test_create_oauth_account_duplicate_external_id() {
    let db = create_test_db().await;

    let account = db.create_account("testuser", &[0u8; 32]).await.unwrap();

    // Create first OAuth link
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_user_123",
        Some("test@example.com"),
    )
    .await
    .unwrap();

    // Try to create duplicate with same provider + external_id
    let result = db
        .create_oauth_account(
            &account.id,
            "google_oauth",
            "google_user_123",
            Some("other@example.com"),
        )
        .await;

    assert!(
        result.is_err(),
        "Should fail on duplicate provider + external_id"
    );
}

#[tokio::test]
async fn test_get_oauth_account() {
    let db = create_test_db().await;

    let account = db.create_account("testuser", &[0u8; 32]).await.unwrap();

    // Create OAuth link
    let created = db
        .create_oauth_account(
            &account.id,
            "google_oauth",
            "google_user_456",
            Some("test@example.com"),
        )
        .await
        .unwrap();

    // Fetch by OAuth ID
    let fetched = db.get_oauth_account(&created.id).await.unwrap();

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.account_id, account.id);
    assert_eq!(fetched.provider, "google_oauth");
    assert_eq!(fetched.external_id, "google_user_456");
}

#[tokio::test]
async fn test_get_oauth_account_not_found() {
    let db = create_test_db().await;

    let result = db.get_oauth_account(&[1u8; 16]).await.unwrap();

    assert!(
        result.is_none(),
        "Should return None for nonexistent OAuth account ID"
    );
}

#[tokio::test]
async fn test_get_oauth_account_by_provider_and_external_id() {
    let db = create_test_db().await;

    let account = db.create_account("testuser", &[0u8; 32]).await.unwrap();

    // Create OAuth link
    let created = db
        .create_oauth_account(
            &account.id,
            "google_oauth",
            "google_user_456",
            Some("test@example.com"),
        )
        .await
        .unwrap();

    // Fetch by provider and external_id
    let fetched = db
        .get_oauth_account_by_provider_and_external_id("google_oauth", "google_user_456")
        .await
        .unwrap();

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.account_id, account.id);
    assert_eq!(fetched.external_id, "google_user_456");
}

#[tokio::test]
async fn test_get_oauth_account_by_provider_and_external_id_not_found() {
    let db = create_test_db().await;

    let result = db
        .get_oauth_account_by_provider_and_external_id("google_oauth", "nonexistent")
        .await
        .unwrap();

    assert!(
        result.is_none(),
        "Should return None for nonexistent OAuth account"
    );
}

#[tokio::test]
async fn test_get_account_by_email() {
    let db = create_test_db().await;

    // Create account with email via OAuth
    let pubkey = [1u8; 32];
    let (account, _oauth) = db
        .create_oauth_linked_account(
            "emailuser",
            &pubkey,
            "user@example.com",
            "google_oauth",
            "google_789",
        )
        .await
        .unwrap();

    // Fetch by email
    let fetched = db.get_account_by_email("user@example.com").await.unwrap();

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, account.id);
    assert_eq!(fetched.username, "emailuser");
    assert_eq!(fetched.email, Some("user@example.com".to_string()));
}

#[tokio::test]
async fn test_get_account_by_email_not_found() {
    let db = create_test_db().await;

    let result = db
        .get_account_by_email("nonexistent@example.com")
        .await
        .unwrap();

    assert!(result.is_none(), "Should return None for nonexistent email");
}

#[tokio::test]
async fn test_create_oauth_linked_account() {
    let db = create_test_db().await;

    let pubkey = [2u8; 32];
    let (account, oauth_acc) = db
        .create_oauth_linked_account(
            "newuser",
            &pubkey,
            "newuser@example.com",
            "google_oauth",
            "google_new_123",
        )
        .await
        .unwrap();

    // Verify account
    assert_eq!(account.username, "newuser");
    assert_eq!(account.email, Some("newuser@example.com".to_string()));
    assert_eq!(account.auth_provider, "google_oauth");

    // Verify account has the key
    let account_with_keys = db.get_account_with_keys("newuser").await.unwrap().unwrap();
    assert_eq!(account_with_keys.public_keys.len(), 1);
    assert_eq!(
        hex::decode(&account_with_keys.public_keys[0].public_key).unwrap(),
        pubkey.to_vec()
    );

    // Verify OAuth link
    assert_eq!(oauth_acc.account_id, account.id);
    assert_eq!(oauth_acc.provider, "google_oauth");
    assert_eq!(oauth_acc.external_id, "google_new_123");
    assert_eq!(oauth_acc.email, Some("newuser@example.com".to_string()));
}

#[tokio::test]
async fn test_create_oauth_linked_account_duplicate_username() {
    let db = create_test_db().await;

    // Create first account
    db.create_account("duplicate", &[1u8; 32]).await.unwrap();

    // Try to create OAuth account with same username
    let result = db
        .create_oauth_linked_account(
            "duplicate",
            &[2u8; 32],
            "new@example.com",
            "google_oauth",
            "google_unique",
        )
        .await;

    assert!(result.is_err(), "Should fail on duplicate username");
}

#[tokio::test]
async fn test_create_oauth_linked_account_transaction_rollback() {
    let db = create_test_db().await;

    // Create an account with a specific external_id
    let pubkey = [3u8; 32];
    db.create_oauth_linked_account(
        "first",
        &pubkey,
        "first@example.com",
        "google_oauth",
        "existing_external_id",
    )
    .await
    .expect("First account creation should succeed");

    // Try to create another with duplicate external_id (should violate UNIQUE constraint on oauth_accounts)
    let result = db
        .create_oauth_linked_account(
            "second",
            &[4u8; 32],
            "second@example.com",
            "google_oauth",
            "existing_external_id",
        )
        .await;

    assert!(
        result.is_err(),
        "Should fail due to duplicate external_id in oauth_accounts"
    );

    // Verify transaction was rolled back - account "second" should not exist
    let account_check = db
        .get_account_by_username("second")
        .await
        .expect("Query should succeed");
    assert!(
        account_check.is_none(),
        "Account 'second' should not exist after transaction rollback"
    );
}

#[tokio::test]
async fn test_usernames_preserve_case_but_unique_case_insensitive() {
    let db = create_test_db().await;

    // Create account with specific case
    let account = db
        .create_account("AliceWonderland", &[1u8; 32])
        .await
        .unwrap();

    // Verify username is stored with original case
    assert_eq!(account.username, "AliceWonderland");

    // Try to create account with same name but different case - should fail
    let result_lower = db.create_account("alicewonderland", &[2u8; 32]).await;
    let result_upper = db.create_account("ALICEWONDERLAND", &[3u8; 32]).await;
    let result_mixed = db.create_account("aLiCeWoNdErLaNd", &[4u8; 32]).await;

    // All should fail due to case-insensitive unique constraint
    assert!(
        result_lower.is_err(),
        "Should not allow duplicate username with different case"
    );
    assert!(
        result_upper.is_err(),
        "Should not allow duplicate username with different case"
    );
    assert!(
        result_mixed.is_err(),
        "Should not allow duplicate username with different case"
    );
}

#[tokio::test]
async fn test_username_search_is_case_insensitive() {
    let db = create_test_db().await;

    // Create account with mixed case
    let _account = db
        .create_account("AliceWonderland", &[1u8; 32])
        .await
        .unwrap();

    // Search with different cases should all find the same account
    let found_lower = db.get_account_by_username("alicewonderland").await.unwrap();
    let found_upper = db.get_account_by_username("ALICEWONDERLAND").await.unwrap();
    let found_mixed = db.get_account_by_username("AliceWonderland").await.unwrap();
    let found_other = db.get_account_by_username("aLiCeWoNdErLaNd").await.unwrap();

    assert!(found_lower.is_some());
    assert!(found_upper.is_some());
    assert!(found_mixed.is_some());
    assert!(found_other.is_some());

    // All should return the same account with original case
    assert_eq!(found_lower.unwrap().username, "AliceWonderland");
    assert_eq!(found_upper.unwrap().username, "AliceWonderland");
    assert_eq!(found_mixed.unwrap().username, "AliceWonderland");
    assert_eq!(found_other.unwrap().username, "AliceWonderland");
}
