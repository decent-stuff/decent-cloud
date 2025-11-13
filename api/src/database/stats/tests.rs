use super::*;
use sqlx::SqlitePool;

async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let migration_sql = include_str!("../../../migrations/001_original_schema.sql");
    sqlx::query(migration_sql).execute(&pool).await.unwrap();
    Database { pool }
}

#[tokio::test]
async fn test_get_platform_stats_empty() {
    let db = setup_test_db().await;
    let stats = db.get_platform_stats().await.unwrap();
    assert_eq!(stats.total_providers, 0);
    assert_eq!(stats.active_providers, 0);
    assert_eq!(stats.total_offerings, 0);
    assert_eq!(stats.total_contracts, 0);
    assert_eq!(stats.total_transfers, 0);
    assert_eq!(stats.total_volume_e9s, 0);
}

#[tokio::test]
async fn test_get_platform_stats_with_data() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    sqlx::query("INSERT INTO provider_profiles (pubkey_hash, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)")
            .bind(&pubkey).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO provider_offerings (pubkey_hash, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-1', 'Test', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 0)")
            .bind(&pubkey).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey_hash, requester_ssh_pubkey, requester_contact, provider_pubkey_hash, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
            .bind(vec![1u8; 32]).bind(vec![2u8; 32]).bind(&pubkey).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns) VALUES ('alice', 'bob', 500, 10, '', 0)")
            .execute(&db.pool).await.unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    sqlx::query("INSERT INTO provider_check_ins (pubkey_hash, memo, nonce_signature, block_timestamp_ns) VALUES (?, 'test', ?, ?)")
            .bind(&pubkey).bind(vec![0u8; 64]).bind(now_ns).execute(&db.pool).await.unwrap();

    let stats = db.get_platform_stats().await.unwrap();
    assert_eq!(stats.total_providers, 1);
    assert_eq!(stats.active_providers, 1);
    assert_eq!(stats.total_offerings, 1);
    assert_eq!(stats.total_contracts, 1);
    assert_eq!(stats.total_transfers, 1);
    assert_eq!(stats.total_volume_e9s, 500);
}

#[tokio::test]
async fn test_get_reputation_none() {
    let db = setup_test_db().await;
    let result = db.get_reputation(&[1u8; 32]).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_reputation_with_changes() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    sqlx::query("INSERT INTO reputation_changes (pubkey_hash, change_amount, reason, block_timestamp_ns) VALUES (?, 10, 'test', 0)")
            .bind(&pubkey).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO reputation_changes (pubkey_hash, change_amount, reason, block_timestamp_ns) VALUES (?, -5, 'penalty', 1000)")
            .bind(&pubkey).execute(&db.pool).await.unwrap();

    let info = db.get_reputation(&pubkey).await.unwrap().unwrap();
    assert_eq!(info.total_reputation, 5);
    assert_eq!(info.change_count, 2);
}

#[tokio::test]
async fn test_get_top_providers_by_reputation() {
    let db = setup_test_db().await;
    let pk1 = vec![1u8; 32];
    let pk2 = vec![2u8; 32];

    sqlx::query("INSERT INTO reputation_changes (pubkey_hash, change_amount, reason, block_timestamp_ns) VALUES (?, 100, 'good', 0)")
            .bind(&pk1).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO reputation_changes (pubkey_hash, change_amount, reason, block_timestamp_ns) VALUES (?, 50, 'ok', 0)")
            .bind(&pk2).execute(&db.pool).await.unwrap();

    let top = db.get_top_providers_by_reputation(1).await.unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].pubkey_hash, pk1);
    assert_eq!(top[0].total_reputation, 100);
}

#[tokio::test]
async fn test_get_provider_stats_empty() {
    let db = setup_test_db().await;
    let stats = db.get_provider_stats(&[1u8; 32]).await.unwrap();
    assert_eq!(stats.total_contracts, 0);
    assert_eq!(stats.pending_contracts, 0);
    assert_eq!(stats.total_revenue_e9s, 0);
    assert_eq!(stats.offerings_count, 0);
}

#[tokio::test]
async fn test_get_provider_stats_with_data() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    sqlx::query("INSERT INTO provider_offerings (pubkey_hash, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-1', 'Test', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 0)")
            .bind(&pubkey).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey_hash, requester_ssh_pubkey, requester_contact, provider_pubkey_hash, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
            .bind(vec![1u8; 32]).bind(vec![2u8; 32]).bind(&pubkey).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey_hash, requester_ssh_pubkey, requester_contact, provider_pubkey_hash, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 2000, 'memo', 1000, 'active')")
            .bind(vec![2u8; 32]).bind(vec![2u8; 32]).bind(&pubkey).execute(&db.pool).await.unwrap();

    let stats = db.get_provider_stats(&pubkey).await.unwrap();
    assert_eq!(stats.total_contracts, 2);
    assert_eq!(stats.pending_contracts, 1);
    assert_eq!(stats.total_revenue_e9s, 3000);
    assert_eq!(stats.offerings_count, 1);
}

#[tokio::test]
async fn test_get_latest_block_timestamp_ns_empty() {
    let db = setup_test_db().await;
    let result = db.get_latest_block_timestamp_ns().await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_latest_block_timestamp_ns_with_data() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];
    let timestamp1 = 1000i64;
    let timestamp2 = 2000i64;

    sqlx::query("INSERT INTO provider_check_ins (pubkey_hash, memo, nonce_signature, block_timestamp_ns) VALUES (?, 'test1', ?, ?)")
            .bind(&pubkey).bind(vec![0u8; 64]).bind(timestamp1).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO provider_check_ins (pubkey_hash, memo, nonce_signature, block_timestamp_ns) VALUES (?, 'test2', ?, ?)")
            .bind(&pubkey).bind(vec![1u8; 64]).bind(timestamp2).execute(&db.pool).await.unwrap();

    let result = db.get_latest_block_timestamp_ns().await.unwrap();
    assert_eq!(result, Some(timestamp2));
}

#[test]
fn export_typescript_types() {
    PlatformStats::export().expect("Failed to export PlatformStats type");
}
