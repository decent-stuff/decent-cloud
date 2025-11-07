use crate::database::{Database, LedgerEntryData};
use sqlx::SqlitePool;

/// Test setup helper for creating an in-memory database with required tables
async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS ledger_entries (label TEXT, key BLOB, value BLOB)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("CREATE TABLE IF NOT EXISTS sync_state (id INTEGER PRIMARY KEY, last_position INTEGER, last_sync_at TIMESTAMP)")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT OR IGNORE INTO sync_state (id, last_position, last_sync_at) VALUES (1, 0, CURRENT_TIMESTAMP)")
        .execute(&pool)
        .await
        .unwrap();
    Database { pool }
}

#[tokio::test]
async fn test_insert_entries_empty() {
    let db = setup_test_db().await;

    let entries = vec![];
    let result = db.insert_entries(entries).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_insert_entries_single() {
    let db = setup_test_db().await;

    let entries = vec![LedgerEntryData {
        label: "test".to_string(),
        key: b"test_key".to_vec(),
        value: b"test_value".to_vec(),
    }];

    let result = db.insert_entries(entries).await;
    assert!(result.is_ok());

    let count = sqlx::query("SELECT COUNT(*) as count FROM ledger_entries")
        .fetch_one(&db.pool)
        .await
        .unwrap()
        .get::<i64, _>("count");
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_insert_entries_multiple() {
    let db = setup_test_db().await;

    let entries = vec![
        LedgerEntryData {
            label: "test1".to_string(),
            key: b"key1".to_vec(),
            value: b"value1".to_vec(),
        },
        LedgerEntryData {
            label: "test2".to_string(),
            key: b"key2".to_vec(),
            value: b"value2".to_vec(),
        },
    ];

    let result = db.insert_entries(entries).await;
    assert!(result.is_ok());

    let count = sqlx::query("SELECT COUNT(*) as count FROM ledger_entries")
        .fetch_one(&db.pool)
        .await
        .unwrap()
        .get::<i64, _>("count");
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_get_last_sync_position_initial() {
    let db = setup_test_db().await;
    
    let position = db.get_last_sync_position().await.unwrap();
    assert_eq!(position, 0);
}

#[tokio::test]
async fn test_update_sync_position() {
    let db = setup_test_db().await;
    
    let new_position = 42;
    db.update_sync_position(new_position).await.unwrap();
    
    let retrieved_position = db.get_last_sync_position().await.unwrap();
    assert_eq!(retrieved_position, new_position);
}

#[tokio::test]
async fn test_update_sync_position_multiple_updates() {
    let db = setup_test_db().await;
    
    db.update_sync_position(100).await.unwrap();
    assert_eq!(db.get_last_sync_position().await.unwrap(), 100);
    
    db.update_sync_position(250).await.unwrap();
    assert_eq!(db.get_last_sync_position().await.unwrap(), 250);
    
    db.update_sync_position(999).await.unwrap();
    assert_eq!(db.get_last_sync_position().await.unwrap(), 999);
}
