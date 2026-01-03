use crate::database::test_helpers::setup_test_db;

#[tokio::test]
async fn test_create_account() {
    let db = setup_test_db().await;
    let username = "alice";
    let public_key = [1u8; 32];

    let account = db
        .create_account(username, &public_key, "test@example.com")
        .await
        .expect("Failed to create account");

    assert_eq!(account.username, username);
    assert_eq!(account.id.len(), 16);
}

#[tokio::test]
async fn test_get_account_by_username() {
    let db = setup_test_db().await;
    let username = "bob";
    let public_key = [2u8; 32];

    db.create_account(username, &public_key, "test@example.com")
        .await
        .expect("Failed to create account");

    let fetched = db.get_account_by_username(username).await.unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().username, username);
}

#[tokio::test]
async fn test_get_account_with_keys() {
    let db = setup_test_db().await;
    let username = "charlie";
    let public_key = [3u8; 32];

    db.create_account(username, &public_key, "test@example.com")
        .await
        .expect("Failed to create account");

    let account_with_keys = db.get_account_with_keys(username).await.unwrap();
    assert!(account_with_keys.is_some());

    let account = account_with_keys.unwrap();
    assert_eq!(account.username, username);
    assert_eq!(account.public_keys.len(), 1);
    assert!(account.public_keys[0].is_active);
}

#[tokio::test]
async fn test_add_account_key() {
    let db = setup_test_db().await;
    let username = "dave";
    let public_key1 = [4u8; 32];
    let public_key2 = [5u8; 32];

    let account = db
        .create_account(username, &public_key1, "test@example.com")
        .await
        .expect("Failed to create account");

    let new_key = db.add_account_key(&account.id, &public_key2).await.unwrap();
    assert_eq!(new_key.public_key, public_key2);

    let keys = db.get_account_keys(&account.id).await.unwrap();
    assert_eq!(keys.len(), 2);
}

#[tokio::test]
async fn test_max_keys_limit() {
    let db = setup_test_db().await;
    let username = "eve";
    let public_key1 = [6u8; 32];

    let account = db
        .create_account(username, &public_key1, "test@example.com")
        .await
        .expect("Failed to create account");

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
    let db = setup_test_db().await;
    let username = "frank";
    let public_key1 = [8u8; 32];
    let public_key2 = [9u8; 32];

    let account = db
        .create_account(username, &public_key1, "test@example.com")
        .await
        .expect("Failed to create account");
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

    assert_eq!(key1_after.is_active, true, "Key1 should still be active");
    assert_eq!(key2_after.is_active, false, "Key2 should be disabled");
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
    let db = setup_test_db().await;
    let username = "grace";
    let public_key = [10u8; 32];

    let account = db
        .create_account(username, &public_key, "test@example.com")
        .await
        .expect("Failed to create account");
    let keys = db.get_account_keys(&account.id).await.unwrap();
    let key_id = keys[0].id.clone();

    // Try to disable the only key - should fail
    let result = db.disable_account_key(&key_id, &key_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_check_nonce_exists() {
    let db = setup_test_db().await;
    let nonce = uuid::Uuid::new_v4();

    // Initially nonce should not exist
    let exists = db.check_nonce_exists(&nonce, 10).await.unwrap();
    assert!(!exists);

    // Insert audit record with nonce
    let public_key = [11u8; 32];
    let signature = [0u8; 64];
    let timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("Failed to get timestamp");

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
    .expect("Failed to insert signature audit");

    // Now nonce should exist
    let exists = db.check_nonce_exists(&nonce, 10).await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_signature_audit_cleanup() {
    let db = setup_test_db().await;
    let public_key = [22u8; 32];
    let signature = [0u8; 64];
    let old_nonce = uuid::Uuid::new_v4();
    let recent_nonce = uuid::Uuid::new_v4();

    // Calculate timestamps
    let old_created_at = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("Failed to get timestamp")
        - (200 * 24 * 60 * 60 * 1_000_000_000);
    let recent_created_at = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("Failed to get timestamp")
        - (10 * 24 * 60 * 60 * 1_000_000_000);
    let client_timestamp = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("Failed to get timestamp");

    // Insert old audit record directly with SQL to control created_at
    sqlx::query(
        "INSERT INTO signature_audit
         (account_id, action, payload, signature, public_key, timestamp, nonce, is_admin_action, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(None::<&[u8]>)
    .bind("old_action")
    .bind("{}")
    .bind(&signature[..])
    .bind(&public_key[..])
    .bind(client_timestamp)
    .bind(&old_nonce.as_bytes()[..])
    .bind(false)
    .bind(old_created_at)
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    // Insert recent audit record directly with SQL to control created_at
    sqlx::query(
        "INSERT INTO signature_audit
         (account_id, action, payload, signature, public_key, timestamp, nonce, is_admin_action, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(None::<&[u8]>)
    .bind("recent_action")
    .bind("{}")
    .bind(&signature[..])
    .bind(&public_key[..])
    .bind(client_timestamp)
    .bind(&recent_nonce.as_bytes()[..])
    .bind(false)
    .bind(recent_created_at)
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    // Clean up records older than 180 days
    let deleted_count = db.cleanup_signature_audit(180).await.unwrap();
    assert_eq!(deleted_count, 1, "Should delete 1 old record");

    // Verify old nonce no longer exists
    let old_exists = db
        .check_nonce_exists(&old_nonce, 365 * 24 * 60)
        .await
        .expect("Failed to check nonce existence");
    assert!(!old_exists, "Old nonce should not exist after cleanup");

    // Verify recent nonce still exists
    let recent_exists = db
        .check_nonce_exists(&recent_nonce, 365 * 24 * 60)
        .await
        .expect("Failed to check nonce existence");
    assert!(recent_exists, "Recent nonce should still exist");
}

#[tokio::test]
async fn test_create_oauth_account() {
    let db = setup_test_db().await;

    // Create a regular account first
    let account = db
        .create_account("testuser", &[0u8; 32], "test@example.com")
        .await
        .expect("Failed to create OAuth linked account");

    // Create OAuth link
    let oauth_acc = db
        .create_oauth_account(
            &account.id,
            "google_oauth",
            "google_user_123",
            Some("test@example.com"),
        )
        .await
        .expect("Failed to create OAuth linked account");

    assert_eq!(oauth_acc.provider, "google_oauth");
    assert_eq!(oauth_acc.external_id, "google_user_123");
    assert_eq!(oauth_acc.email, Some("test@example.com".to_string()));
    assert_eq!(oauth_acc.account_id, account.id);
}

#[tokio::test]
async fn test_create_oauth_account_duplicate_external_id() {
    let db = setup_test_db().await;

    let account = db
        .create_account("testuser", &[0u8; 32], "test@example.com")
        .await
        .expect("Failed to create OAuth linked account");

    // Create first OAuth link
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_user_123",
        Some("test@example.com"),
    )
    .await
    .expect("Failed to create OAuth linked account");

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
    let db = setup_test_db().await;

    let account = db
        .create_account("testuser", &[0u8; 32], "test@example.com")
        .await
        .expect("Failed to get OAuth account");

    // Create OAuth link
    let created = db
        .create_oauth_account(
            &account.id,
            "google_oauth",
            "google_user_456",
            Some("test@example.com"),
        )
        .await
        .expect("Failed to create OAuth linked account");

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
    let db = setup_test_db().await;

    let result = db.get_oauth_account(&[1u8; 16]).await.unwrap();

    assert!(
        result.is_none(),
        "Should return None for nonexistent OAuth account ID"
    );
}

#[tokio::test]
async fn test_get_oauth_account_by_provider_and_external_id() {
    let db = setup_test_db().await;

    let account = db
        .create_account("testuser", &[0u8; 32], "test@example.com")
        .await
        .expect("Failed to get OAuth account by provider and external ID");

    // Create OAuth link
    let created = db
        .create_oauth_account(
            &account.id,
            "google_oauth",
            "google_user_456",
            Some("test@example.com"),
        )
        .await
        .expect("Failed to create OAuth linked account");

    // Fetch by provider and external_id
    let fetched = db
        .get_oauth_account_by_provider_and_external_id("google_oauth", "google_user_456")
        .await
        .expect("Failed to create OAuth linked account");

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, created.id);
    assert_eq!(fetched.account_id, account.id);
    assert_eq!(fetched.external_id, "google_user_456");
}

#[tokio::test]
async fn test_get_oauth_account_by_provider_and_external_id_not_found() {
    let db = setup_test_db().await;

    let result = db
        .get_oauth_account_by_provider_and_external_id("google_oauth", "nonexistent")
        .await
        .expect("Failed to get OAuth account by provider and external ID");

    assert!(
        result.is_none(),
        "Should return None for nonexistent OAuth account"
    );
}

#[tokio::test]
async fn test_get_account_by_email() {
    let db = setup_test_db().await;

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
        .expect("Failed to create OAuth linked account");

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
    let db = setup_test_db().await;

    let result = db
        .get_account_by_email("nonexistent@example.com")
        .await
        .expect("Failed to get account by email");

    assert!(result.is_none(), "Should return None for nonexistent email");
}

#[tokio::test]
async fn test_create_oauth_linked_account() {
    let db = setup_test_db().await;

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
        .expect("Failed to create OAuth linked account");

    // Verify account
    assert_eq!(account.username, "newuser");
    assert_eq!(account.email, Some("newuser@example.com".to_string()));
    assert_eq!(account.auth_provider, "google_oauth");

    // Verify account has the key
    let account_with_keys = db.get_account_with_keys("newuser").await.unwrap().unwrap();
    assert_eq!(account_with_keys.public_keys.len(), 1);
    assert_eq!(
        hex::decode(&account_with_keys.public_keys[0].public_key)
            .expect("Failed to decode public key hex"),
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
    let db = setup_test_db().await;

    // Create first account
    db.create_account("duplicate", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create OAuth linked account");

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
    let db = setup_test_db().await;

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
    let db = setup_test_db().await;

    // Create account with specific case
    let account = db
        .create_account("AliceWonderland", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Verify username is stored with original case
    assert_eq!(account.username, "AliceWonderland");

    // Try to create account with same name but different case - should fail
    let result_lower = db
        .create_account("alicewonderland", &[2u8; 32], "test2@example.com")
        .await;
    let result_upper = db
        .create_account("ALICEWONDERLAND", &[3u8; 32], "test3@example.com")
        .await;
    let result_mixed = db
        .create_account("aLiCeWoNdErLaNd", &[4u8; 32], "test4@example.com")
        .await;

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
    let db = setup_test_db().await;

    // Create account with mixed case
    let _account = db
        .create_account("AliceWonderland", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

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

#[tokio::test]
async fn test_create_oauth_linked_account_queues_welcome_email() {
    let db = setup_test_db().await;

    let pubkey = [5u8; 32];
    let (_account, _oauth_acc) = db
        .create_oauth_linked_account(
            "emailtest",
            &pubkey,
            "emailtest@example.com",
            "google_oauth",
            "google_email_123",
        )
        .await
        .expect("Failed to create OAuth linked account");

    // Verify welcome email was queued
    let pending_emails = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending_emails.len(), 1);

    let email = &pending_emails[0];
    assert_eq!(email.to_addr, "emailtest@example.com");
    assert_eq!(email.from_addr, "noreply@decent-cloud.org");
    assert_eq!(email.subject, "Welcome to Decent Cloud");
    assert!(email.body.contains("emailtest"));
    assert!(email.body.contains("Welcome to Decent Cloud"));
    assert_eq!(email.is_html, false);
    assert_eq!(email.status, "pending");
}

#[tokio::test]
async fn test_create_email_verification_token() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Create verification token
    let token = db
        .create_email_verification_token(&account.id, "test@example.com")
        .await
        .expect("Failed to create account");

    // Token should be 16 bytes (UUID)
    assert_eq!(token.len(), 16);

    // Verify token was stored in database
    let result: Option<(Vec<u8>, Vec<u8>, String)> = sqlx::query_as(
        "SELECT token, account_id, email FROM email_verification_tokens WHERE token = $1",
    )
    .bind(&token)
    .fetch_optional(&db.pool)
    .await
    .expect("Failed to create email verification token");

    assert!(result.is_some());
    let (stored_token, stored_account_id, stored_email) = result.unwrap();
    assert_eq!(stored_token, token);
    assert_eq!(stored_account_id, account.id);
    assert_eq!(stored_email, "test@example.com");
}

#[tokio::test]
async fn test_create_email_verification_token_expires() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Create verification token
    let token = db
        .create_email_verification_token(&account.id, "test@example.com")
        .await
        .expect("Failed to create account");

    // Verify expiry is set (24 hours from now)
    let result: Option<(i64, i64)> = sqlx::query_as(
        "SELECT created_at, expires_at FROM email_verification_tokens WHERE token = $1",
    )
    .bind(&token)
    .fetch_optional(&db.pool)
    .await
    .expect("Failed to create email verification token");

    assert!(result.is_some());
    let (created_at, expires_at) = result.unwrap();
    let expected_expiry = created_at + (24 * 3600);
    assert_eq!(expires_at, expected_expiry);
}

#[tokio::test]
async fn test_verify_email_token_success() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Verify email is not verified initially
    let account_check = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(account_check.email.as_deref(), Some("test@example.com"));

    // Create verification token
    let token = db
        .create_email_verification_token(&account.id, "test@example.com")
        .await
        .expect("Failed to create account");

    // Verify token
    db.verify_email_token(&token).await.unwrap();

    // Check that email_verified is now true
    let result: Option<(bool,)> =
        sqlx::query_as("SELECT email_verified FROM accounts WHERE id = $1")
            .bind(&account.id)
            .fetch_optional(&db.pool)
            .await
            .expect("Failed to create email verification token");

    assert!(result.is_some());
    assert_eq!(result.unwrap().0, true);

    // Check that token is marked as used
    let token_result: Option<(Option<i64>,)> =
        sqlx::query_as("SELECT used_at FROM email_verification_tokens WHERE token = $1")
            .bind(&token)
            .fetch_optional(&db.pool)
            .await
            .expect("Failed to fetch from database");

    assert!(token_result.is_some());
    assert!(token_result.unwrap().0.is_some());
}

#[tokio::test]
async fn test_verify_email_token_invalid() {
    let db = setup_test_db().await;

    // Try to verify with invalid token
    let invalid_token = vec![0u8; 16];
    let result = db.verify_email_token(&invalid_token).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email verification token"));
}

#[tokio::test]
async fn test_verify_email_token_already_used() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Create verification token
    let token = db
        .create_email_verification_token(&account.id, "test@example.com")
        .await
        .expect("Failed to create account");

    // Verify token once
    db.verify_email_token(&token).await.unwrap();

    // Try to verify again
    let result = db.verify_email_token(&token).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already been used"));
}

#[tokio::test]
async fn test_verify_email_token_expired() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Create verification token
    let token = db
        .create_email_verification_token(&account.id, "test@example.com")
        .await
        .expect("Failed to create account");

    // Manually expire the token by updating expires_at to past
    let past = chrono::Utc::now().timestamp() - 3600;
    sqlx::query!(
        "UPDATE email_verification_tokens SET expires_at = $1 WHERE token = $2",
        past,
        token
    )
    .execute(&db.pool)
    .await
    .expect("Failed to create email verification token");

    // Try to verify expired token
    let result = db.verify_email_token(&token).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("expired"));
}

#[tokio::test]
async fn test_is_admin_migration() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Verify is_admin is false by default
    assert_eq!(account.is_admin, false);

    // Verify is_admin is included in get_account query
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.is_admin, false);

    // Verify is_admin is included in get_account_by_username query
    let fetched = db
        .get_account_by_username("testuser")
        .await
        .expect("Failed to create account")
        .unwrap();
    assert_eq!(fetched.is_admin, false);

    // Verify is_admin is included in get_account_by_email query
    let fetched = db
        .get_account_by_email("test@example.com")
        .await
        .expect("Failed to fetch account by username")
        .unwrap();
    assert_eq!(fetched.is_admin, false);

    // Manually set is_admin to TRUE to test non-default value
    sqlx::query!(
        "UPDATE accounts SET is_admin = TRUE WHERE id = $1",
        account.id
    )
    .execute(&db.pool)
    .await
    .expect("Failed to get account by email");

    // Verify is_admin is now true
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.is_admin, true);
}

#[tokio::test]
async fn test_set_admin_status_grant() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("testadmin", &[1u8; 32], "admin@example.com")
        .await
        .expect("Failed to create account");

    // Verify not admin initially
    assert_eq!(account.is_admin, false);

    // Grant admin status
    db.set_admin_status("testadmin", true).await.unwrap();

    // Verify is_admin is now true
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.is_admin, true);
}

#[tokio::test]
async fn test_set_admin_status_revoke() {
    let db = setup_test_db().await;

    // Create account and make them admin
    let account = db
        .create_account("revokeadmin", &[2u8; 32], "revoke@example.com")
        .await
        .expect("Failed to create account");

    sqlx::query!(
        "UPDATE accounts SET is_admin = TRUE WHERE id = $1",
        account.id
    )
    .execute(&db.pool)
    .await
    .expect("Failed to create account");

    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.is_admin, true);

    // Revoke admin status
    db.set_admin_status("revokeadmin", false).await.unwrap();

    // Verify is_admin is now false
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.is_admin, false);
}

#[tokio::test]
async fn test_set_admin_status_case_insensitive() {
    let db = setup_test_db().await;

    // Create account with mixed case
    let account = db
        .create_account("MixedCase", &[3u8; 32], "mixed@example.com")
        .await
        .expect("Failed to create account");

    // Grant admin using different case
    db.set_admin_status("mixedcase", true).await.unwrap();

    // Verify is_admin is true
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.is_admin, true);
}

#[tokio::test]
async fn test_set_admin_status_nonexistent_account() {
    let db = setup_test_db().await;

    // Try to grant admin to nonexistent account
    let result = db.set_admin_status("nonexistent", true).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Account not found"));
}

#[tokio::test]
async fn test_list_admins_empty() {
    let db = setup_test_db().await;

    // Create some non-admin accounts
    db.create_account("user1", &[1u8; 32], "user1@example.com")
        .await
        .expect("Failed to create account");
    db.create_account("user2", &[2u8; 32], "user2@example.com")
        .await
        .expect("Failed to create account");

    // List admins should be empty
    let admins = db.list_admins().await.unwrap();
    assert_eq!(admins.len(), 0);
}

#[tokio::test]
async fn test_list_admins() {
    let db = setup_test_db().await;

    // Create accounts
    db.create_account("admin1", &[1u8; 32], "admin1@example.com")
        .await
        .expect("Failed to create account");
    db.create_account("admin2", &[2u8; 32], "admin2@example.com")
        .await
        .expect("Failed to create account");
    db.create_account("user3", &[3u8; 32], "user3@example.com")
        .await
        .expect("Failed to create account");

    // Grant admin to two accounts
    db.set_admin_status("admin1", true).await.unwrap();
    db.set_admin_status("admin2", true).await.unwrap();

    // List admins
    let admins = db.list_admins().await.unwrap();
    assert_eq!(admins.len(), 2);

    // Verify results are sorted by username
    let usernames: Vec<String> = admins.iter().map(|a| a.username.clone()).collect();
    assert_eq!(usernames, vec!["admin1", "admin2"]);

    // Verify all returned accounts have is_admin = true
    for admin in admins {
        assert_eq!(admin.is_admin, true);
    }
}

#[tokio::test]
async fn test_get_account_with_keys_includes_is_admin() {
    let db = setup_test_db().await;

    // Create account (not admin by default)
    let _account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .expect("Failed to create account");

    // Get account with keys and verify is_admin is false
    let account_with_keys = db.get_account_with_keys("testuser").await.unwrap().unwrap();
    assert!(
        !account_with_keys.is_admin,
        "Non-admin should have is_admin=false"
    );

    // Grant admin status
    db.set_admin_status("testuser", true).await.unwrap();

    // Get account with keys again and verify is_admin is now true
    let account_with_keys = db.get_account_with_keys("testuser").await.unwrap().unwrap();
    assert!(
        account_with_keys.is_admin,
        "Admin should have is_admin=true"
    );
}

#[tokio::test]
async fn test_get_account_with_keys_by_public_key_includes_is_admin() {
    let db = setup_test_db().await;
    let pubkey = [2u8; 32];

    // Create account (not admin by default)
    let _account = db
        .create_account("pkuser", &pubkey, "pkuser@example.com")
        .await
        .expect("Failed to create account");

    // Get account with keys by public key and verify is_admin is false
    let account_with_keys = db
        .get_account_with_keys_by_public_key(&pubkey)
        .await
        .expect("Failed to create account")
        .unwrap();
    assert!(
        !account_with_keys.is_admin,
        "Non-admin should have is_admin=false"
    );

    // Grant admin status
    db.set_admin_status("pkuser", true).await.unwrap();

    // Get account with keys again and verify is_admin is now true
    let account_with_keys = db
        .get_account_with_keys_by_public_key(&pubkey)
        .await
        .expect("Failed to get account with keys by public key")
        .unwrap();
    assert!(
        account_with_keys.is_admin,
        "Admin should have is_admin=true"
    );
}

/// Test that new session keys (added on OAuth re-login) can be used for account lookup.
/// This verifies the fix for admin auth failing when OAuth users log in again.
#[tokio::test]
async fn test_oauth_session_key_can_lookup_account() {
    let db = setup_test_db().await;

    // Create OAuth account with initial key (simulates first OAuth registration)
    let initial_key = [10u8; 32];
    let (account, _oauth) = db
        .create_oauth_linked_account(
            "oauthuser",
            &initial_key,
            "oauth@example.com",
            "google_oauth",
            "google_123",
        )
        .await
        .expect("Failed to create OAuth linked account");

    // Verify initial key can lookup account
    let account_id = db.get_account_id_by_public_key(&initial_key).await.unwrap();
    assert!(account_id.is_some(), "Initial key should find account");
    assert_eq!(account_id.unwrap(), account.id);

    // Add a new session key (simulates what happens on OAuth re-login)
    let session_key = [11u8; 32];
    db.add_account_key(&account.id, &session_key).await.unwrap();

    // Verify new session key can also lookup account (this is what admin auth does)
    let account_id = db.get_account_id_by_public_key(&session_key).await.unwrap();
    assert!(
        account_id.is_some(),
        "New session key should find account for admin auth"
    );
    assert_eq!(account_id.unwrap(), account.id);
}

/// Test that disabled keys cannot be used for account lookup
#[tokio::test]
async fn test_disabled_key_cannot_lookup_account() {
    let db = setup_test_db().await;

    // Create account with two keys
    let key1 = [20u8; 32];
    let key2 = [21u8; 32];

    let account = db
        .create_account("twokeys", &key1, "twokeys@example.com")
        .await
        .expect("Failed to create account");
    let key2_record = db.add_account_key(&account.id, &key2).await.unwrap();

    // Both keys should work initially
    assert!(db
        .get_account_id_by_public_key(&key1)
        .await
        .expect("Failed to create account")
        .is_some());
    assert!(db
        .get_account_id_by_public_key(&key2)
        .await
        .expect("Failed to create account")
        .is_some());

    // Disable key2
    let keys = db.get_account_keys(&account.id).await.unwrap();
    let key1_id = keys
        .iter()
        .find(|k| k.public_key == key1)
        .unwrap()
        .id
        .clone();
    db.disable_account_key(&key2_record.id, &key1_id)
        .await
        .expect("Failed to get account keys");

    // key1 should still work, key2 should not (is_active = FALSE)
    assert!(
        db.get_account_id_by_public_key(&key1)
            .await
            .expect("Failed to get account keys")
            .is_some(),
        "Active key should still find account"
    );
    assert!(
        db.get_account_id_by_public_key(&key2)
            .await
            .expect("Failed to disable account key")
            .is_none(),
        "Disabled key should NOT find account"
    );
}

#[tokio::test]
async fn test_get_account_with_keys_includes_email_and_verification_status() {
    let db = setup_test_db().await;

    // Create account (email_verified=false by default)
    let _account = db
        .create_account("emailuser", &[30u8; 32], "emailuser@example.com")
        .await
        .expect("Failed to create account");

    // Get account with keys and verify email fields
    let account_with_keys = db
        .get_account_with_keys("emailuser")
        .await
        .expect("Failed to create account")
        .unwrap();
    assert_eq!(
        account_with_keys.email,
        Some("emailuser@example.com".to_string())
    );
    assert!(
        !account_with_keys.email_verified,
        "Email should not be verified initially"
    );

    // Verify email
    let token = db
        .create_email_verification_token(
            &hex::decode(&account_with_keys.id).expect("Failed to decode account ID"),
            "emailuser@example.com",
        )
        .await
        .expect("Failed to create email verification token");
    db.verify_email_token(&token).await.unwrap();

    // Get account with keys again and verify email_verified is now true
    let account_with_keys = db
        .get_account_with_keys("emailuser")
        .await
        .expect("Failed to fetch account with keys")
        .unwrap();
    assert_eq!(
        account_with_keys.email,
        Some("emailuser@example.com".to_string())
    );
    assert!(
        account_with_keys.email_verified,
        "Email should be verified after verification"
    );
}

#[tokio::test]
async fn test_get_account_with_keys_by_public_key_includes_email_and_verification_status() {
    let db = setup_test_db().await;
    let pubkey = [31u8; 32];

    // Create account (email_verified=false by default)
    let _account = db
        .create_account("pkemailuser", &pubkey, "pkemailuser@example.com")
        .await
        .expect("Failed to create account");

    // Get account with keys by public key and verify email fields
    let account_with_keys = db
        .get_account_with_keys_by_public_key(&pubkey)
        .await
        .expect("Failed to create account")
        .unwrap();
    assert_eq!(
        account_with_keys.email,
        Some("pkemailuser@example.com".to_string())
    );
    assert!(
        !account_with_keys.email_verified,
        "Email should not be verified initially"
    );

    // Verify email
    let token = db
        .create_email_verification_token(
            &hex::decode(&account_with_keys.id).expect("Failed to decode account ID"),
            "pkemailuser@example.com",
        )
        .await
        .expect("Failed to create email verification token");
    db.verify_email_token(&token).await.unwrap();

    // Get account with keys again and verify email_verified is now true
    let account_with_keys = db
        .get_account_with_keys_by_public_key(&pubkey)
        .await
        .expect("Failed to get account with keys by public key")
        .unwrap();
    assert_eq!(
        account_with_keys.email,
        Some("pkemailuser@example.com".to_string())
    );
    assert!(
        account_with_keys.email_verified,
        "Email should be verified after verification"
    );
}

#[tokio::test]
async fn test_oauth_account_with_keys_has_verified_email() {
    let db = setup_test_db().await;

    let pubkey = [32u8; 32];
    let (_account, _oauth_acc) = db
        .create_oauth_linked_account(
            "oauth_email_user",
            &pubkey,
            "oauth@example.com",
            "google_oauth",
            "google_456",
        )
        .await
        .expect("Failed to create OAuth linked account");

    // Get account with keys and verify email is verified for OAuth accounts
    let account_with_keys = db
        .get_account_with_keys("oauth_email_user")
        .await
        .expect("Failed to create OAuth linked account")
        .unwrap();
    assert_eq!(
        account_with_keys.email,
        Some("oauth@example.com".to_string())
    );
    assert!(
        account_with_keys.email_verified,
        "OAuth accounts should have verified email"
    );
}

#[tokio::test]
async fn test_oauth_account_creation_sets_email_verified() {
    let db = setup_test_db().await;

    let pubkey = [10u8; 32];
    let (account, _oauth_acc) = db
        .create_oauth_linked_account(
            "oauth_user",
            &pubkey,
            "oauth@example.com",
            "google_oauth",
            "google_123",
        )
        .await
        .expect("Failed to create OAuth linked account");

    // Verify email_verified is set to true
    assert_eq!(
        account.email_verified, true,
        "OAuth accounts should have email_verified set to true"
    );
}

#[tokio::test]
async fn test_oauth_linking_to_existing_account_sets_email_verified() {
    let db = setup_test_db().await;

    // Create an account with unverified email
    let account = db
        .create_account("existing_user", &[11u8; 32], "existing@example.com")
        .await
        .unwrap();

    // Verify email is not verified initially
    assert_eq!(
        account.email_verified, false,
        "New accounts should have email_verified=false"
    );

    // Link OAuth account to existing account
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_456",
        Some("existing@example.com"),
    )
    .await
    .expect("Failed to create OAuth linked account");

    // Set email as verified (simulating what oauth_simple.rs does)
    db.set_email_verified(&account.id, true).await.unwrap();

    // Fetch account again and verify email_verified is now set
    let updated_account = db
        .get_account_by_username("existing_user")
        .await
        .expect("Failed to create OAuth linked account")
        .unwrap();

    assert_eq!(
        updated_account.email_verified, true,
        "Linking OAuth should set email_verified to true"
    );
}

#[tokio::test]
async fn test_get_latest_verification_token_time() {
    let db = setup_test_db().await;
    let username = "token_time_user";
    let public_key = [99u8; 32];

    let account = db
        .create_account(username, &public_key, "token@example.com")
        .await
        .expect("Failed to get latest verification token time");

    // No token created yet - should return None
    let time = db
        .get_latest_verification_token_time(&account.id)
        .await
        .expect("Failed to create account");
    assert!(time.is_none());

    // Create first token
    db.create_email_verification_token(&account.id, "token@example.com")
        .await
        .expect("Failed to create account");

    let time1 = db
        .get_latest_verification_token_time(&account.id)
        .await
        .expect("Failed to create email verification token");
    assert!(time1.is_some());

    // Wait enough time to ensure different timestamps (PostgreSQL precision is microseconds)
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    db.create_email_verification_token(&account.id, "token@example.com")
        .await
        .expect("Failed to create email verification token");

    let time2 = db
        .get_latest_verification_token_time(&account.id)
        .await
        .expect("Failed to create email verification token");
    assert!(time2.is_some());

    // Second token time should be greater than or equal to first
    assert!(time2.unwrap() >= time1.unwrap());
}

#[tokio::test]
async fn test_resend_verification_rate_limit() {
    let db = setup_test_db().await;
    let username = "rate_limit_user";
    let public_key = [98u8; 32];

    let account = db
        .create_account(username, &public_key, "rate@example.com")
        .await
        .expect("Failed to create account");

    // Create first token
    db.create_email_verification_token(&account.id, "rate@example.com")
        .await
        .expect("Failed to create account");

    let time = db
        .get_latest_verification_token_time(&account.id)
        .await
        .expect("Failed to create account")
        .unwrap();
    let now = chrono::Utc::now().timestamp();

    // Should be within 60 seconds
    let elapsed = now - time;
    assert!(elapsed < 60);
}

#[tokio::test]
async fn test_get_account_by_chatwoot_user_id() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("chatwoot_user", &[50u8; 32], "chatwoot@example.com")
        .await
        .expect("Failed to create account");

    // Set Chatwoot user ID
    let chatwoot_user_id = 12345i64;
    db.set_chatwoot_user_id(&account.id, chatwoot_user_id)
        .await
        .expect("Failed to create account");

    // Fetch by Chatwoot user ID
    let fetched = db
        .get_account_by_chatwoot_user_id(chatwoot_user_id)
        .await
        .expect("Failed to create account");

    assert!(fetched.is_some());
    let fetched = fetched.unwrap();
    assert_eq!(fetched.id, account.id);
    assert_eq!(fetched.username, "chatwoot_user");
    assert_eq!(fetched.chatwoot_user_id, Some(chatwoot_user_id));
}

#[tokio::test]
async fn test_get_account_by_chatwoot_user_id_not_found() {
    let db = setup_test_db().await;

    let result = db.get_account_by_chatwoot_user_id(99999).await.unwrap();

    assert!(
        result.is_none(),
        "Should return None for nonexistent Chatwoot user ID"
    );
}

#[tokio::test]
async fn test_ensure_account_for_pubkey_creates_new_account() {
    let db = setup_test_db().await;
    let pubkey = [
        0xab, 0xcd, 0xef, 0x12, 0x34, 0x56, 0x78, 0x90, // first 8 chars = abcdef12
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
    ];

    // First call should create a new account
    let account_id = db.ensure_account_for_pubkey(&pubkey).await.unwrap();
    assert!(!account_id.is_empty());

    // Verify the account was created with expected username format
    let account = db
        .get_account_with_keys_by_public_key(&pubkey)
        .await
        .expect("Failed to get account with keys by public key");
    assert!(account.is_some());
    let account = account.unwrap();
    assert!(account.username.starts_with("user_abcdef12"));
}

#[tokio::test]
async fn test_ensure_account_for_pubkey_returns_existing() {
    let db = setup_test_db().await;
    let pubkey = [0x99u8; 32];

    // Create account first
    let first_id = db.ensure_account_for_pubkey(&pubkey).await.unwrap();

    // Second call should return the same account (idempotent)
    let second_id = db.ensure_account_for_pubkey(&pubkey).await.unwrap();
    assert_eq!(first_id, second_id, "Should return existing account");
}

#[tokio::test]
async fn test_ensure_account_for_pubkey_handles_username_collision() {
    let db = setup_test_db().await;

    // Two pubkeys with the same first 8 hex chars (different later bytes)
    let pubkey1 = [
        0xde, 0xad, 0xbe, 0xef, // first 8 chars = deadbeef
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    ]; // last byte different
    let pubkey2 = [
        0xde, 0xad, 0xbe, 0xef, // same first 8 chars
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
    ]; // last byte different

    // Create first account
    let id1 = db.ensure_account_for_pubkey(&pubkey1).await.unwrap();

    // Create second account - should get a suffixed username
    let id2 = db.ensure_account_for_pubkey(&pubkey2).await.unwrap();
    assert_ne!(id1, id2, "Should create different accounts");

    // Verify usernames are different
    let acc1 = db
        .get_account_with_keys_by_public_key(&pubkey1)
        .await
        .expect("Failed to get account with keys by public key")
        .unwrap();
    let acc2 = db
        .get_account_with_keys_by_public_key(&pubkey2)
        .await
        .expect("Failed to get account with keys by public key")
        .unwrap();
    assert_eq!(acc1.username, "user_deadbeef");
    assert!(
        acc2.username.starts_with("user_deadbeef_"),
        "Second account should have suffix"
    );
}

#[tokio::test]
async fn test_admin_set_account_email() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("emailtest", &[60u8; 32], "original@example.com")
        .await
        .expect("Failed to create account");

    // Verify original email
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.email, Some("original@example.com".to_string()));

    // Update email
    db.admin_set_account_email(&account.id, Some("new@example.com"))
        .await
        .expect("Failed to create account");

    // Verify new email and that email_verified was reset
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.email, Some("new@example.com".to_string()));
    assert_eq!(fetched.email_verified, false);
}

#[tokio::test]
async fn test_admin_set_account_email_clear() {
    let db = setup_test_db().await;

    // Create account
    let account = db
        .create_account("clearemail", &[61u8; 32], "clear@example.com")
        .await
        .expect("Failed to create account");

    // Clear email
    db.admin_set_account_email(&account.id, None).await.unwrap();

    // Verify email is cleared
    let fetched = db.get_account(&account.id).await.unwrap().unwrap();
    assert_eq!(fetched.email, None);
}

#[tokio::test]
async fn test_admin_set_account_email_nonexistent() {
    let db = setup_test_db().await;

    // Try to update nonexistent account
    let result = db
        .admin_set_account_email(&[0u8; 16], Some("test@example.com"))
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Account not found"));
}

#[tokio::test]
async fn test_admin_delete_account() {
    let db = setup_test_db().await;

    // Create account with keys
    let account = db
        .create_account("todelete", &[70u8; 32], "delete@example.com")
        .await
        .expect("Failed to create account");

    // Add another key
    db.add_account_key(&account.id, &[71u8; 32]).await.unwrap();

    // Delete account
    let summary = db.admin_delete_account(&account.id).await.unwrap();

    // Verify summary
    assert_eq!(summary.public_keys_deleted, 2);
    assert_eq!(summary.offerings_deleted, 0);
    assert!(!summary.provider_profile_deleted);

    // Verify account is gone
    let fetched = db.get_account(&account.id).await.unwrap();
    assert!(fetched.is_none());
}

#[tokio::test]
async fn test_admin_delete_account_nonexistent() {
    let db = setup_test_db().await;

    // Try to delete nonexistent account
    let result = db.admin_delete_account(&[0u8; 16]).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Account not found"));
}

#[tokio::test]
async fn test_admin_delete_account_with_oauth() {
    let db = setup_test_db().await;

    // Create OAuth account
    let (account, _oauth) = db
        .create_oauth_linked_account(
            "oauthdel",
            &[72u8; 32],
            "oauthdel@example.com",
            "google_oauth",
            "google_del_123",
        )
        .await
        .expect("Failed to create OAuth linked account");

    // Delete account
    let summary = db.admin_delete_account(&account.id).await.unwrap();

    // Verify summary
    assert_eq!(summary.public_keys_deleted, 1);

    // Verify account is gone
    let fetched = db.get_account(&account.id).await.unwrap();
    assert!(fetched.is_none());

    // Verify OAuth account is also gone
    let oauth_fetched = db
        .get_oauth_account_by_provider_and_external_id("google_oauth", "google_del_123")
        .await
        .expect("Failed to get OAuth account by provider and external ID");
    assert!(oauth_fetched.is_none());
}

#[tokio::test]
async fn test_count_accounts() {
    let db = setup_test_db().await;

    // No accounts initially
    let count = db.count_accounts().await.unwrap();
    assert_eq!(count, 0);

    // Create some accounts
    db.create_account("user1", &[1u8; 32], "user1@example.com")
        .await
        .expect("Failed to count accounts");
    db.create_account("user2", &[2u8; 32], "user2@example.com")
        .await
        .expect("Failed to create account");
    db.create_account("user3", &[3u8; 32], "user3@example.com")
        .await
        .expect("Failed to create account");

    let count = db.count_accounts().await.unwrap();
    assert_eq!(count, 3);
}

#[tokio::test]
async fn test_list_all_accounts() {
    let db = setup_test_db().await;

    // Create accounts
    db.create_account("alice", &[1u8; 32], "alice@example.com")
        .await
        .expect("Failed to create account");
    db.create_account("bob", &[2u8; 32], "bob@example.com")
        .await
        .expect("Failed to create account");
    db.create_account("charlie", &[3u8; 32], "charlie@example.com")
        .await
        .expect("Failed to create account");

    // List all accounts
    let accounts = db.list_all_accounts(10, 0).await.unwrap();
    assert_eq!(accounts.len(), 3);

    // Verify sorted by username
    assert_eq!(accounts[0].username, "alice");
    assert_eq!(accounts[1].username, "bob");
    assert_eq!(accounts[2].username, "charlie");
}

#[tokio::test]
async fn test_list_all_accounts_pagination() {
    let db = setup_test_db().await;

    // Create 5 accounts
    for i in 0..5 {
        let mut pk = [0u8; 32];
        pk[0] = i;
        db.create_account(
            &format!("user{}", i),
            &pk,
            &format!("user{}@example.com", i),
        )
        .await
        .expect("Failed to create account");
    }

    // First page (limit 2, offset 0)
    let page1 = db.list_all_accounts(2, 0).await.unwrap();
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].username, "user0");
    assert_eq!(page1[1].username, "user1");

    // Second page (limit 2, offset 2)
    let page2 = db.list_all_accounts(2, 2).await.unwrap();
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].username, "user2");
    assert_eq!(page2[1].username, "user3");

    // Third page (limit 2, offset 4)
    let page3 = db.list_all_accounts(2, 4).await.unwrap();
    assert_eq!(page3.len(), 1);
    assert_eq!(page3[0].username, "user4");

    // Empty page (offset beyond total)
    let empty = db.list_all_accounts(2, 10).await.unwrap();
    assert_eq!(empty.len(), 0);
}

#[tokio::test]
async fn test_list_all_accounts_includes_admin_status() {
    let db = setup_test_db().await;

    // Create accounts
    db.create_account("admin_user", &[1u8; 32], "admin@example.com")
        .await
        .expect("Failed to create account");
    db.create_account("regular_user", &[2u8; 32], "regular@example.com")
        .await
        .expect("Failed to create account");

    // Make one an admin
    db.set_admin_status("admin_user", true).await.unwrap();

    // List accounts
    let accounts = db.list_all_accounts(10, 0).await.unwrap();
    assert_eq!(accounts.len(), 2);

    let admin = accounts
        .iter()
        .find(|a| a.username == "admin_user")
        .unwrap();
    let regular = accounts
        .iter()
        .find(|a| a.username == "regular_user")
        .unwrap();

    assert_eq!(admin.is_admin, true);
    assert_eq!(regular.is_admin, false);
}

/// Test that admin_delete_account properly handles accounts with offerings and profiles
/// that have account_id set (the new foreign key from migration 050).
#[tokio::test]
async fn test_admin_delete_account_with_offerings_and_profile() {
    let db = setup_test_db().await;
    let pubkey = [80u8; 32];

    // Create account
    let account = db
        .create_account("provider_user", &pubkey, "provider@example.com")
        .await
        .expect("Failed to create account");

    // Create provider profile with account_id set
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, account_id, name, description, api_version, profile_version, updated_at_ns)
         VALUES ($1, $2, 'Test Provider', 'Test description', '1.0', '1.0', 0)",
    )
    .bind(&pubkey[..])
    .bind(&account.id)
    .execute(&db.pool)
    .await
    .expect("Failed to create account");

    // Create provider offering with account_id set
    sqlx::query(
        "INSERT INTO provider_offerings (pubkey, account_id, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns)
         VALUES ($1, $2, 'test-offer-1', 'Test Offer', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', false, 0)",
    )
    .bind(&pubkey[..])
    .bind(&account.id)
    .execute(&db.pool)
    .await
    .expect("Failed to execute SQL query");

    // Create signature audit record with account_id set
    let signature = [0u8; 64];
    let nonce = uuid::Uuid::new_v4();
    db.insert_signature_audit(
        Some(&account.id),
        "test_action",
        "{}",
        &signature,
        &pubkey,
        chrono::Utc::now()
            .timestamp_nanos_opt()
            .expect("Failed to get timestamp"),
        &nonce,
        false,
    )
    .await
    .expect("Failed to insert signature audit");

    // Verify data was created
    let profile_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM provider_profiles WHERE account_id = $1")
            .bind(&account.id)
            .fetch_one(&db.pool)
            .await
            .expect("Failed to fetch from database");
    assert_eq!(profile_count.0, 1, "Profile should exist before deletion");

    let offering_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM provider_offerings WHERE account_id = $1")
            .bind(&account.id)
            .fetch_one(&db.pool)
            .await
            .expect("Failed to fetch from database");
    assert_eq!(offering_count.0, 1, "Offering should exist before deletion");

    // Delete account - this should NOT fail with FK constraint error
    let summary = db.admin_delete_account(&account.id).await.unwrap();

    // Verify summary
    assert_eq!(summary.offerings_deleted, 1);
    assert!(summary.provider_profile_deleted);
    assert_eq!(summary.public_keys_deleted, 1);

    // Verify account is gone
    let fetched = db.get_account(&account.id).await.unwrap();
    assert!(fetched.is_none());

    // Verify all related data is deleted
    let profile_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM provider_profiles WHERE pubkey = $1")
            .bind(&pubkey[..])
            .fetch_one(&db.pool)
            .await
            .expect("Failed to get account");
    assert_eq!(profile_count.0, 0, "Profile should be deleted");

    let offering_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM provider_offerings WHERE pubkey = $1")
            .bind(&pubkey[..])
            .fetch_one(&db.pool)
            .await
            .expect("Failed to fetch from database");
    assert_eq!(offering_count.0, 0, "Offering should be deleted");

    let audit_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM signature_audit WHERE account_id = $1")
            .bind(&account.id)
            .fetch_one(&db.pool)
            .await
            .expect("Failed to fetch from database");
    assert_eq!(audit_count.0, 0, "Signature audit should be deleted");
}
