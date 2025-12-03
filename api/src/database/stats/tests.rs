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
        let payment_method = "icpay";
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
        let payment_method = "icpay";
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
        let payment_method = "icpay";
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
    db.create_account("alice", &[1u8; 32], "alice@example.com")
        .await
        .unwrap();

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
    let account = db
        .create_account("alice", &[1u8; 32], "alice@example.com")
        .await
        .unwrap();

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
    db.create_account("bob", &pubkey_full, "bob@example.com")
        .await
        .unwrap();

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
    db.create_account("alice", &pubkey1, "alice@example.com")
        .await
        .unwrap();
    db.create_account("alicia", &pubkey2, "alicia@example.com")
        .await
        .unwrap();

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
    let payment_method = "icpay";
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
    db.create_account("alice", &[1u8; 32], "alice@example.com")
        .await
        .unwrap();
    db.create_account("alicia", &[2u8; 32], "alicia@example.com")
        .await
        .unwrap();
    db.create_account("alex", &[3u8; 32], "alex@example.com")
        .await
        .unwrap();

    // Search with limit of 2
    let results = db.search_accounts("al", 2).await.unwrap();
    assert_eq!(results.len(), 2);
}

#[tokio::test]
async fn test_search_accounts_case_insensitive() {
    let db = setup_test_db().await;

    db.create_account("alice", &[1u8; 32], "alice@example.com")
        .await
        .unwrap();

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
    ProviderTrustMetrics::export().expect("Failed to export ProviderTrustMetrics type");
}

// Trust score calculation tests

#[test]
fn test_trust_score_perfect_provider() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        Some(0.0), // no early cancellations
        Some(0.0), // no provisioning failures
        Some(0.0), // no rejections
        Some(2.0), // 2 hour response time (fast!)
        0,         // no negative reputation
        0,         // no stuck contracts
        1,         // active yesterday
        false,     // no active contracts
        15,        // 15 repeat customers (bonus!)
        98.0,      // 98% completion rate (bonus!)
        true,      // has contact info
    );

    // Base 100 + 5 (repeat customers) + 5 (completion rate) + 5 (fast response) = 115, clamped to 100
    assert_eq!(score, 100);
    assert!(!has_flags);
    assert!(flags.is_empty());
}

#[test]
fn test_trust_score_high_early_cancellation() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        Some(25.0), // 25% early cancellation rate (>20% threshold)
        Some(0.0),
        Some(0.0),
        Some(10.0),
        0,
        0,
        1,
        false,
        0,
        50.0,
        true, // has contact info
    );

    // Base 100 - 25 (early cancellation penalty) = 75
    assert_eq!(score, 75);
    assert!(has_flags);
    assert_eq!(flags.len(), 1);
    assert!(flags[0].contains("early cancellation"));
}

#[test]
fn test_trust_score_provisioning_failure() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        None,
        Some(20.0), // 20% provisioning failure (>15% threshold)
        Some(0.0),
        Some(10.0),
        0,
        0,
        1,
        false,
        0,
        50.0,
        true, // has contact info
    );

    // Base 100 - 20 (provisioning failure penalty) = 80
    assert_eq!(score, 80);
    assert!(has_flags);
    assert_eq!(flags.len(), 1);
    assert!(flags[0].contains("Provisioning failures"));
}

#[test]
fn test_trust_score_slow_response() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        None,
        None,
        None,
        Some(72.0), // 72 hours response time (>48h threshold)
        0,
        0,
        1,
        false,
        0,
        50.0,
        true, // has contact info
    );

    // Base 100 - 15 (slow response penalty) = 85
    assert_eq!(score, 85);
    assert!(has_flags);
    assert_eq!(flags.len(), 1);
    assert!(flags[0].contains("Slow response"));
}

#[test]
fn test_trust_score_ghost_risk() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        None, None, None, None, 0, 0, 14,   // 14 days since last check-in
        true, // HAS active contracts
        0, 50.0, true, // has contact info
    );

    // Base 100 - 10 (ghost risk penalty) = 90
    assert_eq!(score, 90);
    assert!(has_flags);
    assert_eq!(flags.len(), 1);
    assert!(flags[0].contains("Ghost risk"));
    assert!(flags[0].contains("14 days since last activity"));
}

#[test]
fn test_trust_score_ghost_risk_never_checked_in() {
    // -1 means provider never checked in
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        None, None, None, None, 0, 0, -1,   // never checked in
        true, // HAS active contracts
        0, 50.0, true,
    );

    // Base 100 - 10 (ghost risk penalty) = 90
    assert_eq!(score, 90);
    assert!(has_flags);
    assert_eq!(flags.len(), 1);
    assert!(flags[0].contains("Ghost risk"));
    assert!(flags[0].contains("no platform activity"));
}

#[test]
fn test_trust_score_negative_reputation() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        None, None, None, None, -75, // -75 reputation points in 90 days (<-50 threshold)
        0, 1, false, 0, 50.0, true, // has contact info
    );

    // Base 100 - 15 (negative reputation penalty) = 85
    assert_eq!(score, 85);
    assert!(has_flags);
    assert_eq!(flags.len(), 1);
    assert!(flags[0].contains("Negative reputation"));
}

#[test]
fn test_trust_score_multiple_flags() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        Some(30.0),         // high early cancellation
        Some(25.0),         // high provisioning failure
        Some(40.0),         // high rejection rate
        Some(60.0),         // slow response
        -100,               // very negative reputation
        10_000_000_000_000, // $10k stuck (>$5k threshold)
        10,                 // 10 days inactive
        true,               // with active contracts (ghost risk)
        0,
        30.0,
        false, // no contact info (adds 8th flag)
    );

    // All penalties: -25 -20 -15 -15 -15 -10 -10 -10 = -120, clamped to 0
    assert_eq!(score, 0);
    assert!(has_flags);
    assert_eq!(flags.len(), 8); // All 8 flags triggered (including no contact)
}

#[test]
fn test_trust_score_bonuses() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        Some(5.0),  // low early cancellation (no penalty)
        Some(5.0),  // low provisioning failure (no penalty)
        Some(10.0), // low rejection rate (no penalty)
        Some(2.0),  // fast response (<4h = bonus!)
        0,
        0,
        1,
        false,
        20,   // lots of repeat customers (bonus!)
        99.0, // high completion rate (bonus!)
        true, // has contact info
    );

    // Base 100 + 5 + 5 + 5 = 115, clamped to 100
    assert_eq!(score, 100);
    assert!(!has_flags);
    assert!(flags.is_empty());
}

#[test]
fn test_trust_score_no_contact_info() {
    let (score, has_flags, flags) = Database::calculate_trust_score_and_flags(
        None, None, None, None, 0, 0, 1, false, 0, 50.0, false, // no contact info
    );

    // Base 100 - 10 (no contact info penalty) = 90
    assert_eq!(score, 90);
    assert!(has_flags);
    assert_eq!(flags.len(), 1);
    assert!(flags[0].contains("No contact info"));
}

#[tokio::test]
async fn test_get_provider_trust_metrics_new_provider() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile first (required for contact info foreign key)
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Add contact info to avoid "no contact info" penalty
    sqlx::query(
        "INSERT INTO provider_profiles_contacts (provider_pubkey, contact_type, contact_value) VALUES (?, 'email', 'test@example.com')",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Provider with no contracts
    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();

    assert_eq!(metrics.trust_score, 100); // No penalties
    assert_eq!(metrics.total_contracts, 0);
    assert_eq!(metrics.completion_rate_pct, 0.0);
    assert!(metrics.is_new_provider); // <5 completed contracts
    assert_eq!(metrics.repeat_customer_count, 0);
}

#[tokio::test]
async fn test_get_provider_trust_metrics_with_contracts() {
    let db = setup_test_db().await;
    let provider_pubkey = vec![1u8; 32];
    let requester1 = vec![2u8; 32];
    let requester2 = vec![3u8; 32];

    // Create provider profile first (required for contact info foreign key)
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&provider_pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Add contact info to avoid "no contact info" penalty
    sqlx::query(
        "INSERT INTO provider_profiles_contacts (provider_pubkey, contact_type, contact_value) VALUES (?, 'email', 'provider@example.com')",
    )
    .bind(&provider_pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Add some completed contracts
    for i in 0..5 {
        let contract_id = vec![i + 10; 32];
        let requester = if i % 2 == 0 { &requester1 } else { &requester2 };
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, 'usd')",
            contract_id,
            requester,
            provider_pubkey,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Add a check-in
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let nonce = vec![0u8; 64];
    sqlx::query!(
        "INSERT INTO provider_check_ins (pubkey, memo, nonce_signature, block_timestamp_ns) VALUES (?, 'active', ?, ?)",
        provider_pubkey,
        nonce,
        now_ns
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let metrics = db
        .get_provider_trust_metrics(&provider_pubkey)
        .await
        .unwrap();

    assert_eq!(metrics.total_contracts, 5);
    assert_eq!(metrics.completion_rate_pct, 100.0); // All completed
    assert!(!metrics.is_new_provider); // >=5 completed contracts
    assert_eq!(metrics.days_since_last_checkin, 0); // Just checked in
    assert!(!metrics.has_critical_flags);
}

// Tier 3 Contextual Metrics Tests

#[tokio::test]
async fn test_provider_tenure_new() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Add 4 completed contracts (< 5 = "new")
    for i in 0..4 {
        let contract_id = vec![i + 10; 32];
        let requester = vec![2u8; 32];
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, 'usd')",
            contract_id,
            requester,
            pubkey,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert_eq!(metrics.provider_tenure, "new");
}

#[tokio::test]
async fn test_provider_tenure_growing() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Add 5 completed contracts (5 <= x <= 20 = "growing")
    for i in 0..5 {
        let contract_id = vec![i + 10; 32];
        let requester = vec![2u8; 32];
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, 'usd')",
            contract_id,
            requester,
            pubkey,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert_eq!(metrics.provider_tenure, "growing");
}

#[tokio::test]
async fn test_provider_tenure_growing_at_boundary() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Add exactly 20 completed contracts (upper bound of "growing")
    for i in 0..20 {
        let contract_id = vec![i + 10; 32];
        let requester = vec![2u8; 32];
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, 'usd')",
            contract_id,
            requester,
            pubkey,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert_eq!(metrics.provider_tenure, "growing");
}

#[tokio::test]
async fn test_provider_tenure_established() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Add 21 completed contracts (> 20 = "established")
    for i in 0..21 {
        let contract_id = vec![i + 10; 32];
        let requester = vec![2u8; 32];
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, 'usd')",
            contract_id,
            requester,
            pubkey,
            payment_method,
            stripe_payment_intent_id,
            stripe_customer_id
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert_eq!(metrics.provider_tenure, "established");
}

#[tokio::test]
async fn test_provider_tenure_zero_contracts() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // No contracts
    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert_eq!(metrics.provider_tenure, "new");
}

#[tokio::test]
async fn test_avg_contract_duration_ratio_none() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // No completed/cancelled contracts
    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.avg_contract_duration_ratio.is_none());
}

#[tokio::test]
async fn test_avg_contract_duration_ratio_completed_exact() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Contract that ran exactly as expected
    // Expected: 100 hours, Actual: 100 hours (ratio = 1.0)
    let contract_id = vec![10u8; 32];
    let requester = vec![2u8; 32];
    let start_ns = 1000000000000i64; // 1000 seconds
    let end_ns = start_ns + (100 * 3600 * 1_000_000_000i64); // +100 hours in nanoseconds
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    sqlx::query(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, duration_hours, start_timestamp_ns, end_timestamp_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', 100, ?, ?, ?, ?, ?, 'usd')",
    )
    .bind(&contract_id)
    .bind(&requester)
    .bind(&pubkey)
    .bind(start_ns)
    .bind(end_ns)
    .bind(payment_method)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.avg_contract_duration_ratio.is_some());
    let ratio = metrics.avg_contract_duration_ratio.unwrap();
    assert!((ratio - 1.0).abs() < 0.01); // Should be ~1.0
}

#[tokio::test]
async fn test_avg_contract_duration_ratio_completed_longer() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Contract that ran longer than expected
    // Expected: 100 hours, Actual: 150 hours (ratio = 1.5)
    let contract_id = vec![10u8; 32];
    let requester = vec![2u8; 32];
    let start_ns = 1000000000000i64;
    let end_ns = start_ns + (150 * 3600 * 1_000_000_000i64);
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    sqlx::query(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, duration_hours, start_timestamp_ns, end_timestamp_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', 100, ?, ?, ?, ?, ?, 'usd')",
    )
    .bind(&contract_id)
    .bind(&requester)
    .bind(&pubkey)
    .bind(start_ns)
    .bind(end_ns)
    .bind(payment_method)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.avg_contract_duration_ratio.is_some());
    let ratio = metrics.avg_contract_duration_ratio.unwrap();
    assert!((ratio - 1.5).abs() < 0.01); // Should be ~1.5
}

#[tokio::test]
async fn test_avg_contract_duration_ratio_cancelled_early() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Contract cancelled early
    // Expected: 100 hours, Actual: 25 hours (ratio = 0.25)
    let contract_id = vec![10u8; 32];
    let requester = vec![2u8; 32];
    let start_ns = 1000000000000i64;
    let status_updated_ns = start_ns + (25 * 3600 * 1_000_000_000i64);
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    sqlx::query(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, duration_hours, start_timestamp_ns, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', 100, ?, ?, ?, ?, ?, 'usd')",
    )
    .bind(&contract_id)
    .bind(&requester)
    .bind(&pubkey)
    .bind(start_ns)
    .bind(status_updated_ns)
    .bind(payment_method)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.avg_contract_duration_ratio.is_some());
    let ratio = metrics.avg_contract_duration_ratio.unwrap();
    assert!((ratio - 0.25).abs() < 0.01); // Should be ~0.25
}

#[tokio::test]
async fn test_avg_contract_duration_ratio_mixed_contracts() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Contract 1: completed, exact duration (ratio = 1.0)
    let contract_id1 = vec![10u8; 32];
    let requester = vec![2u8; 32];
    let start_ns1 = 1000000000000i64;
    let end_ns1 = start_ns1 + (100 * 3600 * 1_000_000_000i64);
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    sqlx::query(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, duration_hours, start_timestamp_ns, end_timestamp_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', 100, ?, ?, ?, ?, ?, 'usd')",
    )
    .bind(&contract_id1)
    .bind(&requester)
    .bind(&pubkey)
    .bind(start_ns1)
    .bind(end_ns1)
    .bind(payment_method)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Contract 2: cancelled early (ratio = 0.5)
    let contract_id2 = vec![11u8; 32];
    let start_ns2 = 2000000000000i64;
    let status_updated_ns2 = start_ns2 + (50 * 3600 * 1_000_000_000i64);

    sqlx::query(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, duration_hours, start_timestamp_ns, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', 100, ?, ?, ?, ?, ?, 'usd')",
    )
    .bind(&contract_id2)
    .bind(&requester)
    .bind(&pubkey)
    .bind(start_ns2)
    .bind(status_updated_ns2)
    .bind(payment_method)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Average ratio should be (1.0 + 0.5) / 2 = 0.75
    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.avg_contract_duration_ratio.is_some());
    let ratio = metrics.avg_contract_duration_ratio.unwrap();
    assert!((ratio - 0.75).abs() < 0.01); // Should be ~0.75
}

#[tokio::test]
async fn test_avg_contract_duration_ratio_ignores_active_contracts() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // Active contract (should be ignored)
    let contract_id = vec![10u8; 32];
    let requester = vec![2u8; 32];
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    sqlx::query(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, duration_hours, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'active', 100, ?, ?, ?, 'usd')",
    )
    .bind(&contract_id)
    .bind(&requester)
    .bind(&pubkey)
    .bind(payment_method)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.avg_contract_duration_ratio.is_none()); // Should be None (no completed/cancelled)
}

#[tokio::test]
async fn test_no_response_rate_pct_none() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    // No requests
    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.no_response_rate_pct.is_none());
}

#[tokio::test]
async fn test_no_response_rate_pct_zero() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;

    // Add requests from last 90 days that are NOT in "requested" status
    for i in 0..5 {
        let contract_id = vec![i + 10u8; 32];
        let requester = vec![2u8; 32];
        let created_ns = now_ns - (i as i64 + 1) * 10 * ns_per_day; // 10, 20, 30, 40, 50 days ago
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', ?, 'accepted', ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(created_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.no_response_rate_pct.is_some());
    assert_eq!(metrics.no_response_rate_pct.unwrap(), 0.0); // 0/5 = 0%
}

#[tokio::test]
async fn test_no_response_rate_pct_all_ignored() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;
    let cutoff_7d_ns = now_ns - 7 * ns_per_day;

    // Add requests older than 7 days still in "requested" status
    for i in 0..3 {
        let contract_id = vec![i + 10u8; 32];
        let requester = vec![2u8; 32];
        let created_ns = cutoff_7d_ns - (i as i64 + 1) * ns_per_day; // 8, 9, 10 days ago
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', ?, 'requested', ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(created_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.no_response_rate_pct.is_some());
    assert_eq!(metrics.no_response_rate_pct.unwrap(), 100.0); // 3/3 = 100%
}

#[tokio::test]
async fn test_no_response_rate_pct_partial_ignored() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;
    let cutoff_7d_ns = now_ns - 7 * ns_per_day;

    // Add 2 ignored requests (>7 days old, still "requested")
    for i in 0..2 {
        let contract_id = vec![i + 10u8; 32];
        let requester = vec![2u8; 32];
        let created_ns = cutoff_7d_ns - (i as i64 + 1) * ns_per_day;
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', ?, 'requested', ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(created_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Add 3 responded requests (either recent or not "requested")
    for i in 2..5 {
        let contract_id = vec![i + 10u8; 32];
        let requester = vec![2u8; 32];
        let created_ns = now_ns - (i as i64 + 1) * ns_per_day; // 3, 4, 5 days ago
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', ?, 'accepted', ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(created_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.no_response_rate_pct.is_some());
    let rate = metrics.no_response_rate_pct.unwrap();
    assert!((rate - 40.0).abs() < 0.01); // 2/5 = 40%
}

#[tokio::test]
async fn test_no_response_rate_pct_recent_requested_not_counted() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;

    // Add requests <7 days old in "requested" status (should NOT count as ignored)
    for i in 0..3 {
        let contract_id = vec![i + 10u8; 32];
        let requester = vec![2u8; 32];
        let created_ns = now_ns - (i as i64 + 1) * ns_per_day; // 1, 2, 3 days ago
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', ?, 'requested', ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(created_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.no_response_rate_pct.is_some());
    assert_eq!(metrics.no_response_rate_pct.unwrap(), 0.0); // 0/3 = 0% (none old enough)
}

#[tokio::test]
async fn test_no_response_rate_pct_only_counts_last_90_days() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;
    let cutoff_90d_ns = now_ns - 90 * ns_per_day;

    // Add requests older than 90 days (should be excluded from calculation)
    for i in 0..2 {
        let contract_id = vec![i + 10u8; 32];
        let requester = vec![2u8; 32];
        let created_ns = cutoff_90d_ns - (i as i64 + 1) * ns_per_day; // 91, 92 days ago
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', ?, 'requested', ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(created_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.no_response_rate_pct.is_none()); // None because no requests in last 90 days
}

#[tokio::test]
async fn test_abandonment_velocity_none_no_baseline() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;

    // Add contracts in recent period only (no baseline contracts)
    let contract_id = vec![10u8; 32];
    let requester = vec![2u8; 32];
    let status_updated_ns = now_ns - 10 * ns_per_day; // 10 days ago (recent)
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    sqlx::query(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
    )
    .bind(&contract_id)
    .bind(&requester)
    .bind(&pubkey)
    .bind(status_updated_ns)
    .bind(payment_method)
    .bind(stripe_payment_intent_id)
    .bind(stripe_customer_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.abandonment_velocity.is_none()); // None because baseline_total == 0
}

#[tokio::test]
async fn test_abandonment_velocity_zero_no_recent() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;

    // Add contracts in baseline period only (no recent contracts)
    // Baseline: 31-90 days ago
    for i in 0..5 {
        let contract_id = vec![i + 10u8; 32];
        let requester = vec![2u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64 * 5) * ns_per_day; // 40, 45, 50, 55, 60 days ago
        let payment_method = "icpay";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.abandonment_velocity.is_some());
    assert_eq!(metrics.abandonment_velocity.unwrap(), 0.0); // recent_total == 0, so recent_rate = 0.0
}

#[tokio::test]
async fn test_abandonment_velocity_stable() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;
    let requester = vec![2u8; 32];
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    // Baseline: 31-90 days ago - 10 contracts, 2 cancelled (20% rate)
    for i in 0..8 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    for i in 8..10 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Recent: last 30 days - 10 contracts, 2 cancelled (20% rate)
    for i in 20..28 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day; // 1-8 days ago
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    for i in 28..30 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day; // 9-10 days ago
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.abandonment_velocity.is_some());
    let velocity = metrics.abandonment_velocity.unwrap();
    assert!((velocity - 1.0).abs() < 0.01); // 0.2/0.2 = 1.0
}

#[tokio::test]
async fn test_abandonment_velocity_improving() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;
    let requester = vec![2u8; 32];
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    // Baseline: 31-90 days ago - 10 contracts, 4 cancelled (40% rate)
    for i in 0..6 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    for i in 6..10 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Recent: last 30 days - 10 contracts, 2 cancelled (20% rate - improved!)
    for i in 20..28 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    for i in 28..30 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.abandonment_velocity.is_some());
    let velocity = metrics.abandonment_velocity.unwrap();
    assert!((velocity - 0.5).abs() < 0.01); // 0.2/0.4 = 0.5
}

#[tokio::test]
async fn test_abandonment_velocity_spike() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;
    let requester = vec![2u8; 32];
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    // Baseline: 31-90 days ago - 10 contracts, 1 cancelled (10% rate)
    for i in 0..9 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    for i in 9..10 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Recent: last 30 days - 10 contracts, 5 cancelled (50% rate - spike!)
    for i in 20..25 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    for i in 25..30 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.abandonment_velocity.is_some());
    let velocity = metrics.abandonment_velocity.unwrap();
    assert!((velocity - 5.0).abs() < 0.01); // 0.5/0.1 = 5.0
}

#[tokio::test]
async fn test_abandonment_velocity_baseline_zero_cancellations() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create provider profile
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, 'Test', '1.0', '1.0', 0)",
    )
    .bind(&pubkey)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let ns_per_day: i64 = 24 * 3600 * 1_000_000_000;
    let requester = vec![2u8; 32];
    let payment_method = "icpay";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;

    // Baseline: 31-90 days ago - 10 contracts, 0 cancelled (0% rate)
    for i in 0..10 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (40 + i as i64) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    // Recent: last 30 days - 10 contracts, 2 cancelled (20% rate)
    for i in 20..28 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'completed', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }
    for i in 28..30 {
        let contract_id = vec![i + 10u8; 32];
        let status_updated_ns = now_ns - (i as i64 - 19) * ns_per_day;
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, status_updated_at_ns, payment_method, stripe_payment_intent_id, stripe_customer_id, currency) VALUES (?, ?, 'ssh', 'contact', ?, 'off-1', 1000000000, 'memo', 0, 'cancelled', ?, ?, ?, ?, 'usd')",
        )
        .bind(&contract_id)
        .bind(&requester)
        .bind(&pubkey)
        .bind(status_updated_ns)
        .bind(payment_method)
        .bind(stripe_payment_intent_id)
        .bind(stripe_customer_id)
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let metrics = db.get_provider_trust_metrics(&pubkey).await.unwrap();
    assert!(metrics.abandonment_velocity.is_some());
    let velocity = metrics.abandonment_velocity.unwrap();
    assert!((velocity - 0.2).abs() < 0.01); // baseline_rate == 0, so returns recent_rate directly (0.2)
}
