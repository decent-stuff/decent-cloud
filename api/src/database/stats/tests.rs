use super::*;
use crate::database::test_helpers::setup_test_db;

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

    {
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    {
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-1', 'Test', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 0)",
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    let contract_id = vec![1u8; 32];
    let requester_pubkey = vec![2u8; 32];
    {
        let contract_id_ref = &contract_id;
        let requester_pubkey_ref = &requester_pubkey;
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')",
            contract_id_ref,
            requester_pubkey_ref,
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    sqlx::query!(
        "INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns) VALUES ('alice', 'bob', 500, 10, '', 0)"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let nonce_signature = vec![0u8; 64];
    {
        let pubkey_ref = &pubkey;
        let nonce_signature_ref = &nonce_signature;
        sqlx::query!(
            "INSERT INTO provider_check_ins (pubkey, memo, nonce_signature, block_timestamp_ns) VALUES (?, 'test', ?, ?)",
            pubkey_ref,
            nonce_signature_ref,
            now_ns
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

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

    {
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO reputation_changes (pubkey, change_amount, reason, block_timestamp_ns) VALUES (?, 10, 'test', 0)",
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    {
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO reputation_changes (pubkey, change_amount, reason, block_timestamp_ns) VALUES (?, -5, 'penalty', 1000)",
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let info = db.get_reputation(&pubkey).await.unwrap().unwrap();
    assert_eq!(info.total_reputation, 5);
    assert_eq!(info.change_count, 2);
}

#[tokio::test]
async fn test_get_top_providers_by_reputation() {
    let db = setup_test_db().await;
    let pk1 = vec![1u8; 32];
    let pk2 = vec![2u8; 32];

    {
        let pk1_ref = &pk1;
        sqlx::query!(
            "INSERT INTO reputation_changes (pubkey, change_amount, reason, block_timestamp_ns) VALUES (?, 100, 'good', 0)",
            pk1_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    {
        let pk2_ref = &pk2;
        sqlx::query!(
            "INSERT INTO reputation_changes (pubkey, change_amount, reason, block_timestamp_ns) VALUES (?, 50, 'ok', 0)",
            pk2_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let top = db.get_top_providers_by_reputation(1).await.unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].pubkey, pk1);
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

    {
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-1', 'Test', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 0)",
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    let pending_contract_id = vec![1u8; 32];
    let pending_requester = vec![2u8; 32];
    {
        let contract_id_ref = &pending_contract_id;
        let requester_ref = &pending_requester;
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')",
            contract_id_ref,
            requester_ref,
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    let active_contract_id = vec![2u8; 32];
    let active_requester = vec![2u8; 32];
    {
        let contract_id_ref = &active_contract_id;
        let requester_ref = &active_requester;
        let pubkey_ref = &pubkey;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 2000, 'memo', 1000, 'active')",
            contract_id_ref,
            requester_ref,
            pubkey_ref
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

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

    let nonce_signature1 = vec![0u8; 64];
    {
        let pubkey_ref = &pubkey;
        let nonce_ref = &nonce_signature1;
        sqlx::query!(
            "INSERT INTO provider_check_ins (pubkey, memo, nonce_signature, block_timestamp_ns) VALUES (?, 'test1', ?, ?)",
            pubkey_ref,
            nonce_ref,
            timestamp1
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    let nonce_signature2 = vec![1u8; 64];
    {
        let pubkey_ref = &pubkey;
        let nonce_ref = &nonce_signature2;
        sqlx::query!(
            "INSERT INTO provider_check_ins (pubkey, memo, nonce_signature, block_timestamp_ns) VALUES (?, 'test2', ?, ?)",
            pubkey_ref,
            nonce_ref,
            timestamp2
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let result = db.get_latest_block_timestamp_ns().await.unwrap();
    assert_eq!(result, Some(timestamp2));
}

#[test]
fn export_typescript_types() {
    PlatformStats::export().expect("Failed to export PlatformStats type");
}
