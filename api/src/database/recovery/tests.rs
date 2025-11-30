use crate::database::test_helpers::setup_test_db;

#[tokio::test]
async fn test_create_recovery_token() {
    let db = setup_test_db().await;

    // Create account first
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .unwrap();
    // Add email via OAuth link
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_123",
        Some("test@example.com"),
    )
    .await
    .unwrap();

    // Create recovery token
    let token = db.create_recovery_token("test@example.com").await.unwrap();
    assert_eq!(token.len(), 16);
}

#[tokio::test]
async fn test_create_recovery_token_no_email() {
    let db = setup_test_db().await;

    // Try to create token for non-existent email
    let result = db.create_recovery_token("nonexistent@example.com").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No account found"));
}

#[tokio::test]
async fn test_verify_recovery_token() {
    let db = setup_test_db().await;

    // Create account with email
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .unwrap();
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_123",
        Some("test@example.com"),
    )
    .await
    .unwrap();

    // Create token
    let token = db.create_recovery_token("test@example.com").await.unwrap();

    // Verify token
    let account_id = db.verify_recovery_token(&token).await.unwrap();
    assert_eq!(account_id.len(), 16);
}

#[tokio::test]
async fn test_verify_invalid_token() {
    let db = setup_test_db().await;

    let invalid_token = vec![0u8; 16];
    let result = db.verify_recovery_token(&invalid_token).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid"));
}

#[tokio::test]
async fn test_complete_recovery() {
    let db = setup_test_db().await;

    // Create account with one key
    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .unwrap();
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_123",
        Some("test@example.com"),
    )
    .await
    .unwrap();

    // Create recovery token
    let token = db.create_recovery_token("test@example.com").await.unwrap();

    // Complete recovery with new key
    let new_key = [2u8; 32];
    db.complete_recovery(&token, &new_key).await.unwrap();

    // Verify account now has 2 keys
    let account = db
        .get_account_by_username("testuser")
        .await
        .unwrap()
        .unwrap();
    let keys = db.get_account_keys(&account.id).await.unwrap();
    assert_eq!(keys.len(), 2);
}

#[tokio::test]
async fn test_complete_recovery_token_used_twice() {
    let db = setup_test_db().await;

    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .unwrap();
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_123",
        Some("test@example.com"),
    )
    .await
    .unwrap();

    let token = db.create_recovery_token("test@example.com").await.unwrap();

    // Use token once
    db.complete_recovery(&token, &[2u8; 32]).await.unwrap();

    // Try to use again
    let result = db.complete_recovery(&token, &[3u8; 32]).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("already been used"));
}

#[tokio::test]
async fn test_cleanup_expired_recovery_tokens() {
    let db = setup_test_db().await;

    let account = db
        .create_account("testuser", &[1u8; 32], "test@example.com")
        .await
        .unwrap();
    db.create_oauth_account(
        &account.id,
        "google_oauth",
        "google_123",
        Some("test@example.com"),
    )
    .await
    .unwrap();

    // Create token
    db.create_recovery_token("test@example.com").await.unwrap();

    // Manually expire the token by updating expires_at to past
    let past = chrono::Utc::now().timestamp() - 3600;
    sqlx::query!("UPDATE recovery_tokens SET expires_at = ?", past)
        .execute(&db.pool)
        .await
        .unwrap();

    // Cleanup
    let deleted = db.cleanup_expired_recovery_tokens().await.unwrap();
    assert_eq!(deleted, 1);
}
