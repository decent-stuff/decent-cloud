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
        let payment_method = "dct";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending', ?, ?, ?, 'usd')",
            contract_id_ref,
            requester_pubkey_ref,
            pubkey_ref,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
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
        let payment_method = "dct";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending', ?, ?, ?, 'usd')",
            contract_id_ref,
            requester_ref,
            pubkey_ref,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
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
        let payment_method = "dct";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 2000, 'memo', 1000, 'active', ?, ?, ?, 'usd')",
            contract_id_ref,
            requester_ref,
            pubkey_ref,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
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

#[tokio::test]
async fn test_search_accounts_empty() {
    let db = setup_test_db().await;
    let results = db.search_accounts("test", 50).await.unwrap();
    assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_search_accounts_by_username() {
    let db = setup_test_db().await;

    // Create account
    db.create_account("alice", &[1u8; 32]).await.unwrap();

    // Search for username
    let results = db.search_accounts("alice", 50).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].username, "alice");
    assert_eq!(results[0].display_name, None);
    assert_eq!(results[0].reputation_score, 0);
    assert_eq!(results[0].contract_count, 0);
    assert_eq!(results[0].offering_count, 0);
}

#[tokio::test]
async fn test_search_accounts_by_display_name() {
    let db = setup_test_db().await;

    // Create account
    let account = db.create_account("alice", &[1u8; 32]).await.unwrap();

    // Update display name
    sqlx::query!(
        "UPDATE accounts SET display_name = ? WHERE id = ?",
        "Alice Wonderland",
        account.id
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Search by display name
    let results = db.search_accounts("wonderland", 50).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].username, "alice");
    assert_eq!(
        results[0].display_name,
        Some("Alice Wonderland".to_string())
    );
}

#[tokio::test]
async fn test_search_accounts_by_pubkey() {
    let db = setup_test_db().await;

    // Create account with specific pubkey
    let pubkey = vec![0xAB, 0xCD, 0xEF, 0x01, 0x23, 0x45, 0x67, 0x89];
    let pubkey_full = [pubkey.clone(), vec![0u8; 24]].concat();
    db.create_account("bob", &pubkey_full).await.unwrap();

    // Search by pubkey prefix (uppercase hex)
    let results = db.search_accounts("ABCD", 50).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].username, "bob");

    // Search by lowercase hex should also work
    let results = db.search_accounts("abcd", 50).await.unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].username, "bob");
}

#[tokio::test]
async fn test_search_accounts_with_reputation_and_activity() {
    let db = setup_test_db().await;

    // Create two accounts
    let pubkey1 = vec![1u8; 32];
    let pubkey2 = vec![2u8; 32];
    db.create_account("alice", &pubkey1).await.unwrap();
    db.create_account("alicia", &pubkey2).await.unwrap();

    // Add reputation for alice
    sqlx::query!(
        "INSERT INTO reputation_changes (pubkey, change_amount, reason, block_timestamp_ns) VALUES (?, 100, 'test', 0)",
        pubkey1
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Add contract for alice
    let contract_id = vec![3u8; 32];
    let payment_method = "dct";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;
    sqlx::query!(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000, 'memo', 0, 'active', ?, ?, ?, 'usd')",
        contract_id,
        pubkey2,
        pubkey1,
        payment_method,
        stripe_payment_intent_id,
        stripe_customer_id
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Add offering for alice
    sqlx::query!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-1', 'Test', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', 0, 0)",
        pubkey1
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Search for "ali" - should return both, alice first (higher reputation)
    let results = db.search_accounts("ali", 50).await.unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].username, "alice");
    assert_eq!(results[0].reputation_score, 100);
    assert_eq!(results[0].contract_count, 1);
    assert_eq!(results[0].offering_count, 1);
    assert_eq!(results[1].username, "alicia");
    assert_eq!(results[1].reputation_score, 0);
    assert_eq!(results[1].contract_count, 1); // alicia is requester
}

#[tokio::test]
async fn test_search_accounts_limit() {
    let db = setup_test_db().await;

    // Create 3 accounts
    db.create_account("alice", &[1u8; 32]).await.unwrap();
    db.create_account("alicia", &[2u8; 32]).await.unwrap();
    db.create_account("alex", &[3u8; 32]).await.unwrap();

    // Search with limit of 2
    let results = db.search_accounts("al", 2).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_search_accounts_case_insensitive() {
    let db = setup_test_db().await;

    db.create_account("alice", &[1u8; 32]).await.unwrap();

    // Search with different cases - should all match
    let results1 = db.search_accounts("alice", 50).await.unwrap();
    let results2 = db.search_accounts("ALICE", 50).await.unwrap();
    let results3 = db.search_accounts("AlIcE", 50).await.unwrap();

    assert_eq!(results1.len(), 1);
    assert_eq!(results2.len(), 1);
    assert_eq!(results3.len(), 1);
}

#[test]
fn export_typescript_types() {
    PlatformStats::export().expect("Failed to export PlatformStats type");
    AccountSearchResult::export().expect("Failed to export AccountSearchResult type");
}
