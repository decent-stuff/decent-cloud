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
    )
    .await
    .unwrap();

    // Now nonce should exist
    let exists = db.check_nonce_exists(&nonce, 10).await.unwrap();
    assert!(exists);
}
