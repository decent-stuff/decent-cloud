use super::*;
use crate::database::test_helpers::setup_test_db;
use sqlx::Row;

async fn insert_contract_request(
    db: &Database,
    contract_id: &[u8],
    requester_pubkey: &[u8],
    provider_pubkey: &[u8],
    offering_id: &str,
    created_at_ns: i64,
    status: &str,
) {
    let payment_method = "test";
    let payment_status = "succeeded"; // Test payments auto-succeed without checkout
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;
    sqlx::query!(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency) VALUES ($1, $2, 'ssh-key', 'contact', $3, $4, 1000, 'memo', $5, $6, $7, $8, $9, $10, 'usd')",
        contract_id,
        requester_pubkey,
        provider_pubkey,
        offering_id,
        created_at_ns,
        status,
        payment_method,
        stripe_payment_intent_id,
        stripe_customer_id,
        payment_status
    )
    .execute(&db.pool)
    .await
    .unwrap();
}

struct StripeContractParams {
    contract_id: Vec<u8>,
    requester_pubkey: Vec<u8>,
    provider_pubkey: Vec<u8>,
    offering_id: String,
    payment_intent_id: String,
    payment_status: String,
    payment_amount_e9s: i64,
    start_timestamp_ns: i64,
    end_timestamp_ns: i64,
}

async fn insert_stripe_contract_with_timestamps(db: &Database, params: StripeContractParams) {
    let stripe_payment_intent_id: Option<&str> = Some(&params.payment_intent_id);
    let stripe_customer_id: Option<&str> = None;
    let payment_method: &str = "stripe";
    let status: &str = "requested";
    let ssh_pubkey: &str = "ssh-key";
    let contact: &str = "contact";
    let memo: &str = "memo";
    let created_at_ns: i64 = 0;

    sqlx::query!(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, 'usd')",
        params.contract_id,
        params.requester_pubkey,
        ssh_pubkey,
        contact,
        params.provider_pubkey,
        params.offering_id,
        params.payment_amount_e9s,
        params.start_timestamp_ns,
        params.end_timestamp_ns,
        memo,
        created_at_ns,
        status,
        payment_method,
        stripe_payment_intent_id,
        stripe_customer_id,
        params.payment_status
    )
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_get_user_contracts_empty() {
    let db = setup_test_db().await;
    let contracts = db.get_user_contracts(&[1u8; 32]).await.unwrap();
    assert_eq!(contracts.len(), 0);
}

#[tokio::test]
async fn test_get_user_contracts() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![3u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    let contracts = db.get_user_contracts(&user_pk).await.unwrap();
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].contract_id, hex::encode(contract_id));
}

#[tokio::test]
async fn test_get_user_contracts_resolves_provider_username() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![3u8; 32];

    // Provider has an account with a known username; requester does not.
    db.create_account("provider_alice", &provider_pk, "provider@example.com")
        .await
        .expect("Failed to create provider account");

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    let contracts = db.get_user_contracts(&requester_pk).await.unwrap();
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].provider_username,
        Some("provider_alice".to_string()),
        "get_user_contracts should resolve provider_username via account_public_keys join"
    );
    assert_eq!(
        contracts[0].requester_username, None,
        "requester without an account should yield requester_username = None"
    );

    // Single-contract lookup must resolve the username too.
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(
        contract.provider_username,
        Some("provider_alice".to_string()),
        "get_contract should resolve provider_username via account_public_keys join"
    );
}

#[tokio::test]
async fn test_get_provider_contracts() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![3u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    let contracts = db.get_provider_contracts(&provider_pk).await.unwrap();
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].provider_pubkey, hex::encode(provider_pk));
}

#[tokio::test]
async fn test_get_pending_provider_contracts() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];

    // Insert contracts with different statuses
    let contract1 = vec![1u8; 32];
    let requester = vec![1u8; 32];
    insert_contract_request(
        &db,
        &contract1,
        &requester,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;
    let contract2 = vec![2u8; 32];
    insert_contract_request(
        &db,
        &contract2,
        &requester,
        &provider_pk,
        "off-1",
        500,
        "pending",
    )
    .await;
    let contract3 = vec![3u8; 32];
    insert_contract_request(
        &db,
        &contract3,
        &requester,
        &provider_pk,
        "off-1",
        1000,
        "active",
    )
    .await;

    let contracts = db
        .get_pending_provider_contracts(&provider_pk)
        .await
        .unwrap();
    // Should return both 'requested' and 'pending' contracts, but not 'active'
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().any(|c| c.status == "requested"));
    assert!(contracts.iter().any(|c| c.status == "pending"));
}

#[tokio::test]
async fn test_get_contract_by_id() {
    let db = setup_test_db().await;
    let contract_id = vec![3u8; 32];

    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    let contract = db.get_contract(&contract_id).await.unwrap();
    assert!(contract.is_some());
    assert_eq!(contract.unwrap().contract_id, hex::encode(contract_id));
}

#[tokio::test]
async fn test_get_contract_by_id_not_found() {
    let db = setup_test_db().await;
    let contract = db.get_contract(&[99u8; 32]).await.unwrap();
    assert!(contract.is_none());
}

#[tokio::test]
async fn test_create_rental_request_with_test_payment_method() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-payment-1', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: Some("Test rental".to_string()),
        duration_hours: None,
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.payment_method, "test");
    // Test payment method auto-succeeds without a checkout flow.
    assert_eq!(contract.payment_status, "succeeded");
}

#[tokio::test]
async fn test_create_rental_request_with_stripe_payment_method() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-payment-2', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: Some("Test rental".to_string()),
        duration_hours: None,
        payment_method: Some("stripe".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.payment_method, "stripe");
}

#[tokio::test]
async fn test_create_rental_request_invalid_payment_method() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-payment-3', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: Some("Test rental".to_string()),
        duration_hours: None,
        payment_method: Some("paypal".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let result = db.create_rental_request(&user_pk, params).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid payment method"));
}

#[tokio::test]
async fn test_list_contracts_pagination() {
    let db = setup_test_db().await;

    let requester = vec![1u8; 32];
    let provider = vec![2u8; 32];
    for i in 0..5 {
        let contract_id = vec![i as u8; 32];
        insert_contract_request(
            &db,
            &contract_id,
            &requester,
            &provider,
            "off-1",
            i * 1000,
            "pending",
        )
        .await;
    }

    let page1 = db.list_contracts(2, 0).await.unwrap();
    assert_eq!(page1.len(), 2);

    let page2 = db.list_contracts(2, 2).await.unwrap();
    assert_eq!(page2.len(), 2);

    let page3 = db.list_contracts(2, 4).await.unwrap();
    assert_eq!(page3.len(), 1);
}

#[tokio::test]
async fn test_create_rental_request_success() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering first (no explicit id, let it auto-increment)
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-rental-1', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-rsa AAAAB3...".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: Some("Test rental".to_string()),
        duration_hours: None,
        payment_method: Some("stripe".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    assert_eq!(contract_id.len(), 32);

    // Verify contract was created
    let contract = db.get_contract(&contract_id).await.unwrap();
    assert!(contract.is_some());
    let contract = contract.unwrap();
    assert_eq!(contract.requester_pubkey, hex::encode(user_pk));
    assert_eq!(contract.provider_pubkey, hex::encode(provider_pk));
    assert_eq!(contract.offering_id, "off-rental-1");
    assert_eq!(contract.status, "requested");
    assert_eq!(contract.requester_ssh_pubkey, "ssh-rsa AAAAB3...");
    assert_eq!(contract.requester_contact, "email:test@example.com");
    assert_eq!(contract.request_memo, "Test rental");
    assert_eq!(contract.payment_amount_e9s, 100_000_000_000);
    assert_eq!(contract.payment_method, "stripe");
    assert_eq!(contract.stripe_payment_intent_id, None);
    assert_eq!(contract.stripe_customer_id, None);
}

#[tokio::test]
async fn test_create_rental_request_with_defaults() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create user account
    let account = db
        .create_account("testuser", &user_pk, "test@example.com")
        .await
        .unwrap();

    // Create offering (no explicit id)
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-rental-2', 'Test Server', 'USD', 50.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    // Add SSH key to account
    db.add_account_external_key(&account.id, "ssh-ed25519", "AAAAC3...user-key", None, None)
        .await
        .unwrap();

    // Note: Account email (test@example.com) is set during create_account
    // No need to add contact email - account email is used as default contact

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: None,
        contact_method: None,
        request_memo: None,
        duration_hours: None,
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();

    // Verify defaults were used (account email as contact)
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.requester_ssh_pubkey, "AAAAC3...user-key");
    assert_eq!(contract.requester_contact, "email:test@example.com");
    assert_eq!(contract.request_memo, "Rental request for Test Server");
}

#[tokio::test]
async fn test_create_rental_request_offering_not_found() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];

    let params = RentalRequestParams {
        offering_db_id: 999,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: None,
        duration_hours: None,
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let result = db.create_rental_request(&user_pk, params).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Offering not found"));
}

#[tokio::test]
async fn test_create_rental_request_calculates_price() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering with specific price (no explicit id)
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-rental-3', 'Expensive Server', 'USD', 499.99, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("contact".to_string()),
        request_memo: None,
        duration_hours: None,
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    // 499.99 * 1_000_000_000 = 499_990_000_000
    assert_eq!(contract.payment_amount_e9s, 499_990_000_000);
}

#[tokio::test]
async fn test_create_rental_request_eur_stripe() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering with EUR currency
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-eur-1', 'EU Server', 'EUR', 89.99, 0, 'public', 'compute', 'monthly', 'in_stock', 'DE', 'Berlin', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:eu@example.com".to_string()),
        request_memo: Some("EU rental".to_string()),
        duration_hours: Some(720),
        payment_method: Some("stripe".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    assert_eq!(contract.currency, "EUR");
    assert_eq!(contract.payment_method, "stripe");
    assert_eq!(contract.payment_amount_e9s, 89_990_000_000);
    assert_eq!(contract.payment_status, "pending"); // Stripe payments start as pending
}

#[tokio::test]
async fn test_update_contract_status_records_history() {
    let db = setup_test_db().await;
    let contract_id = vec![9u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    db.update_contract_status(&contract_id, "accepted", &provider_pk, Some("all good"))
        .await
        .unwrap();

    let contract_id_param = contract_id.clone();
    let status: String = sqlx::query_scalar!(
        r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = $1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(status, "accepted");

    let contract_id_param = contract_id.clone();
    let history = sqlx::query!(
        r#"SELECT old_status as "old_status!", new_status as "new_status!", change_memo FROM contract_status_history WHERE contract_id = $1 ORDER BY changed_at_ns DESC LIMIT 1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(history.old_status, "pending");
    assert_eq!(history.new_status, "accepted");
    assert_eq!(history.change_memo.as_deref(), Some("all good"));
}

#[tokio::test]
async fn test_update_contract_status_rejects_non_provider() {
    let db = setup_test_db().await;
    let contract_id = vec![5u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let attacker_pk = vec![3u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-2",
        0,
        "requested",
    )
    .await;

    let result = db
        .update_contract_status(&contract_id, "accepted", &attacker_pk, None)
        .await;
    assert!(result.is_err());

    let contract_id_param = contract_id.clone();
    let history_count: i64 = sqlx::query_scalar!(
        r#"SELECT COUNT(*) as "count!: i64" FROM contract_status_history WHERE contract_id = $1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(history_count, 0);
}

#[tokio::test]
async fn test_add_provisioning_details_persists_connection_info() {
    let db = setup_test_db().await;
    let contract_id = vec![7u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-3",
        0,
        "accepted",
    )
    .await;

    db.add_provisioning_details(&contract_id, "ip:1.2.3.4\nuser:root")
        .await
        .unwrap();

    let contract_id_param = contract_id.clone();
    let provisioning = sqlx::query!(
        "SELECT provisioning_instance_details FROM contract_sign_requests WHERE contract_id = $1",
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        provisioning.provisioning_instance_details.as_deref(),
        Some("ip:1.2.3.4\nuser:root")
    );

    let contract_id_param = contract_id.clone();
    let detail_row = sqlx::query!(
        r#"SELECT contract_id as "contract_id!", instance_ip, instance_credentials, connection_instructions, provisioned_at_ns as "provisioned_at_ns!" FROM contract_provisioning_details WHERE contract_id = $1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(detail_row.contract_id, contract_id);
    assert_eq!(detail_row.instance_ip, None);
    assert_eq!(detail_row.instance_credentials, None);
    assert_eq!(
        detail_row.connection_instructions.as_deref(),
        Some("ip:1.2.3.4\nuser:root")
    );
    assert!(detail_row.provisioned_at_ns > 0);
}

#[tokio::test]
async fn test_add_provisioning_details_extracts_gateway_fields() {
    let db = setup_test_db().await;
    let contract_id = vec![77u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-gw",
        0,
        "accepted",
    )
    .await;

    let instance_json = r#"{
        "external_id": "vm-123",
        "ip_address": "10.0.1.5",
        "ssh_port": 22,
        "gateway_slug": "k7m2p4",
        "gateway_subdomain": "k7m2p4.dc-lk.decent-cloud.org",
        "gateway_ssh_port": 20000,
        "gateway_port_range_start": 20000,
        "gateway_port_range_end": 20009
    }"#;

    db.add_provisioning_details(&contract_id, instance_json)
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.gateway_slug.as_deref(), Some("k7m2p4"));
    assert_eq!(
        contract.gateway_subdomain.as_deref(),
        Some("k7m2p4.dc-lk.decent-cloud.org")
    );
    assert_eq!(contract.gateway_ssh_port, Some(20000));
    assert_eq!(contract.gateway_port_range_start, Some(20000));
    assert_eq!(contract.gateway_port_range_end, Some(20009));
}

#[tokio::test]
async fn test_add_provisioning_details_handles_missing_gateway_fields() {
    let db = setup_test_db().await;
    let contract_id = vec![78u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-nogw",
        0,
        "accepted",
    )
    .await;

    // Instance without gateway fields (legacy or gateway disabled)
    let instance_json = r#"{"external_id": "vm-456", "ip_address": "10.0.1.6", "ssh_port": 22}"#;

    db.add_provisioning_details(&contract_id, instance_json)
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.gateway_slug, None);
    assert_eq!(contract.gateway_subdomain, None);
    assert_eq!(contract.gateway_ssh_port, None);
    assert_eq!(contract.gateway_port_range_start, None);
    assert_eq!(contract.gateway_port_range_end, None);
}

#[tokio::test]
async fn test_request_ssh_key_rotation_stages_pending_key_until_completion() {
    let db = setup_test_db().await;
    let contract_id = vec![79u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let new_key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAINewKey staged@test";

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-rotate",
        0,
        "active",
    )
    .await;

    db.add_provisioning_details(&contract_id, "ip:1.2.3.4\nuser:root")
        .await
        .unwrap();

    db.request_ssh_key_rotation(&contract_id, new_key)
        .await
        .unwrap();

    let staged = sqlx::query(
        r#"SELECT c.requester_ssh_pubkey,
                  pd.pending_requester_ssh_pubkey,
                  pd.ssh_key_rotation_requested_at_ns
           FROM contract_sign_requests c
           INNER JOIN contract_provisioning_details pd ON pd.contract_id = c.contract_id
           WHERE c.contract_id = $1"#,
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(staged.get::<String, _>("requester_ssh_pubkey"), "ssh-key");
    assert_eq!(
        staged
            .get::<Option<String>, _>("pending_requester_ssh_pubkey")
            .as_deref(),
        Some(new_key)
    );
    assert!(staged
        .get::<Option<i64>, _>("ssh_key_rotation_requested_at_ns")
        .is_some());

    let pending = db
        .get_pending_ssh_key_rotations(&provider_pk)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].contract_id, hex::encode(&contract_id));
    assert_eq!(pending[0].requester_ssh_pubkey, new_key);

    let completed_key = db.complete_ssh_key_rotation(&contract_id).await.unwrap();
    assert_eq!(completed_key, new_key);

    let applied = sqlx::query(
        r#"SELECT c.requester_ssh_pubkey,
                  pd.pending_requester_ssh_pubkey,
                  pd.ssh_key_rotation_requested_at_ns
           FROM contract_sign_requests c
           INNER JOIN contract_provisioning_details pd ON pd.contract_id = c.contract_id
           WHERE c.contract_id = $1"#,
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(applied.get::<String, _>("requester_ssh_pubkey"), new_key);
    assert!(applied
        .get::<Option<String>, _>("pending_requester_ssh_pubkey")
        .is_none());
    assert!(applied
        .get::<Option<i64>, _>("ssh_key_rotation_requested_at_ns")
        .is_none());
}

#[tokio::test]
async fn test_get_ssh_key_rotation_events_for_user_returns_rotation_events() {
    let db = setup_test_db().await;
    let contract_id = vec![80u8; 32];
    let requester_pk = vec![3u8; 32];
    let provider_pk = vec![4u8; 32];
    let other_pk = vec![5u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-sse-rot",
        0,
        "active",
    )
    .await;

    db.add_provisioning_details(&contract_id, "ip:1.2.3.4\nuser:root")
        .await
        .unwrap();

    let other_contract_id = vec![81u8; 32];
    insert_contract_request(
        &db,
        &other_contract_id,
        &other_pk,
        &provider_pk,
        "off-sse-rot2",
        0,
        "active",
    )
    .await;

    db.add_provisioning_details(&other_contract_id, "ip:5.6.7.8\nuser:root")
        .await
        .unwrap();

    let before_rotation = crate::now_ns().unwrap();
    db.request_ssh_key_rotation(&contract_id, "ssh-ed25519 NEWKEY1")
        .await
        .unwrap();
    db.request_ssh_key_rotation(&other_contract_id, "ssh-ed25519 NEWKEY2")
        .await
        .unwrap();

    let events = db
        .get_ssh_key_rotation_events_for_user(&requester_pk, 0)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].contract_id, hex::encode(&contract_id));
    assert_eq!(events[0].event_type, "ssh_key_rotation");
    assert_eq!(events[0].actor, "tenant");
    assert!(events[0].created_at >= before_rotation);

    db.complete_ssh_key_rotation(&contract_id).await.unwrap();
    db.insert_contract_event(
        &contract_id,
        "ssh_key_rotation_complete",
        None,
        None,
        "provider",
        Some("SSH key rotated to ssh-ed25519 NEWKEY... by agent"),
    )
    .await
    .unwrap();

    let all_events = db
        .get_ssh_key_rotation_events_for_user(&requester_pk, 0)
        .await
        .unwrap();
    assert_eq!(all_events.len(), 2);
    assert_eq!(all_events[0].event_type, "ssh_key_rotation");
    assert_eq!(all_events[1].event_type, "ssh_key_rotation_complete");

    let after_all = all_events[1].created_at;
    let filtered = db
        .get_ssh_key_rotation_events_for_user(&requester_pk, after_all)
        .await
        .unwrap();
    assert!(
        filtered.is_empty(),
        "after_ns filter should exclude events at or before the cutoff"
    );

    let other_events = db
        .get_ssh_key_rotation_events_for_user(&other_pk, 0)
        .await
        .unwrap();
    assert_eq!(other_events.len(), 1);
    assert_eq!(other_events[0].contract_id, hex::encode(&other_contract_id));
}

#[tokio::test]
async fn test_cancel_contract_success_requested() {
    let db = setup_test_db().await;
    let contract_id = vec![10u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;

    db.cancel_contract(
        &contract_id,
        &requester_pk,
        Some("User requested cancellation"),
        None,
    )
    .await
    .unwrap();

    let contract_id_param = contract_id.clone();
    let status: String = sqlx::query_scalar!(
        r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = $1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(status, "cancelled");

    let contract_id_param = contract_id.clone();
    let history = sqlx::query!(
        r#"SELECT old_status as "old_status!", new_status as "new_status!", change_memo FROM contract_status_history WHERE contract_id = $1 ORDER BY changed_at_ns DESC LIMIT 1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(history.old_status, "requested");
    assert_eq!(history.new_status, "cancelled");
    assert_eq!(
        history.change_memo.as_deref(),
        Some("User requested cancellation")
    );
}

#[tokio::test]
async fn test_cancel_contract_success_all_cancellable_statuses() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    let cancellable_statuses = [
        "requested",
        "pending",
        "accepted",
        "provisioning",
        "provisioned",
        "active",
    ];

    for (i, status) in cancellable_statuses.iter().enumerate() {
        let contract_id = vec![10 + i as u8; 32];

        insert_contract_request(
            &db,
            &contract_id,
            &requester_pk,
            &provider_pk,
            "off-1",
            0,
            status,
        )
        .await;

        let result = db
            .cancel_contract(&contract_id, &requester_pk, None, None)
            .await;
        assert!(
            result.is_ok(),
            "Cancellation should succeed for status '{}', but got error: {:?}",
            status,
            result.err()
        );

        let contract_id_param = contract_id.clone();
        let new_status: String = sqlx::query_scalar!(
            r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = $1"#,
            contract_id_param
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(new_status, "cancelled");
    }
}

#[tokio::test]
async fn test_cancel_contract_rejects_unauthorized_user() {
    let db = setup_test_db().await;
    let contract_id = vec![11u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let attacker_pk = vec![3u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;

    let result = db
        .cancel_contract(&contract_id, &attacker_pk, None, None)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("only the requester can cancel"));

    let contract_id_param = contract_id.clone();
    let status: String = sqlx::query_scalar!(
        r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = $1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(status, "requested");
}

#[tokio::test]
async fn test_cancel_contract_rejects_provider_cancellation() {
    let db = setup_test_db().await;
    let contract_id = vec![12u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    let result = db
        .cancel_contract(&contract_id, &provider_pk, None, None)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("only the requester can cancel"));
}

#[tokio::test]
async fn test_cancel_contract_fails_for_non_cancellable_statuses() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Only terminal statuses are non-cancellable (rejected, cancelled, completed)
    let non_cancellable_statuses = ["rejected", "cancelled", "completed"];

    for (i, status) in non_cancellable_statuses.iter().enumerate() {
        let contract_id = vec![20 + i as u8; 32];

        insert_contract_request(
            &db,
            &contract_id,
            &requester_pk,
            &provider_pk,
            "off-1",
            0,
            status,
        )
        .await;

        let result = db
            .cancel_contract(&contract_id, &requester_pk, None, None)
            .await;
        assert!(
            result.is_err(),
            "Cancellation should fail for status '{}'",
            status
        );
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("cannot be cancelled"),
            "Error message should indicate status cannot be cancelled for '{}'",
            status
        );
    }
}

#[tokio::test]
async fn test_cancel_contract_not_found_includes_hex_id() {
    let db = setup_test_db().await;
    let nonexistent_id = vec![99u8; 32];
    let requester_pk = vec![1u8; 32];

    let result = db
        .cancel_contract(&nonexistent_id, &requester_pk, None, None)
        .await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Contract not found"));
    assert!(error_msg.contains(&hex::encode(&nonexistent_id)));
}

#[tokio::test]
async fn test_payment_status_test_payment_succeeds_immediately() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-payment-status-1', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: Some("Test rental".to_string()),
        duration_hours: None,
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    // Test payments auto-succeed without a checkout flow, so payment_status is 'succeeded'
    assert_eq!(contract.payment_method, "test");
    assert_eq!(contract.payment_status, "succeeded");
}

#[tokio::test]
async fn test_payment_status_stripe_payment_starts_pending() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES ($1, 'off-payment-status-2', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0) RETURNING id as \"id!\"",
        provider_pk_clone
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: Some("Test rental".to_string()),
        duration_hours: None,
        payment_method: Some("stripe".to_string()),
        buyer_address: None,
        operating_system: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    // Stripe payments require webhook confirmation, so payment_status should start as 'pending'
    assert_eq!(contract.payment_method, "stripe");
    assert_eq!(contract.payment_status, "pending");
}

// Refund calculation tests
// Note: service_start_ns represents when service was provisioned (user got access)
// If None, service never started -> full refund

#[test]
fn test_calculate_prorated_refund_service_never_started() {
    // Service never provisioned (service_start_ns = None) -> full refund
    let payment_amount_e9s = 1_000_000_000;
    let current_timestamp_ns = 1500;

    let refund = Database::calculate_prorated_refund(
        payment_amount_e9s,
        None, // Service never started
        Some(2000),
        current_timestamp_ns,
        0,
    );
    assert_eq!(refund, payment_amount_e9s); // Full refund
}

#[test]
fn test_calculate_prorated_refund_full_refund_before_service_start() {
    // Service provisioned but current time is before provisioning -> full refund
    let payment_amount_e9s = 1_000_000_000;
    let service_start_ns = 1000;
    let end_timestamp_ns = 2000;
    let current_timestamp_ns = 500; // Before service started

    let refund = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        0,
    );

    assert_eq!(refund, payment_amount_e9s);
}

#[test]
fn test_calculate_prorated_refund_half_used() {
    // Service is 50% through, should get 50% refund
    let payment_amount_e9s = 1_000_000_000;
    let service_start_ns = 1000;
    let end_timestamp_ns = 3000; // Duration: 2000ns
    let current_timestamp_ns = 2000; // Halfway through service

    let refund = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        0,
    );

    // Should be approximately 50% (500M e9s)
    assert!((499_000_000..=501_000_000).contains(&refund));
}

#[test]
fn test_calculate_prorated_refund_no_refund_after_end() {
    // Contract has already ended, no refund
    let payment_amount_e9s = 1_000_000_000;
    let service_start_ns = 1000;
    let end_timestamp_ns = 2000;
    let current_timestamp_ns = 3000; // After end

    let refund = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        0,
    );

    assert_eq!(refund, 0);
}

#[test]
fn test_calculate_prorated_refund_missing_end_timestamp() {
    // Missing end timestamp should return 0 (invalid contract)
    let payment_amount_e9s = 1_000_000_000;
    let current_timestamp_ns = 1500;

    let refund = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(1000),
        None,
        current_timestamp_ns,
        0,
    );
    assert_eq!(refund, 0);
}

#[test]
fn test_calculate_prorated_refund_90_percent_remaining() {
    // Used 10% of service, should get 90% refund
    let payment_amount_e9s = 1_000_000_000;
    let service_start_ns = 0;
    let end_timestamp_ns = 10_000; // Duration: 10,000ns
    let current_timestamp_ns = 1_000; // 10% used

    let refund = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        0,
    );

    // Should be approximately 90% (900M e9s)
    assert!((899_000_000..=901_000_000).contains(&refund));
}

#[test]
fn test_calculate_prorated_refund_credits_paused_time() {
    // Without pause credit, customer used 60% of the window -> 40% refund.
    // With 30% of the window credited as pause, billable use drops to 30% -> 70% refund.
    // This invariant -- "every paused nanosecond comes back to the customer" --
    // is the load-bearing reason for adding total_paused_ns to the formula.
    let payment_amount_e9s = 1_000_000_000;
    let service_start_ns: i64 = 0;
    let end_timestamp_ns: i64 = 10_000; // duration = 10_000
    let current_timestamp_ns: i64 = 6_000; // elapsed = 6_000

    // Baseline (no pause): 40% remaining -> 400M.
    let baseline = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        0,
    );
    assert!(
        (399_000_000..=401_000_000).contains(&baseline),
        "baseline must be ~40%, got {}",
        baseline
    );

    // With 3000ns paused, billable_used = 3000 -> remaining = 7000 -> 70% refund.
    let credited = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        3_000,
    );
    assert!(
        (699_000_000..=701_000_000).contains(&credited),
        "with 30% paused credit, refund must be ~70%, got {}",
        credited
    );

    // Pause covering the whole elapsed window -> full refund (customer was
    // never charged for any time they had service).
    let fully_paused = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        6_000,
    );
    assert_eq!(
        fully_paused, payment_amount_e9s,
        "if the paused window covers all elapsed time, full refund is owed"
    );

    // Defensive: negative pause input is treated as zero (cannot inflate refunds).
    let neg = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(service_start_ns),
        Some(end_timestamp_ns),
        current_timestamp_ns,
        -1_000,
    );
    assert_eq!(
        neg, baseline,
        "negative pause must be sanitized, refund must equal baseline"
    );
}

#[tokio::test]
async fn test_cancel_contract_test_payment_no_refund() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![100u8; 32];

    // Insert Test payment contract (auto-succeeded, no checkout)
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;

    // Cancel without a Stripe client. The Test payment method has no refund
    // path, so no refund is issued and payment_status is left unchanged.
    let result = db
        .cancel_contract(&contract_id, &requester_pk, Some("Test cancel"), None)
        .await;

    assert!(result.is_ok());

    // Verify contract is cancelled
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    assert_eq!(contract.payment_status, "succeeded"); // Test payment status unchanged
    assert!(contract.refund_amount_e9s.is_none());
    assert!(contract.stripe_refund_id.is_none());
}

#[tokio::test]
async fn test_cancel_contract_stripe_payment_without_client() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![101u8; 32];

    // Insert Stripe contract with succeeded payment
    // Use future timestamps so refund is calculated (contract hasn't expired)
    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let start_ns = now_ns - 1_000_000_000; // Started 1 second ago
    let end_ns = now_ns + 10_000_000_000; // Ends in 10 seconds
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-1".to_string(),
            payment_intent_id: "pi_test_123".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1000000000,
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;

    // Cancel without Stripe client (refund amount calculated but not processed)
    let result = db
        .cancel_contract(&contract_id, &requester_pk, Some("Test cancel"), None)
        .await;

    assert!(result.is_ok());

    // Verify contract is cancelled with refund amount but no refund ID
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // R5: no Stripe client -> refund NOT issued, so payment_status must stay
    // 'succeeded' (never claim 'refunded' with no money returned).
    assert_eq!(contract.payment_status, "succeeded");
    assert!(contract.refund_amount_e9s.is_some());
    assert!(contract.stripe_refund_id.is_none()); // No client to process refund
}

#[tokio::test]
async fn test_cancel_contract_unauthorized() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let unauthorized_pk = vec![99u8; 32];
    let contract_id = vec![102u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;

    // Attempt cancel by unauthorized user
    let result = db
        .cancel_contract(
            &contract_id,
            &unauthorized_pk,
            Some("Unauthorized"),
            None,
        )
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unauthorized"));

    // Verify contract still in original status
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "requested");
}

#[tokio::test]
async fn test_cancel_contract_invalid_status() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![103u8; 32];

    // Insert contract in non-cancellable status (terminal status)
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "completed", // Terminal status - not cancellable
    )
    .await;

    // Attempt cancel
    let result = db
        .cancel_contract(&contract_id, &requester_pk, Some("Test cancel"), None)
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("cannot be cancelled"));

    // Verify contract still in original status
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "completed");
}

/// The Stripe cancel path must refund the prorated remainder for unused time.
/// Under Stripe-only no funds are ever pre-released to the provider, so the
/// refund equals the gross prorated amount. Regression guard for the over-refund
/// bug where Stripe refunded the full gross amount ignoring time used.
#[tokio::test]
async fn test_cancel_stripe_contract_refund_is_prorated() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![71u8; 32];
    let payment_amount_e9s = 1_000_000_000i64; // $1.00 == 100 cents

    let now_ns = crate::now_ns().unwrap();
    let start_ns = now_ns - (10 * 24 * 3600 * 1_000_000_000i64); // started 10 days ago
    let end_ns = start_ns + (30 * 24 * 3600 * 1_000_000_000i64); // 30-day term

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "off-1".to_string(),
            payment_intent_id: "pi_test_released".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s,
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;

    // Service started at start_ns; no funds are ever pre-released under
    // Stripe-only, so the refund is the gross prorated amount for unused time.
    sqlx::query(
        "UPDATE contract_sign_requests SET provisioning_completed_at_ns = $1 WHERE contract_id = $2",
    )
    .bind(start_ns)
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_before_cancel = crate::now_ns().unwrap();
    db.cancel_contract(&contract_id, &requester_pk, Some("cancel"), None)
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // R5: no Stripe client -> refund NOT issued, payment_status stays 'succeeded'.
    assert_eq!(contract.payment_status, "succeeded");

    // Refund equals the gross prorated remainder (no released subtraction).
    let gross = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(start_ns),
        Some(end_ns),
        now_before_cancel,
        0,
    );
    let refund = contract
        .refund_amount_e9s
        .expect("stripe cancel must record a refund");
    assert!(
        (refund - gross).abs() < 1_000_000,
        "Stripe refund must equal the gross prorated remainder: expected ~{gross}, got {refund}"
    );
}

/// Dispute-lost refund equals the gross prorated remainder for unused time,
/// matching the cancel/reject paths. Under Stripe-only no funds are ever
/// pre-released, so there is nothing to subtract. Regression guard for R9.
#[tokio::test]
async fn test_dispute_lost_refund_is_gross_prorated() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![81u8; 32];
    let payment_amount_e9s = 1_000_000_000i64; // $1.00 == 100 cents

    let now_ns = crate::now_ns().unwrap();
    let start_ns = now_ns - (10 * 24 * 3600 * 1_000_000_000i64); // started 10 days ago
    let end_ns = start_ns + (30 * 24 * 3600 * 1_000_000_000i64); // 30-day term

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk,
            provider_pubkey: provider_pk,
            offering_id: "off-1".to_string(),
            payment_intent_id: "pi_test_dispute".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s,
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;

    // Service started at start_ns; under Stripe-only there are no pre-released
    // funds, so the refund is the gross prorated amount for unused time.
    sqlx::query(
        "UPDATE contract_sign_requests SET provisioning_completed_at_ns = $1 WHERE contract_id = $2",
    )
    .bind(start_ns)
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let now_before = crate::now_ns().unwrap();
    // No Stripe client -> issue_audited_refund returns None (no real refund),
    // but the refund_amount_e9s IS persisted for reconciliation.
    let (refund_e9s, refund_id) = db
        .process_dispute_lost_refund(&contract_id, "du_r9", None)
        .await
        .unwrap();

    assert!(refund_id.is_none(), "no Stripe client -> no real refund id");

    let gross = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(start_ns),
        Some(end_ns),
        now_before,
        0,
    );
    let refund = refund_e9s.expect("dispute-lost must record a refund amount");
    assert!(
        (refund - gross).abs() < 1_000_000,
        "dispute-lost refund must equal the gross prorated remainder: expected ~{gross}, got {refund}"
    );
    // Hard money invariant: refund must never exceed the collected payment.
    assert!(
        refund <= payment_amount_e9s,
        "over-refund: refund({refund}) > payment({payment_amount_e9s})"
    );
}

/// A succeeded Stripe payment that carries neither a PaymentIntent id nor a
/// checkout session id is a data-integrity violation. Cancellation must fail
/// loudly rather than silently cancel the contract without refunding.
#[tokio::test]
async fn test_cancel_stripe_contract_without_payment_intent_id_fails_loudly() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![72u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;
    // Stripe + succeeded, but both Stripe ids are NULL.
    sqlx::query(
        "UPDATE contract_sign_requests SET payment_method = 'stripe', payment_status = 'succeeded', stripe_payment_intent_id = NULL, stripe_checkout_session_id = NULL WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let result = db
.cancel_contract(&contract_id, &requester_pk, Some("cancel"), None)
        .await;
    assert!(
        result.is_err(),
        "cancel must fail when a succeeded Stripe payment has no payment id"
    );
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.contains("no payment_intent_id"),
        "unexpected error message: {err}"
    );

    // The contract must remain uncancelled -- no silent state change on failure.
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "requested");
    assert_eq!(contract.payment_status, "succeeded");
}

/// Live integration test (gated on a test-mode `STRIPE_SECRET_KEY`): proves the
/// REAL refund path -- `cancel_contract` -> `StripeClient::create_refund` --
/// issues the correct prorated refund and that Stripe's ledger reflects exactly
/// the amount we recorded. Skipped when no test key is configured, so CI
/// without secrets still passes.
#[tokio::test]
async fn test_cancel_stripe_contract_issues_live_refund() {
    let secret_key = match std::env::var("STRIPE_SECRET_KEY") {
        Ok(k) if k.starts_with("sk_test_") => k,
        Ok(_) => {
            eprintln!("skipping live Stripe test: STRIPE_SECRET_KEY is not a test-mode key");
            return;
        }
        Err(_) => {
            eprintln!("skipping live Stripe test: STRIPE_SECRET_KEY not set");
            return;
        }
    };

    let http = reqwest::Client::new();

    // 1) Create + confirm a real test-mode PaymentIntent so there is a charge to refund.
    let pi: serde_json::Value = http
        .post(format!(
            "{}/v1/payment_intents",
            crate::stripe_client::STRIPE_API_BASE
        ))
        .basic_auth(&secret_key, Some(""))
        .form(&[
            ("amount", "500"),
            ("currency", "usd"),
            ("payment_method", "pm_card_visa"),
            ("payment_method_types[]", "card"),
            ("confirm", "true"),
        ])
        .send()
        .await
        .expect("create PaymentIntent")
        .json()
        .await
        .expect("parse PaymentIntent");
    let pi_id = pi["id"].as_str().expect("payment_intent id").to_string();
    assert_eq!(
        pi["status"].as_str(),
        Some("succeeded"),
        "test PaymentIntent must be succeeded: {pi:#}"
    );

    // 2) Build a contract paid by that PaymentIntent.
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![74u8; 32];
    let payment_amount_e9s = 5_000_000_000i64; // 500 cents == $5.00
    let now_ns = crate::now_ns().unwrap();
    let start_ns = now_ns - (10 * 24 * 3600 * 1_000_000_000i64);
    let end_ns = start_ns + (30 * 24 * 3600 * 1_000_000_000i64);
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "off-1".to_string(),
            payment_intent_id: pi_id.clone(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s,
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;
    // Service started at start_ns; under Stripe-only no funds are pre-released.
    sqlx::query(
        "UPDATE contract_sign_requests SET provisioning_completed_at_ns = $1 WHERE contract_id = $2",
    )
    .bind(start_ns)
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // 3) Cancel through the REAL Stripe client -> real refund on Stripe's test ledger.
    let stripe_client = crate::stripe_client::StripeClient::new().expect("stripe client");
    let now_before = crate::now_ns().unwrap();
    db.cancel_contract(
        &contract_id,
        &requester_pk,
        Some("live refund test"),
        Some(&stripe_client),
    )
    .await
    .expect("cancel_contract with live refund");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    assert_eq!(contract.payment_status, "refunded");
    let refund_e9s = contract.refund_amount_e9s.expect("refund recorded");
    let refund_id = contract.stripe_refund_id.expect("stripe refund id recorded");
    assert!(refund_id.starts_with("re_"), "unexpected refund id: {refund_id}");

    // Refund equals the gross prorated remainder (no released subtraction).
    let gross = Database::calculate_prorated_refund(
        payment_amount_e9s,
        Some(start_ns),
        Some(end_ns),
        now_before,
        0,
    );
    assert!(
        (refund_e9s - gross).abs() < 1_000_000,
        "net refund {refund_e9s} should equal gross {gross}"
    );

    // 4) Stripe's ledger must reflect exactly the cents we recorded.
    let expected_cents = refund_e9s / 10_000_000;
    let refunds: serde_json::Value = http
        .get(format!(
            "{}/v1/refunds?payment_intent={pi_id}&limit=10",
            crate::stripe_client::STRIPE_API_BASE
        ))
        .basic_auth(&secret_key, Some(""))
        .send()
        .await
        .expect("list refunds")
        .json()
        .await
        .expect("parse refunds");
    let data = refunds["data"].as_array().expect("refunds data");
    assert_eq!(
        data.len(),
        1,
        "exactly one refund expected on a fresh PaymentIntent"
    );
    let ledger_cents: i64 = data.iter().map(|r| r["amount"].as_i64().unwrap_or(0)).sum();
    assert_eq!(
        ledger_cents, expected_cents,
        "Stripe ledger refund ({ledger_cents}c) must match recorded amount ({expected_cents}c)"
    );
}

#[tokio::test]
async fn test_try_auto_accept_contract_enabled() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![3u8; 32];

    // Create provider profile with auto_accept_rentals enabled
    sqlx::query!(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals) VALUES ($1, 'Test Provider', 'v1', '1.0', 0, TRUE)",
        provider_pk
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Create contract in 'requested' status with payment_status='succeeded'
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;

    // Try auto-accept
    let result = db.try_auto_accept_contract(&contract_id).await.unwrap();
    assert!(result, "Should return true when contract was auto-accepted");

    // Verify contract status changed to 'accepted'
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "accepted");
}

#[tokio::test]
async fn test_try_auto_accept_contract_disabled() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![3u8; 32];

    // Create provider profile with auto_accept_rentals explicitly disabled
    sqlx::query!(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals) VALUES ($1, 'Test Provider', 'v1', '1.0', 0, FALSE)",
        provider_pk
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Create contract in 'requested' status with payment_status='succeeded'
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;

    // Try auto-accept - should return false since auto_accept_rentals is disabled
    let result = db.try_auto_accept_contract(&contract_id).await.unwrap();
    assert!(!result, "Should return false when auto-accept is disabled");

    // Verify contract status unchanged
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "requested");
}

#[tokio::test]
async fn test_try_auto_accept_contract_idempotent() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![3u8; 32];

    // Create provider profile with auto_accept_rentals enabled
    sqlx::query!(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals) VALUES ($1, 'Test Provider', 'v1', '1.0', 0, TRUE)",
        provider_pk
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Create contract already in 'accepted' status
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "accepted",
    )
    .await;

    // Try auto-accept - should return false since already accepted (idempotent)
    let result = db.try_auto_accept_contract(&contract_id).await.unwrap();
    assert!(
        !result,
        "Should return false when contract already accepted"
    );

    // Verify contract status unchanged
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "accepted");
}

/// Helper: insert a provider profile with auto_accept_rentals=TRUE.
async fn insert_provider_with_auto_accept(db: &Database, provider_pk: &[u8]) {
    sqlx::query!(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals) VALUES ($1, 'Test Provider', 'v1', '1.0', 0, TRUE)",
        provider_pk
    )
    .execute(&db.pool)
    .await
    .unwrap();
}

/// Helper: insert a contract with specified offering_id and duration_hours in "requested" status.
async fn insert_requested_contract_with_duration(
    db: &Database,
    contract_id: &[u8],
    requester_pk: &[u8],
    provider_pk: &[u8],
    offering_id: &str,
    duration_hours: Option<i64>,
) {
    let payment_method = "test";
    let payment_status = "succeeded";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;
    sqlx::query!(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, duration_hours, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency) VALUES ($1, $2, 'ssh-key', 'contact', $3, $4, 1000, $5, 'memo', 0, 'requested', $6, $7, $8, $9, 'usd')",
        contract_id,
        requester_pk,
        provider_pk,
        offering_id,
        duration_hours,
        payment_method,
        stripe_payment_intent_id,
        stripe_customer_id,
        payment_status
    )
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_try_auto_accept_contract_with_matching_rule() {
    let db = setup_test_db().await;
    let provider_pk = vec![85u8; 32];
    let requester_pk = vec![86u8; 32];
    let contract_id = vec![87u8; 32];

    insert_provider_with_auto_accept(&db, &provider_pk).await;
    // Create a rule: accept offering "off-rule" for 24–720 hours
    db.create_auto_accept_rule(&provider_pk, "off-rule", Some(24), Some(720))
        .await
        .unwrap();

    insert_requested_contract_with_duration(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-rule",
        Some(48), // within range
    )
    .await;

    let accepted = db.try_auto_accept_contract(&contract_id).await.unwrap();
    assert!(
        accepted,
        "Contract within rule range should be auto-accepted"
    );

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "accepted");
}

#[tokio::test]
async fn test_try_auto_accept_contract_with_non_matching_rule() {
    let db = setup_test_db().await;
    let provider_pk = vec![88u8; 32];
    let requester_pk = vec![89u8; 32];
    let contract_id = vec![90u8; 32];

    insert_provider_with_auto_accept(&db, &provider_pk).await;
    // Rule: only accept 720–8760 hours
    db.create_auto_accept_rule(&provider_pk, "off-strict", Some(720), Some(8760))
        .await
        .unwrap();

    insert_requested_contract_with_duration(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-strict",
        Some(48), // below min → should NOT auto-accept
    )
    .await;

    let accepted = db.try_auto_accept_contract(&contract_id).await.unwrap();
    assert!(
        !accepted,
        "Contract outside rule range must not be auto-accepted"
    );

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "requested");
}

#[tokio::test]
async fn test_try_auto_accept_contract_no_rule_means_accept_all() {
    let db = setup_test_db().await;
    let provider_pk = vec![91u8; 32];
    let requester_pk = vec![92u8; 32];
    let contract_id = vec![93u8; 32];

    insert_provider_with_auto_accept(&db, &provider_pk).await;
    // No rule for "off-any" → accept all

    insert_requested_contract_with_duration(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-any",
        Some(1), // any duration
    )
    .await;

    let accepted = db.try_auto_accept_contract(&contract_id).await.unwrap();
    assert!(accepted, "No rule for offering means accept all");
}

#[tokio::test]
async fn test_cancel_active_contract_with_prorated_refund() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![50u8; 32];

    // Insert active contract with instance details
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        1_000_000_000, // 1 ICP in e9s
        "active",
    )
    .await;

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    // Use recent start time and future end time for clearer refund calculation
    let start_ns = now_ns - (3600 * 1_000_000_000i64); // Started 1 hour ago
    let end_ns = now_ns + (23 * 3600 * 1_000_000_000i64); // 24 hour contract, 23 hours left

    // Set Stripe payment and instance details
    let instance_details =
        r#"{"external_id":"vm-12345","ip_address":"192.168.1.100","ssh_port":22}"#;
    sqlx::query!(
        "UPDATE contract_sign_requests SET payment_method = $1, payment_status = $2, stripe_payment_intent_id = $3, provisioning_instance_details = $4, provisioning_completed_at_ns = $5, start_timestamp_ns = $6, end_timestamp_ns = $7 WHERE contract_id = $8",
        "stripe",
        "succeeded",
        "pi_test_active",
        instance_details,
        start_ns,
        start_ns,
        end_ns,
        contract_id
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Cancel the active contract
    db.cancel_contract(
        &contract_id,
        &requester_pk,
        Some("User cancelled active rental"),
        None,
    )
    .await
    .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // R5: no Stripe client -> refund NOT issued, payment_status stays 'succeeded'.
    assert_eq!(contract.payment_status, "succeeded");
    assert!(contract.refund_amount_e9s.is_some());
    // Prorated refund should be present (23/24 hours remaining = ~96% of $1)
    let refund = contract.refund_amount_e9s.unwrap();
    assert!(refund > 0, "Should have a refund amount");
    assert!(
        refund < 1_000_000_000,
        "Refund should be less than full amount"
    );
}

#[tokio::test]
async fn test_get_pending_termination_contracts() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];

    // Create a cancelled contract WITH instance details (should be returned)
    let contract_id_1 = vec![60u8; 32];
    insert_contract_request(
        &db,
        &contract_id_1,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "cancelled",
    )
    .await;

    let instance_details_1 = r#"{"external_id":"vm-001","ip_address":"10.0.0.1","ssh_port":22}"#;
    sqlx::query!(
        "UPDATE contract_sign_requests SET provisioning_instance_details = $1 WHERE contract_id = $2",
        instance_details_1,
        contract_id_1
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Create a cancelled contract WITHOUT instance details (should NOT be returned)
    let contract_id_2 = vec![61u8; 32];
    insert_contract_request(
        &db,
        &contract_id_2,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "cancelled",
    )
    .await;

    // Create an active contract WITH instance details (should NOT be returned - not cancelled)
    let contract_id_3 = vec![62u8; 32];
    insert_contract_request(
        &db,
        &contract_id_3,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "active",
    )
    .await;

    let instance_details_3 = r#"{"external_id":"vm-003","ip_address":"10.0.0.3","ssh_port":22}"#;
    sqlx::query!(
        "UPDATE contract_sign_requests SET provisioning_instance_details = $1 WHERE contract_id = $2",
        instance_details_3,
        contract_id_3
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Get pending terminations
    let pending = db
        .get_pending_termination_contracts(&provider_pk)
        .await
        .unwrap();

    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].contract_id, hex::encode(&contract_id_1));
    assert_eq!(pending[0].instance_details, instance_details_1);
}

#[tokio::test]
async fn test_mark_contract_terminated() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![70u8; 32];

    // Create cancelled contract with instance details
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "cancelled",
    )
    .await;

    let instance_details =
        r#"{"external_id":"vm-to-terminate","ip_address":"10.0.0.5","ssh_port":22}"#;
    sqlx::query!(
        "UPDATE contract_sign_requests SET provisioning_instance_details = $1 WHERE contract_id = $2",
        instance_details,
        contract_id
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Verify it appears in pending terminations
    let pending = db
        .get_pending_termination_contracts(&provider_pk)
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);

    // Mark as terminated
    db.mark_contract_terminated(&contract_id).await.unwrap();

    // Verify it no longer appears in pending terminations
    let pending = db
        .get_pending_termination_contracts(&provider_pk)
        .await
        .unwrap();
    assert_eq!(pending.len(), 0);

    // Verify terminated_at_ns is set
    let contract_id_param = contract_id.clone();
    let terminated_at: Option<i64> = sqlx::query_scalar!(
        r#"SELECT terminated_at_ns FROM contract_sign_requests WHERE contract_id = $1"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(terminated_at.is_some());
}

#[tokio::test]
async fn test_mark_contract_terminated_not_cancelled() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![71u8; 32];

    // Create active contract (not cancelled)
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "active",
    )
    .await;

    // Attempt to mark as terminated should fail
    let result = db.mark_contract_terminated(&contract_id).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not in cancelled status"));
}

// Tests for reconciliation support - verifying get_contract returns data needed for expiry checks

#[tokio::test]
async fn test_get_contract_returns_end_timestamp_for_active() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![80u8; 32];

    let now = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let future = now + 3_600_000_000_000; // 1 hour in future

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "offering-1".to_string(),
            payment_intent_id: "pi_test".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1000,
            start_timestamp_ns: now,
            end_timestamp_ns: future,
        },
    )
    .await;

    // Update status to provisioned (active)
    sqlx::query("UPDATE contract_sign_requests SET status = 'provisioned' WHERE contract_id = $1")
        .bind(&contract_id)
        .execute(&db.pool)
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    assert_eq!(contract.status, "provisioned");
    assert!(contract.end_timestamp_ns.is_some());
    assert_eq!(contract.end_timestamp_ns.unwrap(), future);
    // Verify contract is NOT expired (end_timestamp_ns is in future)
    assert!(contract.end_timestamp_ns.unwrap() > now);
}

#[tokio::test]
async fn test_get_contract_returns_end_timestamp_for_expired() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![81u8; 32];

    let now = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let past = now - 3_600_000_000_000; // 1 hour ago

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "offering-1".to_string(),
            payment_intent_id: "pi_test2".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1000,
            start_timestamp_ns: past - 7_200_000_000_000, // 2 hours before end
            end_timestamp_ns: past,
        },
    )
    .await;

    // Update status to provisioned (was running)
    sqlx::query("UPDATE contract_sign_requests SET status = 'provisioned' WHERE contract_id = $1")
        .bind(&contract_id)
        .execute(&db.pool)
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    assert_eq!(contract.status, "provisioned");
    assert!(contract.end_timestamp_ns.is_some());
    // Verify contract IS expired (end_timestamp_ns is in past)
    assert!(contract.end_timestamp_ns.unwrap() < now);
}

#[tokio::test]
async fn test_get_contract_returns_cancelled_status() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![82u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "cancelled",
    )
    .await;

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    assert_eq!(contract.status, "cancelled");
}

#[tokio::test]
async fn test_get_contract_returns_provider_pubkey() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];
    let requester_pk = vec![1u8; 32];
    let contract_id = vec![83u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    // Verify provider_pubkey matches (for authorization checks in reconcile)
    assert_eq!(hex::decode(&contract.provider_pubkey).unwrap(), provider_pk);
}

#[tokio::test]
async fn test_get_contract_not_found() {
    let db = setup_test_db().await;
    let non_existent_id = vec![99u8; 32];

    let contract = db.get_contract(&non_existent_id).await.unwrap();

    assert!(contract.is_none());
}

#[tokio::test]
async fn test_provisioning_lock_race_condition() {
    let db = setup_test_db().await;
    let provider_pk = vec![1u8; 32];
    let requester_pk = vec![2u8; 32];
    let contract_id = vec![3u8; 32];

    // 1. Create a contract ready for provisioning
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-race",
        0,
        "accepted",
    )
    .await;

    // 2. Create two agents
    let agent1_pk = vec![101u8; 32];
    let agent2_pk = vec![102u8; 32];
    let lock_duration_ns = 5 * 60 * 1_000_000_000; // 5 minutes

    // 3. Simulate race condition
    let db_clone1 = db.clone();
    let db_clone2 = db.clone();
    let contract_id_clone1 = contract_id.clone();
    let contract_id_clone2 = contract_id.clone();
    let agent1_pk_clone = agent1_pk.clone();
    let agent2_pk_clone = agent2_pk.clone();

    let task1: tokio::task::JoinHandle<Result<bool>> = tokio::spawn(async move {
        db_clone1
            .acquire_provisioning_lock(&contract_id_clone1, &agent1_pk_clone, lock_duration_ns)
            .await
    });
    let task2: tokio::task::JoinHandle<Result<bool>> = tokio::spawn(async move {
        db_clone2
            .acquire_provisioning_lock(&contract_id_clone2, &agent2_pk_clone, lock_duration_ns)
            .await
    });

    let (result1, result2) = tokio::join!(task1, task2);
    let result1 = result1.unwrap().unwrap();
    let result2 = result2.unwrap().unwrap();

    // 4. Assert that only one agent got the lock
    assert_ne!(result1, result2, "One agent must win, the other must lose");
    assert!(
        result1 || result2,
        "At least one agent must acquire the lock"
    );

    // 5. Verify lock state in DB
    let winner = if result1 { &agent1_pk } else { &agent2_pk };
    let c: (Option<Vec<u8>>,) = sqlx::query_as(
        "SELECT provisioning_lock_agent FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(c.0.as_deref(), Some(winner.as_slice()));

    // 6. Test that the loser cannot acquire the lock now
    let loser_pk = if result1 { &agent2_pk } else { &agent1_pk };
    let loser_can_lock = db
        .acquire_provisioning_lock(&contract_id, loser_pk, lock_duration_ns)
        .await
        .unwrap();
    assert!(
        !loser_can_lock,
        "Loser should not be able to acquire the lock while it's held"
    );

    // 7. Test that winner can re-acquire (idempotency)
    let winner_can_relock = db
        .acquire_provisioning_lock(&contract_id, winner, lock_duration_ns)
        .await
        .unwrap();
    assert!(
        winner_can_relock,
        "Winner should be able to re-acquire their own lock"
    );

    // 8. Test that winner can release the lock
    let released = db
        .release_provisioning_lock(&contract_id, winner)
        .await
        .unwrap();
    assert!(released, "Winner should be able to release the lock");

    // 9. Verify lock is released in DB
    let c: (Option<Vec<u8>>,) = sqlx::query_as(
        "SELECT provisioning_lock_agent FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(c.0.is_none(), "Lock should be released in the database");

    // 10. Test that the loser can now acquire the lock
    let loser_can_lock_now = db
        .acquire_provisioning_lock(&contract_id, loser_pk, lock_duration_ns)
        .await
        .unwrap();
    assert!(
        loser_can_lock_now,
        "Loser should be able to acquire the lock after it was released"
    );
}

#[tokio::test]
async fn test_provisioning_lock_expiration() {
    let db = setup_test_db().await;
    let provider_pk = vec![1u8; 32];
    let requester_pk = vec![2u8; 32];
    let contract_id = vec![4u8; 32];

    // Create a contract ready for provisioning
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-expire",
        0,
        "accepted",
    )
    .await;

    let agent1_pk = vec![101u8; 32];
    let agent2_pk = vec![102u8; 32];

    // Agent 1 acquires lock with very short duration (1 nanosecond - effectively expired immediately)
    let lock_duration_ns = 1i64;
    let result1 = db
        .acquire_provisioning_lock(&contract_id, &agent1_pk, lock_duration_ns)
        .await
        .unwrap();
    assert!(result1, "Agent 1 should acquire the lock");

    // Simulate time passing - manually set expires_ns to past
    let past_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)")
        - 1_000_000_000;
    sqlx::query(
        "UPDATE contract_sign_requests SET provisioning_lock_expires_ns = $1 WHERE contract_id = $2",
    )
    .bind(past_ns)
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Agent 2 should be able to acquire the expired lock
    let lock_duration_ns = 5 * 60 * 1_000_000_000i64;
    let result2 = db
        .acquire_provisioning_lock(&contract_id, &agent2_pk, lock_duration_ns)
        .await
        .unwrap();
    assert!(
        result2,
        "Agent 2 should acquire the lock since Agent 1's lock expired"
    );

    // Verify agent 2 now holds the lock
    let c: (Option<Vec<u8>>,) = sqlx::query_as(
        "SELECT provisioning_lock_agent FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(c.0.as_deref(), Some(agent2_pk.as_slice()));
}

#[tokio::test]
async fn test_cleanup_expired_provisioning_locks() {
    let db = setup_test_db().await;
    let provider_pk = vec![1u8; 32];
    let requester_pk = vec![2u8; 32];
    let agent_pk = vec![101u8; 32];

    // Create two contracts with locks
    let contract_id_1 = vec![10u8; 32];
    let contract_id_2 = vec![11u8; 32];

    insert_contract_request(
        &db,
        &contract_id_1,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "accepted",
    )
    .await;
    insert_contract_request(
        &db,
        &contract_id_2,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "accepted",
    )
    .await;

    // Acquire locks on both
    let lock_duration_ns = 5 * 60 * 1_000_000_000i64;
    db.acquire_provisioning_lock(&contract_id_1, &agent_pk, lock_duration_ns)
        .await
        .unwrap();
    db.acquire_provisioning_lock(&contract_id_2, &agent_pk, lock_duration_ns)
        .await
        .unwrap();

    // Set contract_id_1's lock to expired (in the past)
    let past_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)")
        - 1_000_000_000;
    sqlx::query(
        "UPDATE contract_sign_requests SET provisioning_lock_expires_ns = $1 WHERE contract_id = $2",
    )
    .bind(past_ns)
    .bind(&contract_id_1)
    .execute(&db.pool)
    .await
    .unwrap();

    // Run cleanup
    let cleaned = db.clear_expired_provisioning_locks().await.unwrap();
    assert_eq!(cleaned, 1, "Should clean up exactly 1 expired lock");

    // Verify contract_id_1's lock is cleared
    let c1: (Option<Vec<u8>>,) = sqlx::query_as(
        "SELECT provisioning_lock_agent FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id_1)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(c1.0.is_none(), "Expired lock should be cleared");

    // Verify contract_id_2's lock is still held
    let c2: (Option<Vec<u8>>,) = sqlx::query_as(
        "SELECT provisioning_lock_agent FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id_2)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(c2.0.is_some(), "Non-expired lock should still be held");
}

// === Contract Usage Tracking Tests ===

// === Contract Health Check Tests ===

#[tokio::test]
async fn test_record_health_check_success() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![3u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let checked_at = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let check_id = db
        .record_health_check(
            &contract_id,
            checked_at,
            "healthy",
            Some(42),
            Some(r#"{"port":22}"#),
        )
        .await
        .unwrap();

    assert!(check_id > 0, "Should return positive check ID");
}

#[tokio::test]
async fn test_record_health_check_all_status_values() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![4u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let now = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");

    // Test all valid status values
    for status in ["healthy", "unhealthy", "unknown"] {
        let check_id = db
            .record_health_check(&contract_id, now, status, None, None)
            .await
            .unwrap();
        assert!(check_id > 0, "Should record '{}' status", status);
    }
}

#[tokio::test]
async fn test_record_health_check_invalid_status() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![5u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let now = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let result = db
        .record_health_check(&contract_id, now, "invalid_status", None, None)
        .await;

    assert!(result.is_err(), "Should reject invalid status");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Invalid health status"),
        "Error should mention invalid status: {}",
        err
    );
}

#[tokio::test]
async fn test_get_recent_health_checks_ordered_by_checked_at() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![6u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    // Insert checks at different times
    let base_ns = 1000000000000000000_i64; // 1 second in nanoseconds
    db.record_health_check(&contract_id, base_ns, "healthy", Some(10), None)
        .await
        .unwrap();
    db.record_health_check(
        &contract_id,
        base_ns + 60_000_000_000,
        "unhealthy",
        Some(500),
        None,
    )
    .await
    .unwrap();
    db.record_health_check(
        &contract_id,
        base_ns + 120_000_000_000,
        "healthy",
        Some(15),
        None,
    )
    .await
    .unwrap();

    let checks = db.get_recent_health_checks(&contract_id, 10).await.unwrap();

    assert_eq!(checks.len(), 3, "Should return all 3 health checks");
    // Should be ordered by checked_at DESC (most recent first)
    assert_eq!(checks[0].checked_at, base_ns + 120_000_000_000);
    assert_eq!(checks[1].checked_at, base_ns + 60_000_000_000);
    assert_eq!(checks[2].checked_at, base_ns);
    assert_eq!(checks[0].status, "healthy");
    assert_eq!(checks[1].status, "unhealthy");
    assert_eq!(checks[2].status, "healthy");
}

#[tokio::test]
async fn test_get_recent_health_checks_respects_limit() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![7u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    // Insert 5 checks
    let base_ns = 1000000000000000000_i64;
    for i in 0..5 {
        db.record_health_check(
            &contract_id,
            base_ns + i * 60_000_000_000,
            "healthy",
            None,
            None,
        )
        .await
        .unwrap();
    }

    let checks = db.get_recent_health_checks(&contract_id, 2).await.unwrap();

    assert_eq!(checks.len(), 2, "Should respect limit of 2");
    // Should return the 2 most recent
    assert_eq!(checks[0].checked_at, base_ns + 4 * 60_000_000_000);
    assert_eq!(checks[1].checked_at, base_ns + 3 * 60_000_000_000);
}

#[tokio::test]
async fn test_get_recent_health_checks_empty() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![8u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let checks = db.get_recent_health_checks(&contract_id, 10).await.unwrap();

    assert_eq!(
        checks.len(),
        0,
        "Should return empty vec for contract with no health checks"
    );
}

#[tokio::test]
async fn test_record_health_check_with_details_json() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![9u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let details = r#"{"ssh_status":"ok","http_status":200,"memory_mb":1024}"#;
    let now = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");

    db.record_health_check(&contract_id, now, "healthy", Some(25), Some(details))
        .await
        .unwrap();

    let checks = db.get_recent_health_checks(&contract_id, 1).await.unwrap();

    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].details, Some(details.to_string()));
    assert_eq!(checks[0].latency_ms, Some(25));
}

// === Provider Health Summary Tests ===

#[tokio::test]
async fn test_get_provider_health_summary_no_checks() {
    let db = setup_test_db().await;
    let provider_pk = vec![10u8; 32];

    let summary = db
        .get_provider_health_summary(&provider_pk, Some(30))
        .await
        .unwrap();

    assert_eq!(summary.total_checks, 0);
    assert_eq!(summary.healthy_checks, 0);
    assert_eq!(summary.unhealthy_checks, 0);
    assert_eq!(summary.unknown_checks, 0);
    assert_eq!(summary.uptime_percent, 0.0);
    assert!(summary.avg_latency_ms.is_none());
    assert_eq!(summary.contracts_monitored, 0);
}

#[tokio::test]
async fn test_get_provider_health_summary_with_checks() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![11u8; 32];
    let contract_id = vec![12u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    // Insert health checks (recent enough to be in the 30 day window)
    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let one_hour_ns = 60 * 60 * 1_000_000_000_i64;

    // 8 healthy, 2 unhealthy = 80% uptime
    for i in 0..8 {
        db.record_health_check(
            &contract_id,
            now_ns - i * one_hour_ns,
            "healthy",
            Some(20 + i as i32),
            None,
        )
        .await
        .unwrap();
    }
    for i in 0..2 {
        db.record_health_check(
            &contract_id,
            now_ns - (8 + i) * one_hour_ns,
            "unhealthy",
            Some(100),
            None,
        )
        .await
        .unwrap();
    }

    let summary = db
        .get_provider_health_summary(&provider_pk, Some(30))
        .await
        .unwrap();

    assert_eq!(summary.total_checks, 10);
    assert_eq!(summary.healthy_checks, 8);
    assert_eq!(summary.unhealthy_checks, 2);
    assert_eq!(summary.unknown_checks, 0);
    assert!((summary.uptime_percent - 80.0).abs() < 0.01);
    assert!(summary.avg_latency_ms.is_some());
    assert_eq!(summary.contracts_monitored, 1);
}

#[tokio::test]
async fn test_get_provider_health_summary_multiple_contracts() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![13u8; 32];
    let contract_id_1 = vec![14u8; 32];
    let contract_id_2 = vec![15u8; 32];

    insert_contract_request(
        &db,
        &contract_id_1,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;
    insert_contract_request(
        &db,
        &contract_id_2,
        &user_pk,
        &provider_pk,
        "off-2",
        0,
        "provisioned",
    )
    .await;

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");

    // Contract 1: 3 healthy
    for i in 0..3 {
        db.record_health_check(
            &contract_id_1,
            now_ns - i * 60_000_000_000,
            "healthy",
            Some(10),
            None,
        )
        .await
        .unwrap();
    }

    // Contract 2: 2 healthy, 1 unhealthy
    for i in 0..2 {
        db.record_health_check(
            &contract_id_2,
            now_ns - i * 60_000_000_000,
            "healthy",
            Some(15),
            None,
        )
        .await
        .unwrap();
    }
    db.record_health_check(
        &contract_id_2,
        now_ns - 3 * 60_000_000_000,
        "unhealthy",
        None,
        None,
    )
    .await
    .unwrap();

    let summary = db
        .get_provider_health_summary(&provider_pk, Some(30))
        .await
        .unwrap();

    assert_eq!(summary.total_checks, 6);
    assert_eq!(summary.healthy_checks, 5);
    assert_eq!(summary.unhealthy_checks, 1);
    // 5/6 = 83.33%
    assert!((summary.uptime_percent - 83.33).abs() < 0.1);
    assert_eq!(summary.contracts_monitored, 2);
}

#[tokio::test]
async fn test_get_provider_health_summary_respects_time_window() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![16u8; 32];
    let contract_id = vec![17u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let one_day_ns = 24 * 60 * 60 * 1_000_000_000_i64;

    // Recent check (within 7 days)
    db.record_health_check(&contract_id, now_ns - one_day_ns, "healthy", None, None)
        .await
        .unwrap();

    // Old check (more than 7 days ago)
    db.record_health_check(
        &contract_id,
        now_ns - 10 * one_day_ns,
        "unhealthy",
        None,
        None,
    )
    .await
    .unwrap();

    // Query with 7 day window
    let summary = db
        .get_provider_health_summary(&provider_pk, Some(7))
        .await
        .unwrap();

    // Should only include the recent healthy check
    assert_eq!(summary.total_checks, 1);
    assert_eq!(summary.healthy_checks, 1);
    assert_eq!(summary.unhealthy_checks, 0);
    assert_eq!(summary.uptime_percent, 100.0);

    // Query with 30 day window (should include both)
    let summary_30 = db
        .get_provider_health_summary(&provider_pk, Some(30))
        .await
        .unwrap();

    assert_eq!(summary_30.total_checks, 2);
    assert_eq!(summary_30.healthy_checks, 1);
    assert_eq!(summary_30.unhealthy_checks, 1);
    assert_eq!(summary_30.uptime_percent, 50.0);
}

#[tokio::test]
async fn test_get_provider_health_summary_default_30_days() {
    let db = setup_test_db().await;
    let provider_pk = vec![18u8; 32];

    // No explicit days parameter (should default to 30)
    let summary = db
        .get_provider_health_summary(&provider_pk, None)
        .await
        .unwrap();

    // Should work with no data
    assert_eq!(summary.total_checks, 0);
    // Verify the period is approximately 30 days
    let expected_period_ns = 30 * 24 * 60 * 60 * 1_000_000_000_i64;
    let actual_period_ns = summary.period_end_ns - summary.period_start_ns;
    // Allow 1 second tolerance for test execution time
    assert!((actual_period_ns - expected_period_ns).abs() < 1_000_000_000);
}

#[tokio::test]
async fn test_get_provider_health_summary_all_status_types() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![19u8; 32];
    let contract_id = vec![20u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned",
    )
    .await;

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");

    // Add one of each status
    db.record_health_check(&contract_id, now_ns, "healthy", None, None)
        .await
        .unwrap();
    db.record_health_check(
        &contract_id,
        now_ns - 60_000_000_000,
        "unhealthy",
        None,
        None,
    )
    .await
    .unwrap();
    db.record_health_check(
        &contract_id,
        now_ns - 120_000_000_000,
        "unknown",
        None,
        None,
    )
    .await
    .unwrap();

    let summary = db
        .get_provider_health_summary(&provider_pk, Some(30))
        .await
        .unwrap();

    assert_eq!(summary.total_checks, 3);
    assert_eq!(summary.healthy_checks, 1);
    assert_eq!(summary.unhealthy_checks, 1);
    assert_eq!(summary.unknown_checks, 1);
    // Only healthy counts toward uptime: 1/3 = 33.33%
    assert!((summary.uptime_percent - 33.33).abs() < 0.1);
}

// === Subscription Management Tests ===

// === Cloud Resource Provisioning Bridge Tests ===

#[tokio::test]
async fn test_update_contract_provisioned_by_cloud_resource_sets_gateway_fields() {
    let db = setup_test_db().await;

    let contract_id = vec![0xF1u8; 32];
    let requester = [0xF2u8; 32];
    let provider = [0xF3u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester,
        &provider,
        "test-offering",
        1000,
        "accepted",
    )
    .await;

    let instance_details = r#"{"public_ip":"1.2.3.4","ssh_port":22}"#;

    db.update_contract_provisioned_by_cloud_resource(
        &contract_id,
        instance_details,
        Some("abc123"),
        Some("abc123.hz-nbg1.dev-gw.decent-cloud.org"),
        Some(22),
    )
    .await
    .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "active");
    assert_eq!(
        contract.provisioning_instance_details.as_deref(),
        Some(instance_details)
    );
    assert!(contract.provisioning_completed_at_ns.is_some());

    // Verify gateway fields were set on the contract row
    let row: (Option<String>, Option<String>, Option<i32>) = sqlx::query_as(
        "SELECT gateway_slug, gateway_subdomain, gateway_ssh_port FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(row.0.as_deref(), Some("abc123"));
    assert_eq!(
        row.1.as_deref(),
        Some("abc123.hz-nbg1.dev-gw.decent-cloud.org")
    );
    assert_eq!(row.2, Some(22));

    let provisioning_row: (Option<String>,) = sqlx::query_as(
        "SELECT connection_instructions FROM contract_provisioning_details WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(provisioning_row.0.as_deref(), Some(instance_details));
}

#[tokio::test]
async fn test_create_rental_request_reserves_self_provisioned_resource() {
    let db = setup_test_db().await;
    let requester = vec![1u8; 32];
    let provider = vec![2u8; 32];

    let account = db
        .create_account("selfprov_provider", &provider, "selfprov@example.com")
        .await
        .unwrap();
    let cloud_account = db
        .create_cloud_account(
            &account.id,
            crate::cloud::types::BackendType::Hetzner,
            "selfprov-hetzner",
            "encrypted",
            None,
        )
        .await
        .unwrap();
    let cloud_account_id: uuid::Uuid = cloud_account.id.parse().unwrap();

    let resource = db
        .create_cloud_resource(
            &cloud_account_id,
            "selfprov-ext",
            "selfprov-vm",
            "cx22",
            "nbg1",
            "ubuntu-24.04",
            "ssh-ed25519 AAAA owner",
        )
        .await
        .unwrap();
    let resource_id: uuid::Uuid = resource.id.parse().unwrap();
    db.update_cloud_resource_status(&resource_id, "running")
        .await
        .unwrap();

    let offering_id: i64 = sqlx::query_scalar(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, offering_source, created_at_ns) VALUES ($1, 'off-self-prov', 'Self Prov', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 'self_provisioned', 0) RETURNING id",
    )
    .bind(&provider)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    db.list_on_marketplace(&resource_id, &account.id, offering_id)
        .await
        .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-ed25519 AAAA tenant".to_string()),
        contact_method: Some("email:tenant@example.com".to_string()),
        request_memo: Some("Rent self-provisioned VM".to_string()),
        duration_hours: Some(24),
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: Some("Ubuntu 24.04".to_string()),
    };

    let contract_id = db.create_rental_request(&requester, params).await.unwrap();

    let stock_status: (String,) =
        sqlx::query_as("SELECT stock_status FROM provider_offerings WHERE id = $1")
            .bind(offering_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(stock_status.0, "out_of_stock");

    let linked_contract: (Option<Vec<u8>>,) =
        sqlx::query_as("SELECT contract_id FROM cloud_resources WHERE id = $1")
            .bind(resource_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(linked_contract.0, Some(contract_id));
}

#[tokio::test]
async fn test_try_activate_self_provisioned_contract_promotes_reserved_contract_to_active() {
    let db = setup_test_db().await;
    let requester = vec![3u8; 32];
    let provider = vec![4u8; 32];
    let contract_id = vec![0x90u8; 32];

    let account = db
        .create_account("activate_provider", &provider, "activate@example.com")
        .await
        .unwrap();
    let cloud_account = db
        .create_cloud_account(
            &account.id,
            crate::cloud::types::BackendType::Hetzner,
            "activate-hetzner",
            "encrypted",
            None,
        )
        .await
        .unwrap();
    let cloud_account_id: uuid::Uuid = cloud_account.id.parse().unwrap();

    let resource = db
        .create_cloud_resource(
            &cloud_account_id,
            "activate-ext",
            "activate-vm",
            "cx22",
            "nbg1",
            "ubuntu-24.04",
            "ssh-ed25519 AAAA owner",
        )
        .await
        .unwrap();
    let resource_id: uuid::Uuid = resource.id.parse().unwrap();
    db.update_cloud_resource_provisioned(
        &resource_id,
        "activate-ext",
        "203.0.113.10",
        "ssh-key-id",
        "gwslug",
        Some("gwslug.dc-lk.dev-gw.decent-cloud.org"),
        2201,
        2201,
        2210,
    )
    .await
    .unwrap();

    let offering_id: i64 = sqlx::query_scalar(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, offering_source, created_at_ns) VALUES ($1, 'off-activate-self', 'Self Prov', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'out_of_stock', 'US', 'NYC', FALSE, 'self_provisioned', 0) RETURNING id",
    )
    .bind(&provider)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    db.list_on_marketplace(&resource_id, &account.id, offering_id)
        .await
        .unwrap();

    insert_contract_request(
        &db,
        &contract_id,
        &requester,
        &provider,
        &offering_id.to_string(),
        0,
        "accepted",
    )
    .await;

    sqlx::query("UPDATE cloud_resources SET contract_id = $1 WHERE id = $2")
        .bind(&contract_id)
        .bind(resource_id)
        .execute(&db.pool)
        .await
        .unwrap();

    assert!(db
        .try_activate_self_provisioned_contract(&contract_id)
        .await
        .unwrap());

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "active");
    assert!(contract
        .provisioning_instance_details
        .as_deref()
        .unwrap()
        .contains("203.0.113.10"));

    let provisioning_row: (Option<String>,) = sqlx::query_as(
        "SELECT connection_instructions FROM contract_provisioning_details WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(provisioning_row
        .0
        .as_deref()
        .unwrap()
        .contains("gwslug.dc-lk.dev-gw.decent-cloud.org"));
}

#[tokio::test]
async fn test_cancel_contract_releases_self_provisioned_resource_not_deletes() {
    let db = setup_test_db().await;
    let requester = vec![5u8; 32];
    let provider = vec![6u8; 32];

    let account = db
        .create_account("cancel_sp_provider", &provider, "cancel-sp@example.com")
        .await
        .unwrap();
    let cloud_account = db
        .create_cloud_account(
            &account.id,
            crate::cloud::types::BackendType::Hetzner,
            "cancel-sp-hetzner",
            "encrypted",
            None,
        )
        .await
        .unwrap();
    let cloud_account_id: uuid::Uuid = cloud_account.id.parse().unwrap();

    let resource = db
        .create_cloud_resource(
            &cloud_account_id,
            "cancel-sp-ext",
            "cancel-sp-vm",
            "cx22",
            "nbg1",
            "ubuntu-24.04",
            "ssh-ed25519 AAAA owner",
        )
        .await
        .unwrap();
    let resource_id: uuid::Uuid = resource.id.parse().unwrap();
    db.update_cloud_resource_status(&resource_id, "running")
        .await
        .unwrap();

    let offering_id: i64 = sqlx::query_scalar(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, offering_source, created_at_ns) VALUES ($1, 'off-cancel-sp', 'Self Prov Cancel', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 'self_provisioned', 0) RETURNING id",
    )
    .bind(&provider)
    .fetch_one(&db.pool)
    .await
    .unwrap();

    db.list_on_marketplace(&resource_id, &account.id, offering_id)
        .await
        .unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-ed25519 AAAA tenant".to_string()),
        contact_method: Some("email:tenant@example.com".to_string()),
        request_memo: Some("Rent self-provisioned VM".to_string()),
        duration_hours: Some(24),
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: Some("Ubuntu 24.04".to_string()),
    };

    let contract_id = db.create_rental_request(&requester, params).await.unwrap();

    let stock_after_reserve: (String,) =
        sqlx::query_as("SELECT stock_status FROM provider_offerings WHERE id = $1")
            .bind(offering_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(stock_after_reserve.0, "out_of_stock");

    db.cancel_contract(
        &contract_id,
        &requester,
        Some("Tenant cancelled"),
        None,
    )
    .await
    .unwrap();

    let contract_status: (String,) =
        sqlx::query_as("SELECT status FROM contract_sign_requests WHERE contract_id = $1")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(contract_status.0, "cancelled");

    let resource_state: (Option<Vec<u8>>, String) =
        sqlx::query_as("SELECT contract_id, status FROM cloud_resources WHERE id = $1")
            .bind(resource_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        resource_state.0, None,
        "self-provisioned resource should be released (contract_id NULL), not deleted"
    );
    assert_eq!(
        resource_state.1, "running",
        "self-provisioned resource status should remain running, not set to deleting"
    );

    let stock_after_cancel: (String,) =
        sqlx::query_as("SELECT stock_status FROM provider_offerings WHERE id = $1")
            .bind(offering_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        stock_after_cancel.0, "in_stock",
        "offering should be restocked after cancel"
    );
}

// --- purge_terminal_contracts tests ---

/// Helper to insert a terminal contract with a specific status_updated_at_ns
async fn insert_terminal_contract(
    db: &Database,
    contract_id: &[u8],
    status: &str,
    status_updated_at_ns: i64,
) {
    let requester = vec![10u8; 32];
    let provider = vec![11u8; 32];
    insert_contract_request(db, contract_id, &requester, &provider, "off-1", 0, status).await;
    sqlx::query(
        "UPDATE contract_sign_requests SET status_updated_at_ns = $1 WHERE contract_id = $2",
    )
    .bind(status_updated_at_ns)
    .bind(contract_id)
    .execute(&db.pool)
    .await
    .unwrap();
}

fn old_timestamp_ns(days_ago: i64) -> i64 {
    chrono::Utc::now().timestamp_nanos_opt().unwrap() - (days_ago * 24 * 60 * 60 * 1_000_000_000)
}

#[tokio::test]
async fn test_purge_terminal_contracts_deletes_old_expired() {
    let db = setup_test_db().await;
    let contract_id = vec![50u8; 32];

    // 200-day-old expired contract
    insert_terminal_contract(&db, &contract_id, "expired", old_timestamp_ns(200)).await;

    let purged = db.purge_terminal_contracts(180).await.unwrap();
    assert_eq!(purged, 1);

    // Verify contract is gone
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM contract_sign_requests WHERE contract_id = $1")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 0);
}

#[tokio::test]
async fn test_purge_terminal_contracts_preserves_recent() {
    let db = setup_test_db().await;
    let old_id = vec![51u8; 32];
    let recent_id = vec![52u8; 32];

    insert_terminal_contract(&db, &old_id, "cancelled", old_timestamp_ns(200)).await;
    insert_terminal_contract(&db, &recent_id, "cancelled", old_timestamp_ns(10)).await;

    let purged = db.purge_terminal_contracts(180).await.unwrap();
    assert_eq!(purged, 1);

    // Recent contract survives
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM contract_sign_requests WHERE contract_id = $1")
            .bind(&recent_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(count.0, 1);
}

#[tokio::test]
async fn test_purge_terminal_contracts_preserves_active() {
    let db = setup_test_db().await;
    let active_id = vec![53u8; 32];

    // Active contract with old timestamp should NOT be purged
    insert_terminal_contract(&db, &active_id, "active", old_timestamp_ns(200)).await;

    let purged = db.purge_terminal_contracts(180).await.unwrap();
    assert_eq!(purged, 0);
}

#[tokio::test]
async fn test_purge_terminal_contracts_skips_unreported_usage() {
    let db = setup_test_db().await;
    let contract_id = vec![54u8; 32];

    insert_terminal_contract(&db, &contract_id, "expired", old_timestamp_ns(200)).await;

    // Add unreported usage
    sqlx::query(
        "INSERT INTO contract_usage (contract_id, billing_period_start, billing_period_end, reported_to_stripe) VALUES ($1, 0, 100, FALSE)",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let purged = db.purge_terminal_contracts(180).await.unwrap();
    assert_eq!(purged, 0, "should skip contracts with unreported usage");
}

#[tokio::test]
async fn test_purge_terminal_contracts_cleans_related_tables() {
    let db = setup_test_db().await;
    let contract_id = vec![55u8; 32];
    let hex_id = hex::encode(&contract_id);

    insert_terminal_contract(&db, &contract_id, "rejected", old_timestamp_ns(200)).await;

    // Insert data in non-cascading tables
    sqlx::query(
        "INSERT INTO contract_usage (contract_id, billing_period_start, billing_period_end, reported_to_stripe) VALUES ($1, 0, 100, TRUE)",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO contract_usage_events (contract_id, event_type) VALUES ($1, 'heartbeat')",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO contract_health_checks (contract_id, checked_at, status) VALUES ($1, 0, 'healthy')",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    sqlx::query("INSERT INTO escrow (contract_id, payer_pubkey, payee_pubkey, amount_e9s, status, created_at_ns) VALUES ($1, $2, $3, 100, 'held', 0)")
        .bind(&contract_id)
        .bind(vec![10u8; 32])
        .bind(vec![11u8; 32])
        .execute(&db.pool)
        .await
        .unwrap();

    sqlx::query(
        "INSERT INTO bandwidth_history (contract_id, gateway_slug, provider_pubkey, recorded_at_ns) VALUES ($1, 'slug', 'provider', 0)",
    )
    .bind(&hex_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // Also insert cascading data to verify it's cleaned too
    sqlx::query(
        "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns) VALUES ($1, 'active', 'rejected', $2, 0)",
    )
    .bind(&contract_id)
    .bind(vec![10u8; 32])
    .execute(&db.pool)
    .await
    .unwrap();

    let purged = db.purge_terminal_contracts(180).await.unwrap();
    assert_eq!(purged, 1);

    // Verify all related data is gone
    let usage_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM contract_usage WHERE contract_id = $1")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(usage_count.0, 0, "contract_usage should be purged");

    let events_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM contract_usage_events WHERE contract_id = $1")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(events_count.0, 0, "contract_usage_events should be purged");

    let health_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM contract_health_checks WHERE contract_id = $1")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(health_count.0, 0, "contract_health_checks should be purged");

    let escrow_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM escrow WHERE contract_id = $1")
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(escrow_count.0, 0, "escrow should be purged");

    let bw_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM bandwidth_history WHERE contract_id = $1")
            .bind(&hex_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(bw_count.0, 0, "bandwidth_history should be purged");

    let history_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM contract_status_history WHERE contract_id = $1")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        history_count.0, 0,
        "contract_status_history should be purged"
    );
}

#[tokio::test]
async fn test_purge_terminal_contracts_all_terminal_statuses() {
    let db = setup_test_db().await;
    let ids: Vec<Vec<u8>> = (0..3).map(|i| vec![60 + i; 32]).collect();

    insert_terminal_contract(&db, &ids[0], "rejected", old_timestamp_ns(200)).await;
    insert_terminal_contract(&db, &ids[1], "cancelled", old_timestamp_ns(200)).await;
    insert_terminal_contract(&db, &ids[2], "expired", old_timestamp_ns(200)).await;

    let purged = db.purge_terminal_contracts(180).await.unwrap();
    assert_eq!(purged, 3, "all three terminal statuses should be purged");
}

#[tokio::test]
async fn test_purge_terminal_contracts_none_to_purge() {
    let db = setup_test_db().await;

    let purged = db.purge_terminal_contracts(180).await.unwrap();
    assert_eq!(purged, 0);
}

// --- duration validation tests ---

/// Helper to insert an offering with optional min/max contract hours
async fn insert_offering_with_duration_limits(
    db: &Database,
    provider_pk: &[u8],
    offering_id_str: &str,
    min_hours: Option<i64>,
    max_hours: Option<i64>,
) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns, min_contract_hours, max_contract_hours) VALUES ($1, $2, 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', FALSE, 0, $3, $4) RETURNING id",
    )
    .bind(provider_pk)
    .bind(offering_id_str)
    .bind(min_hours)
    .bind(max_hours)
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

fn rental_params(offering_db_id: i64, duration_hours: Option<i64>) -> RentalRequestParams {
    RentalRequestParams {
        offering_db_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: None,
        duration_hours,
        payment_method: Some("test".to_string()),
        buyer_address: None,
        operating_system: None,
    }
}

#[tokio::test]
async fn test_create_rental_rejects_negative_duration() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    let oid = insert_offering_with_duration_limits(&db, &provider_pk, "off-neg", None, None).await;
    let result = db
        .create_rental_request(&user_pk, rental_params(oid, Some(-5)))
        .await;

    let err = result.unwrap_err().to_string();
    assert!(err.contains("at least 1"), "got: {}", err);
}

#[tokio::test]
async fn test_create_rental_rejects_zero_duration() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    let oid = insert_offering_with_duration_limits(&db, &provider_pk, "off-zero", None, None).await;
    let result = db
        .create_rental_request(&user_pk, rental_params(oid, Some(0)))
        .await;

    let err = result.unwrap_err().to_string();
    assert!(err.contains("at least 1"), "got: {}", err);
}

#[tokio::test]
async fn test_create_rental_rejects_below_min_contract_hours() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    let oid =
        insert_offering_with_duration_limits(&db, &provider_pk, "off-min", Some(24), None).await;
    let result = db
        .create_rental_request(&user_pk, rental_params(oid, Some(12)))
        .await;

    let err = result.unwrap_err().to_string();
    assert!(err.contains("minimum 24 hours"), "got: {}", err);
}

#[tokio::test]
async fn test_create_rental_rejects_above_max_contract_hours() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    let oid =
        insert_offering_with_duration_limits(&db, &provider_pk, "off-max", None, Some(720)).await;
    let result = db
        .create_rental_request(&user_pk, rental_params(oid, Some(1440)))
        .await;

    let err = result.unwrap_err().to_string();
    assert!(err.contains("maximum 720 hours"), "got: {}", err);
}

#[tokio::test]
async fn test_create_rental_accepts_valid_duration_within_limits() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    let oid =
        insert_offering_with_duration_limits(&db, &provider_pk, "off-valid", Some(24), Some(2160))
            .await;
    let contract_id = db
        .create_rental_request(&user_pk, rental_params(oid, Some(720)))
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.duration_hours, Some(720));
}

#[tokio::test]
async fn test_create_rental_default_720_rejected_if_below_min() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Offering requires minimum 2160 hours (3 months) - default 720 should fail
    let oid =
        insert_offering_with_duration_limits(&db, &provider_pk, "off-high-min", Some(2160), None)
            .await;
    let result = db
        .create_rental_request(&user_pk, rental_params(oid, None))
        .await;

    let err = result.unwrap_err().to_string();
    assert!(err.contains("minimum 2160 hours"), "got: {}", err);
}

// --- extend_contract duration validation tests ---

/// Create an active contract for extension testing
async fn create_active_contract(
    db: &Database,
    contract_id: &[u8],
    requester: &[u8],
    provider: &[u8],
    offering_id_str: &str,
    duration_hours: i64,
) {
    insert_contract_request(
        db,
        contract_id,
        requester,
        provider,
        offering_id_str,
        0,
        "active",
    )
    .await;
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap();
    let end_ns = now_ns + (duration_hours * 3600 * 1_000_000_000);
    sqlx::query(
        "UPDATE contract_sign_requests SET end_timestamp_ns = $1, duration_hours = $2 WHERE contract_id = $3",
    )
    .bind(end_ns)
    .bind(duration_hours)
    .bind(contract_id)
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_extend_contract_rejects_zero_hours() {
    let db = setup_test_db().await;
    let requester = vec![70u8; 32];
    let provider = vec![71u8; 32];
    let contract_id = vec![72u8; 32];

    insert_offering_with_duration_limits(&db, &provider, "off-ext-zero", None, None).await;
    create_active_contract(
        &db,
        &contract_id,
        &requester,
        &provider,
        "off-ext-zero",
        720,
    )
    .await;

    let result = db.extend_contract(&contract_id, &requester, 0, None).await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("at least 1"), "got: {}", err);
}

#[tokio::test]
async fn test_extend_contract_rejects_negative_hours() {
    let db = setup_test_db().await;
    let requester = vec![73u8; 32];
    let provider = vec![74u8; 32];
    let contract_id = vec![75u8; 32];

    insert_offering_with_duration_limits(&db, &provider, "off-ext-neg", None, None).await;
    create_active_contract(&db, &contract_id, &requester, &provider, "off-ext-neg", 720).await;

    let result = db
        .extend_contract(&contract_id, &requester, -10, None)
        .await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("at least 1"), "got: {}", err);
}

#[tokio::test]
async fn test_extend_contract_rejects_exceeding_max_hours() {
    let db = setup_test_db().await;
    let requester = vec![76u8; 32];
    let provider = vec![77u8; 32];
    let contract_id = vec![78u8; 32];

    insert_offering_with_duration_limits(&db, &provider, "off-ext-max", None, Some(1000)).await;
    create_active_contract(&db, &contract_id, &requester, &provider, "off-ext-max", 720).await;

    // 720 + 500 = 1220 > 1000 max
    let result = db
        .extend_contract(&contract_id, &requester, 500, None)
        .await;
    let err = result.unwrap_err().to_string();
    assert!(err.contains("exceed maximum 1000 hours"), "got: {}", err);
}

#[tokio::test]
async fn test_extend_contract_accepts_valid_extension() {
    let db = setup_test_db().await;
    let requester = vec![79u8; 32];
    let provider = vec![80u8; 32];
    let contract_id = vec![81u8; 32];

    insert_offering_with_duration_limits(&db, &provider, "off-ext-ok", None, Some(2000)).await;
    create_active_contract(&db, &contract_id, &requester, &provider, "off-ext-ok", 720).await;

    // 720 + 200 = 920 < 2000 max
    let payment = db
        .extend_contract(&contract_id, &requester, 200, None)
        .await
        .unwrap();
    assert!(payment > 0, "extension should have a payment amount");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.duration_hours, Some(920));
}

// === Contract Health Summary Tests ===

#[tokio::test]
async fn test_get_contract_health_summary_with_checks() {
    let db = setup_test_db().await;
    let user_pk = vec![90u8; 32];
    let provider_pk = vec![91u8; 32];
    let contract_id = vec![92u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-chk-1",
        0,
        "active",
    )
    .await;

    let base_ns = 1_700_000_000_000_000_000_i64;

    // 3 healthy, 1 unhealthy, 1 unknown
    db.record_health_check(&contract_id, base_ns, "healthy", Some(10), None)
        .await
        .unwrap();
    db.record_health_check(
        &contract_id,
        base_ns + 60_000_000_000,
        "healthy",
        Some(20),
        None,
    )
    .await
    .unwrap();
    db.record_health_check(
        &contract_id,
        base_ns + 120_000_000_000,
        "unhealthy",
        None,
        None,
    )
    .await
    .unwrap();
    db.record_health_check(
        &contract_id,
        base_ns + 180_000_000_000,
        "healthy",
        Some(30),
        None,
    )
    .await
    .unwrap();
    db.record_health_check(
        &contract_id,
        base_ns + 240_000_000_000,
        "unknown",
        None,
        None,
    )
    .await
    .unwrap();

    let summary = db.get_contract_health_summary(&contract_id).await.unwrap();

    assert_eq!(summary.total_checks, 5);
    assert_eq!(summary.healthy_checks, 3);
    assert_eq!(summary.unhealthy_checks, 1);
    assert_eq!(summary.unknown_checks, 1);
    assert!(
        (summary.uptime_percent - 60.0).abs() < 0.01,
        "expected 60% uptime, got {}",
        summary.uptime_percent
    );
    // avg of 10, 20, 30 = 20ms (unhealthy and unknown have no latency)
    let avg = summary
        .avg_latency_ms
        .expect("avg_latency_ms should be Some when latency data exists");
    assert!(
        (avg - 20.0).abs() < 0.01,
        "expected avg latency 20ms, got {}",
        avg
    );
    assert_eq!(summary.last_checked_at, Some(base_ns + 240_000_000_000));
}

#[tokio::test]
async fn test_get_contract_health_summary_all_unhealthy_returns_zero_uptime() {
    let db = setup_test_db().await;
    let user_pk = vec![93u8; 32];
    let provider_pk = vec![94u8; 32];
    let contract_id = vec![95u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-chk-2",
        0,
        "active",
    )
    .await;

    let base_ns = 1_700_000_000_000_000_000_i64;
    db.record_health_check(&contract_id, base_ns, "unhealthy", Some(999), None)
        .await
        .unwrap();
    db.record_health_check(
        &contract_id,
        base_ns + 60_000_000_000,
        "unhealthy",
        Some(999),
        None,
    )
    .await
    .unwrap();

    let summary = db.get_contract_health_summary(&contract_id).await.unwrap();

    assert_eq!(summary.total_checks, 2);
    assert_eq!(summary.healthy_checks, 0);
    assert_eq!(summary.unhealthy_checks, 2);
    assert_eq!(summary.uptime_percent, 0.0);
}

#[tokio::test]
async fn test_get_contract_health_summary_no_checks_returns_none_last_checked_at() {
    let db = setup_test_db().await;
    let user_pk = vec![96u8; 32];
    let provider_pk = vec![97u8; 32];
    let contract_id = vec![98u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &user_pk,
        &provider_pk,
        "off-chk-3",
        0,
        "active",
    )
    .await;

    let summary = db.get_contract_health_summary(&contract_id).await.unwrap();

    assert_eq!(summary.total_checks, 0);
    assert_eq!(summary.healthy_checks, 0);
    assert_eq!(summary.uptime_percent, 0.0);
    assert!(
        summary.last_checked_at.is_none(),
        "last_checked_at should be None when there are no checks"
    );
    assert!(
        summary.avg_latency_ms.is_none(),
        "avg_latency_ms should be None when there are no checks"
    );
}

/// Verifies the checkout.session.completed flow populates BOTH
/// `stripe_checkout_session_id` (cs_*) AND `stripe_payment_intent_id` (pi_*).
/// Issue #422: previously the cs_* ID was misnamed and stored in the PI column.
#[tokio::test]
async fn test_checkout_session_completed_captures_pi_id() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![222u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    let session_id = "cs_test_session_422";
    let payment_intent_id = "pi_test_intent_422";

    db.update_checkout_session_payment(
        &contract_id,
        session_id,
        Some(payment_intent_id),
        Some(150_000_000),
        Some("eu_vat: DE123456789"),
        false,
        Some("in_test_invoice_422"),
    )
    .await
    .expect("update_checkout_session_payment should succeed");

    let contract = db
        .get_contract(&contract_id)
        .await
        .unwrap()
        .expect("contract should exist");

    assert_eq!(
        contract.stripe_checkout_session_id.as_deref(),
        Some(session_id),
        "checkout session ID must land in stripe_checkout_session_id"
    );
    assert_eq!(
        contract.stripe_payment_intent_id.as_deref(),
        Some(payment_intent_id),
        "real PaymentIntent ID must land in stripe_payment_intent_id"
    );
    assert_eq!(contract.payment_status, "succeeded");
    assert_eq!(contract.tax_amount_e9s, Some(150_000_000));
    assert_eq!(
        contract.stripe_invoice_id.as_deref(),
        Some("in_test_invoice_422")
    );
}

/// Negative path: when Stripe has not yet attached a PaymentIntent to the
/// session, the PI column is left NULL but the session column is still set.
#[tokio::test]
async fn test_checkout_session_completed_without_pi_leaves_pi_null() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![223u8; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "pending",
    )
    .await;

    db.update_checkout_session_payment(
        &contract_id,
        "cs_test_session_no_pi",
        None,
        None,
        None,
        false,
        None,
    )
    .await
    .expect("update_checkout_session_payment should succeed without PI");

    let contract = db
        .get_contract(&contract_id)
        .await
        .unwrap()
        .expect("contract should exist");

    assert_eq!(
        contract.stripe_checkout_session_id.as_deref(),
        Some("cs_test_session_no_pi")
    );
    assert!(
        contract.stripe_payment_intent_id.is_none(),
        "PI column must remain NULL when Stripe sent no payment_intent"
    );
}

// =============================================================================
// Stripe dispute pause/resume helpers (Phase 1: DB layer only).
// Webhook handlers + dc-agent runtime change land in Phase 2.
// =============================================================================

fn dispute_upsert<'a>(
    contract_id: Option<&'a [u8]>,
    dispute_id: &'a str,
    charge_id: &'a str,
    status: &'a str,
    amount_cents: i64,
    raw_event: &'a serde_json::Value,
) -> ContractDisputeUpsert<'a> {
    ContractDisputeUpsert {
        contract_id,
        stripe_dispute_id: dispute_id,
        stripe_charge_id: charge_id,
        stripe_payment_intent_id: None,
        reason: Some("fraudulent"),
        status,
        amount_cents,
        currency: "usd",
        evidence_due_by_ns: None,
        funds_withdrawn_at_ns: None,
        closed_at_ns: None,
        raw_event,
    }
}

async fn count_disputes(db: &Database, dispute_id: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contract_disputes WHERE stripe_dispute_id = $1",
    )
    .bind(dispute_id)
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

async fn count_history(db: &Database, contract_id: &[u8], new_status: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contract_status_history WHERE contract_id = $1 AND new_status = $2",
    )
    .bind(contract_id)
    .bind(new_status)
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

async fn count_events(db: &Database, contract_id: &[u8], event_type: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM contract_events WHERE contract_id = $1 AND event_type = $2",
    )
    .bind(contract_id)
    .bind(event_type)
    .fetch_one(&db.pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn test_upsert_dispute_idempotent() {
    // Stripe replays webhooks indefinitely on non-2xx; the upsert MUST keep
    // exactly one row per stripe_dispute_id, refresh mutable fields on
    // replay, and never blow away the original created_at_ns.
    let db = setup_test_db().await;
    let contract_id = vec![0xAA; 32];
    insert_contract_request(
        &db,
        &contract_id,
        &[1u8; 32],
        &[2u8; 32],
        "off-disp",
        0,
        "active",
    )
    .await;

    let raw_v1 = serde_json::json!({"id": "du_idemp", "v": 1});
    db.upsert_contract_dispute(dispute_upsert(
        Some(&contract_id),
        "du_idemp",
        "ch_idemp",
        "needs_response",
        500,
        &raw_v1,
    ))
    .await
    .unwrap();

    // First insert: row count = 1
    assert_eq!(count_disputes(&db, "du_idemp").await, 1);
    let initial: (i64, String, serde_json::Value) = sqlx::query_as(
        "SELECT created_at_ns, status, raw_event FROM contract_disputes WHERE stripe_dispute_id = 'du_idemp'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(initial.1, "needs_response");

    // Replay with new status + payload
    let raw_v2 = serde_json::json!({"id": "du_idemp", "v": 2, "extra": true});
    db.upsert_contract_dispute(dispute_upsert(
        Some(&contract_id),
        "du_idemp",
        "ch_idemp",
        "under_review",
        500,
        &raw_v2,
    ))
    .await
    .unwrap();

    // Still one row -- idempotent on stripe_dispute_id UNIQUE
    assert_eq!(
        count_disputes(&db, "du_idemp").await,
        1,
        "ON CONFLICT must keep a single row per stripe_dispute_id"
    );

    let after: (i64, String, serde_json::Value) = sqlx::query_as(
        "SELECT created_at_ns, status, raw_event FROM contract_disputes WHERE stripe_dispute_id = 'du_idemp'",
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        after.0, initial.0,
        "created_at_ns must be preserved across replays"
    );
    assert_eq!(
        after.1, "under_review",
        "status must reflect the latest delivery"
    );
    assert_eq!(
        after.2, raw_v2,
        "raw_event must reflect the latest delivery"
    );
}

#[tokio::test]
async fn test_pause_contract_idempotent() {
    // pause_contract on an Active contract: status -> paused, paused_at_ns set,
    // pause_reason set, ONE history row, ONE 'paused' event.
    // Calling again with the same reason: NO new history/event row.
    // Calling with a DIFFERENT reason on a paused contract: loud failure.
    let db = setup_test_db().await;
    let contract_id = vec![0xB1; 32];
    insert_contract_request(
        &db,
        &contract_id,
        &[1u8; 32],
        &[2u8; 32],
        "off-pause",
        0,
        "active",
    )
    .await;

    db.pause_contract(&contract_id, "stripe_dispute:du_p1")
        .await
        .unwrap();

    let row: (String, Option<i64>, Option<String>) = sqlx::query_as(
        "SELECT status, paused_at_ns, pause_reason FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(row.0, "paused", "status must be paused");
    assert!(row.1.is_some(), "paused_at_ns must be set");
    assert_eq!(
        row.2.as_deref(),
        Some("stripe_dispute:du_p1"),
        "pause_reason must be persisted"
    );
    assert_eq!(count_history(&db, &contract_id, "paused").await, 1);
    assert_eq!(count_events(&db, &contract_id, "paused").await, 1);

    // Replay -- same reason. Must be a no-op (no extra history, no extra event,
    // and paused_at_ns must NOT advance to "now" -- otherwise resume would
    // under-credit the customer for the original pause window).
    let paused_at_before = row.1.unwrap();
    db.pause_contract(&contract_id, "stripe_dispute:du_p1")
        .await
        .unwrap();
    assert_eq!(
        count_history(&db, &contract_id, "paused").await,
        1,
        "replay must NOT insert another history row"
    );
    assert_eq!(
        count_events(&db, &contract_id, "paused").await,
        1,
        "replay must NOT insert another event row"
    );
    let paused_at_after: Option<i64> =
        sqlx::query_scalar("SELECT paused_at_ns FROM contract_sign_requests WHERE contract_id = $1")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(
        paused_at_after,
        Some(paused_at_before),
        "replay must NOT bump paused_at_ns"
    );

    // Conflicting concurrent pause -- loud failure (operator-level event).
    let err = db
        .pause_contract(&contract_id, "stripe_dispute:du_OTHER")
        .await
        .unwrap_err();
    let msg = format!("{:#}", err);
    assert!(
        msg.contains("already paused"),
        "conflicting reason must surface error referencing the existing pause, got: {}",
        msg
    );
}

#[tokio::test]
async fn test_resume_contract_credits_paused_time() {
    // pause -> small wait -> resume. After resume:
    //  - status back to active
    //  - paused_at_ns / pause_reason cleared
    //  - total_paused_ns >= the wait we slept
    //  - one resume event recorded
    //  - resume returns ResumeOutcome { resumed: true, status: Active, credited > 0 }
    // Then a SECOND resume on an already-active contract is a no-op.
    let db = setup_test_db().await;
    let contract_id = vec![0xB2; 32];
    insert_contract_request(
        &db,
        &contract_id,
        &[1u8; 32],
        &[2u8; 32],
        "off-resume",
        0,
        "active",
    )
    .await;

    db.pause_contract(&contract_id, "stripe_dispute:du_r1")
        .await
        .unwrap();

    // Wait long enough that the credited interval is unambiguously > 0.
    // (now_ns is monotonic + ns-resolution; even ~10ms gives ~10M ns.)
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    let outcome = db.resume_contract(&contract_id).await.unwrap();
    assert!(outcome.resumed, "first resume on a paused contract must run");
    assert_eq!(outcome.status, dcc_common::ContractStatus::Active);
    assert!(
        outcome.credited_pause_ns >= 10_000_000,
        "credited_pause_ns must be >= 10ms (we slept 20ms), got {}",
        outcome.credited_pause_ns
    );

    let row: (String, Option<i64>, Option<String>, i64) = sqlx::query_as(
        "SELECT status, paused_at_ns, pause_reason, total_paused_ns FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(row.0, "active");
    assert!(row.1.is_none(), "paused_at_ns must be cleared on resume");
    assert!(row.2.is_none(), "pause_reason must be cleared on resume");
    assert_eq!(
        row.3, outcome.credited_pause_ns,
        "total_paused_ns must equal the credited interval (no prior pauses)"
    );
    assert_eq!(count_events(&db, &contract_id, "resumed").await, 1);

    // Second resume -- already active. Must be a no-op (resumed=false), and
    // total_paused_ns must NOT change.
    let noop = db.resume_contract(&contract_id).await.unwrap();
    assert!(!noop.resumed);
    assert_eq!(noop.credited_pause_ns, 0);
    assert_eq!(noop.status, dcc_common::ContractStatus::Active);
    let total_after: i64 = sqlx::query_scalar(
        "SELECT total_paused_ns FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(
        total_after, row.3,
        "no-op resume must not change total_paused_ns"
    );
    assert_eq!(
        count_events(&db, &contract_id, "resumed").await,
        1,
        "no-op resume must not insert another event"
    );
}

#[tokio::test]
async fn test_terminate_for_dispute_lost() {
    // pause then terminate -> status=cancelled, payment_status=disputed,
    // history row pause->cancelled, event 'dispute_lost' with the dispute id.
    // Replay (already terminal) -> no second history row, but a fresh
    // dispute_lost event for the audit trail.
    let db = setup_test_db().await;
    let contract_id = vec![0xB3; 32];
    insert_contract_request(
        &db,
        &contract_id,
        &[1u8; 32],
        &[2u8; 32],
        "off-lost",
        0,
        "active",
    )
    .await;

    db.pause_contract(&contract_id, "stripe_dispute:du_lost1")
        .await
        .unwrap();
    db.terminate_contract_for_dispute_lost(&contract_id, "du_lost1")
        .await
        .unwrap();

    let row: (String, String) = sqlx::query_as(
        "SELECT status, payment_status FROM contract_sign_requests WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(row.0, "cancelled");
    assert_eq!(row.1, "disputed");
    assert_eq!(
        count_history(&db, &contract_id, "cancelled").await,
        1,
        "exactly one paused->cancelled history row"
    );
    assert_eq!(
        count_events(&db, &contract_id, "dispute_lost").await,
        1,
        "exactly one dispute_lost event after first call"
    );
    let detail: Option<String> = sqlx::query_scalar(
        "SELECT details FROM contract_events WHERE contract_id = $1 AND event_type = 'dispute_lost' ORDER BY id DESC LIMIT 1",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert!(
        detail.as_deref().unwrap_or("").contains("du_lost1"),
        "event details must include the stripe_dispute_id, got: {:?}",
        detail
    );

    // Replay on a terminal contract -- no second history row, but a NEW
    // dispute_lost event row preserves the audit trail of the second
    // delivery (Stripe replays are themselves operator-relevant signal).
    db.terminate_contract_for_dispute_lost(&contract_id, "du_lost1")
        .await
        .unwrap();
    assert_eq!(
        count_history(&db, &contract_id, "cancelled").await,
        1,
        "terminal short-circuit must NOT insert another history row"
    );
    assert_eq!(
        count_events(&db, &contract_id, "dispute_lost").await,
        2,
        "second delivery must record an audit event"
    );
}

/// End-to-end wiring test for issue #411: when cancel_contract refunds a
/// Stripe-paid contract, it MUST first write a `requested` row to
/// `refund_audit` with the deterministic idempotency key. The
/// `stripe_client=None` path documented elsewhere in this file lets us
/// exercise the audit + key construction without a Stripe HTTP mock; the
/// status stays at `requested` (no Stripe call to flip it).
#[tokio::test]
async fn test_cancel_refund_uses_idempotency_key() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![201u8; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let start_ns = now_ns - 1_000_000_000;
    let end_ns = now_ns + 10_000_000_000;
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-1".to_string(),
            payment_intent_id: "pi_audit_test".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;

    db.cancel_contract(&contract_id, &requester_pk, Some("e2e"), None)
        .await
        .expect("cancel must succeed");

    // Exactly one audit row for this contract, status=requested (no Stripe
    // client -> no API call -> no completion).
    let rows: Vec<(String, String, i64, Option<String>)> = sqlx::query_as(
        "SELECT idempotency_key, status, amount_cents, stripe_refund_id
           FROM refund_audit WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 1, "cancel must record exactly one audit row");
    let (key, status, amount_cents, refund_id) = rows.into_iter().next().unwrap();
    assert!(
        key.starts_with(&format!("cancel:{}:cancel:", hex::encode(&contract_id))),
        "idempotency_key must follow the cancel:<contract_hex>:cancel:<ts> shape, got {}",
        key
    );
    assert_eq!(status, "requested", "no Stripe client -> stays at requested");
    assert!(amount_cents > 0, "refund must be positive on a fresh cancel");
    assert!(refund_id.is_none(), "no Stripe call -> no refund_id");
}

// =============================================================================
// Money-safety hardening (Phase 1A).
// =============================================================================

/// R10 / A1 belt-and-suspenders: the DB CHECK constraint (migration 047)
/// makes the allow-list un-bypassable even via direct SQL. A raw UPDATE with
/// a garbage value must violate the constraint.
#[tokio::test]
async fn test_payment_status_check_constraint_blocks_raw_update() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xC3; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "requested",
    )
    .await;

    let result = sqlx::query(
        "UPDATE contract_sign_requests SET payment_status = 'garbage' WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await;
    assert!(
        result.is_err(),
        "CHECK constraint must reject a garbage payment_status via direct SQL"
    );
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.to_lowercase().contains("check")
            || err.to_lowercase().contains("constraint")
            || err.contains("payment_status"),
        "violation must reference the check constraint / column, got: {err}"
    );
}

// -----------------------------------------------------------------------------
// R1 / A2: provider status transitions MUST NOT reach provisioned/active on an
// unpaid contract. A provider driving requested -> accepted -> provisioning ->
// provisioned while payment_status is still 'pending' delivers a VM against a
// payment that never landed. The only existing gate was the later
// acquire_provisioning_lock conditional UPDATE; update_contract_status itself
// never read payment_status.

/// Override payment_status / amount on an inserted contract. Runtime query
/// (not sqlx::query!) so test setup never forces a sqlx-prepare cycle.
async fn set_contract_payment(
    db: &Database,
    contract_id: &[u8],
    payment_status: &str,
    payment_amount_e9s: i64,
) {
    sqlx::query(
        "UPDATE contract_sign_requests SET payment_status = $1, payment_amount_e9s = $2 WHERE contract_id = $3",
    )
    .bind(payment_status)
    .bind(payment_amount_e9s)
    .bind(contract_id)
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_update_contract_status_blocks_provisioned_when_unpaid() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xD1; 32];

    // Paid contract whose checkout never completed: non-zero amount, payment
    // still pending. Status already at 'provisioning' so the only remaining
    // hop is provisioning -> provisioned. (insert_contract_request defaults
    // payment_status='succeeded'; flip it to 'pending' to model the unpaid case.)
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-a2",
        0,
        "provisioning",
    )
    .await;
    set_contract_payment(&db, &contract_id, "pending", 1000).await;

    let result = db
        .update_contract_status(&contract_id, "provisioned", &provider_pk, None)
        .await;
    assert!(
        result.is_err(),
        "transition to provisioned must be rejected while payment_status is pending"
    );
    let err = format!("{:#}", result.unwrap_err());
    assert!(
        err.to_lowercase().contains("payment"),
        "error must explain the payment gate, got: {err}"
    );

    // Row untouched -- no silent partial transition.
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "provisioning");
    assert_eq!(contract.payment_status, "pending");
}

#[tokio::test]
async fn test_update_contract_status_blocks_active_when_unpaid() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xD2; 32];

    // Start from 'paused' (NOT provisioned/active): migration 048 forbids a
    // provisioned/active row from holding an unpaid, non-zero state, so we
    // cannot construct one. 'paused' is a valid ->active source state and is
    // outside the 048 gate, letting us model an unpaid contract attempting to
    // (re)enter 'active'. The code gate must refuse it.
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-a2",
        0,
        "paused",
    )
    .await;
    set_contract_payment(&db, &contract_id, "pending", 1000).await;

    let result = db
        .update_contract_status(&contract_id, "active", &provider_pk, None)
        .await;
    assert!(
        result.is_err(),
        "transition to active must be rejected while payment_status is pending"
    );
    let err = format!("{:#}", result.unwrap_err());
    assert!(err.to_lowercase().contains("payment"), "got: {err}");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "paused");
}

#[tokio::test]
async fn test_update_contract_status_allows_provisioned_when_paid() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xD3; 32];

    // Same transition, but payment_status='succeeded' (the insert default) ->
    // must succeed.
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-a2",
        0,
        "provisioning",
    )
    .await;

    db.update_contract_status(&contract_id, "provisioned", &provider_pk, None)
        .await
        .expect("paid contract may transition to provisioned");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "provisioned");
}

#[tokio::test]
async fn test_update_contract_status_allows_provisioned_when_free() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xD4; 32];

    // Free / self-rental contract: payment_amount_e9s == 0. Must NOT be blocked
    // even though payment_status is not 'succeeded'.
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-a2",
        0,
        "provisioning",
    )
    .await;
    set_contract_payment(&db, &contract_id, "pending", 0).await;

    db.update_contract_status(&contract_id, "provisioned", &provider_pk, None)
        .await
        .expect("free (amount=0) contract may transition regardless of payment_status");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "provisioned");
}

/// R1 / A2 belt-and-suspenders: the DB CHECK (migration 048) makes the gate
/// un-bypassable even via direct SQL. Start from 'provisioning' (allowed by
/// 048 to hold an unpaid, non-zero state) and attempt a raw UPDATE to a gated
/// status ('provisioned') -- the constraint must reject it.
#[tokio::test]
async fn test_provisioning_requires_payment_check_constraint() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xD5; 32];

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-a2",
        0,
        "provisioning",
    )
    .await;
    set_contract_payment(&db, &contract_id, "pending", 1000).await;

    let result = sqlx::query(
        "UPDATE contract_sign_requests SET status = 'provisioned' WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await;
    assert!(
        result.is_err(),
        "CHECK constraint must reject provisioned on an unpaid non-zero contract"
    );
}

// -----------------------------------------------------------------------------
// R2/R3 / A3: refund accounting integrity.
//
// Under Stripe-only no funds are ever pre-released to the provider, so the
// refund is bounded purely by the collected payment. reject/cancel/dispute
// routes through calculate_net_refund_e9s (the gross prorated refund);
// migration 049 is the un-bypassable CHECK that refunded <= payment.

/// R3 variant: reject MUST record the prorated refund owed to the customer and
/// never refund more than the collected payment. Regression guard for the
/// over-refund where reject used payment_amount_e9s directly on top of an
/// already-refunded/disputed contract.
#[tokio::test]
async fn test_reject_contract_records_refund_not_exceeding_payment() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xE1; 32];
    let payment_amount_e9s = 1_000_000_000i64; // $1.00 == 100 cents

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-a3",
        0,
        "accepted", // reject is valid from accepted
    )
    .await;
    // insert_contract_request forces amount=1000 and the Test method; set the
    // realistic Stripe payment. Reject is pre-service, so the prorated refund
    // equals the full payment.
    sqlx::query(
        "UPDATE contract_sign_requests SET payment_amount_e9s = $1, payment_status = 'succeeded', payment_method = 'stripe', stripe_payment_intent_id = 'pi_test_reject' WHERE contract_id = $2",
    )
    .bind(payment_amount_e9s)
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    db.reject_contract(&contract_id, &provider_pk, Some("reject"), None)
        .await
        .expect("reject must succeed");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "rejected");
    let refund = contract
        .refund_amount_e9s
        .expect("reject of a succeeded payment must record a refund");
    // Pre-service reject refunds the full payment; never more.
    assert!(
        refund <= payment_amount_e9s,
        "reject refund ({refund}) must never exceed the collected payment ({payment_amount_e9s})"
    );
    assert!(
        refund > 0,
        "reject of a succeeded payment must record a positive refund"
    );
}

/// R2 / A3 belt-and-suspenders: the DB CHECK (migration 049) makes
/// "refunded <= payment" un-bypassable even via direct SQL. A raw UPDATE that
/// pushes the refund past payment_amount must violate the constraint.
#[tokio::test]
async fn test_release_refund_integrity_check_constraint() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xE2; 32];
    let payment_amount_e9s = 1_000_000_000i64;

    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-a3",
        0,
        "accepted",
    )
    .await;
    // Set the collected payment, then attempt to push the refund past the
    // ceiling via direct SQL.
    sqlx::query(
        "UPDATE contract_sign_requests SET payment_amount_e9s = $1 WHERE contract_id = $2",
    )
    .bind(payment_amount_e9s)
    .bind(&contract_id)
    .execute(&db.pool)
    .await
    .unwrap();

    // 1.1B refund > 1.0B payment -> must be rejected.
    let result = sqlx::query(
        "UPDATE contract_sign_requests SET refund_amount_e9s = 1_100_000_000 WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await;
    assert!(
        result.is_err(),
        "CHECK constraint must reject refunded > payment"
    );
}

// -----------------------------------------------------------------------------
// R5 / A4: no silent refund marking.
//
// issue_audited_refund returns Ok(None) when no Stripe client is configured
// (pure-DB / dry-run). Callers used to treat that as success and flip
// payment_status to 'refunded' -- telling the customer "refunded" while no
// money was actually returned. The invariant: payment_status='refunded' is
// ONLY ever set when a real refund id (Stripe re_*) was recorded.

/// With no Stripe client, a cancel computes the refund but cannot issue it, so
/// payment_status must NOT become 'refunded' (it stays in its prior state).
#[tokio::test]
async fn test_cancel_stripe_without_client_does_not_mark_refunded() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xF1; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow (year > 2262)");
    let start_ns = now_ns - 1_000_000_000;
    let end_ns = now_ns + 10_000_000_000;
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-a4".to_string(),
            payment_intent_id: "pi_a4_noclient".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;

    // Cancel with NO Stripe client: refund computes but cannot be issued.
    db.cancel_contract(&contract_id, &requester_pk, Some("cancel"), None)
        .await
        .expect("cancel must still succeed (status flip)");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // R5 invariant: no refund id -> payment_status must NOT be 'refunded'.
    assert_ne!(
        contract.payment_status, "refunded",
        "payment_status must not become 'refunded' when no refund was actually issued"
    );
    assert_eq!(
        contract.payment_status, "succeeded",
        "prior payment_status must be preserved"
    );
    assert!(
        contract.stripe_refund_id.is_none(),
        "no Stripe client -> no refund id"
    );
}

/// Mirror of the above for the reject path: rejecting a succeeded Stripe
/// contract with no client must NOT mark payment_status='refunded'.
#[tokio::test]
async fn test_reject_stripe_without_client_does_not_mark_refunded() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![0xF2; 32];

    let now_ns = crate::now_ns().unwrap();
    let start_ns = now_ns;
    let end_ns = now_ns + 10_000_000_000;
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "off-a4-reject".to_string(),
            payment_intent_id: "pi_a4_reject".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;

    db.reject_contract(&contract_id, &provider_pk, Some("reject"), None)
        .await
        .expect("reject must still succeed (status flip)");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "rejected");
    assert_ne!(
        contract.payment_status, "refunded",
        "reject with no client must not mark refunded"
    );
    assert_eq!(contract.payment_status, "succeeded");
    assert!(contract.stripe_refund_id.is_none());
}

// =========================================================================
// Refund approval gate tests (migration 051, refund_requests table)
// =========================================================================

/// Helper: fetch the refund_request row for a contract (if any).
async fn fetch_refund_request_for_contract(
    db: &Database,
    contract_id: &[u8],
) -> Option<crate::database::refund_requests::RefundRequest> {
    sqlx::query_as(
        r#"SELECT id, contract_id, requester_pubkey, refund_amount_e9s,
                  reason, status, user_latest_payment_e9s, cap_exceeded,
                  payment_intent_id, currency, stripe_dispute_id,
                  stripe_refund_id, idempotency_key, created_at_ns,
                  reviewed_at_ns, reviewed_by, review_note
             FROM refund_requests WHERE contract_id = $1"#,
    )
    .bind(contract_id)
    .fetch_optional(&db.pool)
    .await
    .unwrap()
}

/// Helper: set created_at_ns on a contract (non-macro sqlx — literal value).
async fn set_contract_created_at(db: &Database, contract_id: &[u8], ts: i64) {
    sqlx::query("UPDATE contract_sign_requests SET created_at_ns = $1 WHERE contract_id = $2")
        .bind(ts)
        .bind(contract_id)
        .execute(&db.pool)
        .await
        .unwrap();
}

/// Helper: insert a refund_request row directly (non-macro sqlx — SQLX_OFFLINE).
async fn insert_refund_request(
    db: &Database,
    contract_id: &[u8],
    requester_pubkey: &[u8],
    refund_amount_e9s: i64,
    reason: &str,
    status: &str,
    user_latest_payment_e9s: i64,
    cap_exceeded: bool,
    payment_intent_id: &str,
    stripe_refund_id: Option<&str>,
) -> i64 {
    let created_ns = crate::now_ns().unwrap();
    let idem = format!("{}:{}", reason, hex::encode(contract_id));
    let row: (i64,) = sqlx::query_as(
        r#"INSERT INTO refund_requests
             (contract_id, requester_pubkey, refund_amount_e9s, reason, status,
              user_latest_payment_e9s, cap_exceeded, payment_intent_id, currency,
              stripe_refund_id, idempotency_key, created_at_ns)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, 'usd', $9, $10, $11)
           RETURNING id"#,
    )
    .bind(contract_id)
    .bind(requester_pubkey)
    .bind(refund_amount_e9s)
    .bind(reason)
    .bind(status)
    .bind(user_latest_payment_e9s)
    .bind(cap_exceeded)
    .bind(payment_intent_id)
    .bind(stripe_refund_id)
    .bind(&idem)
    .bind(created_ns)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    row.0
}

#[tokio::test]
async fn test_cancel_creates_auto_issued_refund_request_when_cap_passes() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![201u8; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow");
    let start_ns = now_ns - 1_000_000_000;
    let end_ns = now_ns + 10_000_000_000;

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-gate-1".to_string(),
            payment_intent_id: "pi_gate_auto".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000, // $10
            start_timestamp_ns: start_ns,
            end_timestamp_ns: end_ns,
        },
    )
    .await;

    // Single contract → user_latest_payment = this contract's payment = 1B.
    // Prorated refund ≈ full amount (just started). refund ≤ cap → auto-issue.
    db.cancel_contract(&contract_id, &requester_pk, Some("test"), None)
        .await
        .expect("cancel should succeed");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // No stripe client → refund not actually issued → payment_status stays 'succeeded' (R5)
    assert_eq!(contract.payment_status, "succeeded");

    let rr = fetch_refund_request_for_contract(&db, &contract_id)
        .await
        .expect("refund_request row must exist");
    assert_eq!(rr.reason, "cancel");
    assert_eq!(rr.status, "auto_issued");
    assert!(!rr.cap_exceeded);
    assert_eq!(rr.refund_amount_e9s, 1_000_000_000);
}

#[tokio::test]
async fn test_cancel_holds_refund_when_cap_exceeded() {
    let db = setup_test_db().await;
    let requester_pk = vec![3u8; 32];
    let provider_pk = vec![4u8; 32];

    // Contract A: large payment ($200), older created_at → refund target
    let contract_a = vec![202u8; 32];
    // Contract B: small payment ($1), newer created_at → becomes "latest payment" cap
    let contract_b = vec![203u8; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow");

    // Insert contract A (the one we'll cancel)
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_a.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "off-gate-a".to_string(),
            payment_intent_id: "pi_gate_a".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 2_000_000_000, // $200
            start_timestamp_ns: now_ns - 1_000_000_000,
            end_timestamp_ns: now_ns + 10_000_000_000,
        },
    )
    .await;
    // Set created_at_ns to 100 (older)
    set_contract_created_at(&db, &contract_a, 100).await;

    // Insert contract B (the "latest payment" — only $1)
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_b.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "off-gate-b".to_string(),
            payment_intent_id: "pi_gate_b".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 100_000_000, // $1
            start_timestamp_ns: now_ns - 1_000_000_000,
            end_timestamp_ns: now_ns + 10_000_000_000,
        },
    )
    .await;
    // Set created_at_ns to 200 (newer → becomes the "latest payment")
    set_contract_created_at(&db, &contract_b, 200).await;

    // Cancel contract A: prorated refund ≈ $200, but latest payment is $1 → cap exceeded
    db.cancel_contract(&contract_a, &requester_pk, Some("test"), None)
        .await
        .expect("cancel should still succeed (status flip proceeds)");

    let contract = db.get_contract(&contract_a).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // Pending approval → no refund issued → payment_status stays 'succeeded'
    assert_eq!(contract.payment_status, "succeeded");

    let rr = fetch_refund_request_for_contract(&db, &contract_a)
        .await
        .expect("refund_request row must exist");
    assert_eq!(rr.reason, "cancel");
    assert_eq!(rr.status, "pending");
    assert!(rr.cap_exceeded);
    assert_eq!(rr.user_latest_payment_e9s, 100_000_000); // $1 from contract B
    assert_eq!(rr.refund_amount_e9s, 2_000_000_000); // $200
}

#[tokio::test]
async fn test_admin_approve_pending_refund_request() {
    let db = setup_test_db().await;
    let requester_pk = vec![5u8; 32];
    let provider_pk = vec![6u8; 32];
    let contract_id = vec![204u8; 32];
    let admin_pk = vec![7u8; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow");

    // Single contract — cancel creates auto_issued (cap passes). We then
    // simulate a pending request by inserting one directly.
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-approve".to_string(),
            payment_intent_id: "pi_approve".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 500_000_000, // $5
            start_timestamp_ns: now_ns - 1_000_000_000,
            end_timestamp_ns: now_ns + 10_000_000_000,
        },
    )
    .await;

    // Insert a pending refund_request directly (simulating cap-exceeded scenario)
    let rr_id = insert_refund_request(
        &db,
        &contract_id,
        &requester_pk,
        500_000_000,
        "cancel",
        "pending",
        100_000_000, // latest payment only $1
        true,
        "pi_approve",
        None,
    )
    .await;

    // Admin approves (no stripe client → dry-run, but status flips)
    let approved = db
        .approve_refund_request(rr_id, &admin_pk, Some("LGTM"), None)
        .await
        .expect("approve should succeed");

    assert_eq!(approved.status, "approved");
    assert!(approved.reviewed_at_ns.is_some());
    assert_eq!(approved.reviewed_by, Some(admin_pk.clone()));
    assert_eq!(approved.review_note.as_deref(), Some("LGTM"));
}

#[tokio::test]
async fn test_admin_decline_pending_refund_request() {
    let db = setup_test_db().await;
    let requester_pk = vec![8u8; 32];
    let provider_pk = vec![9u8; 32];
    let contract_id = vec![205u8; 32];
    let admin_pk = vec![10u8; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow");

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-decline".to_string(),
            payment_intent_id: "pi_decline".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 500_000_000,
            start_timestamp_ns: now_ns - 1_000_000_000,
            end_timestamp_ns: now_ns + 10_000_000_000,
        },
    )
    .await;

    let rr_id = insert_refund_request(
        &db,
        &contract_id,
        &requester_pk,
        500_000_000,
        "cancel",
        "pending",
        100_000_000,
        true,
        "pi_decline",
        None,
    )
    .await;

    let declined = db
        .decline_refund_request(rr_id, &admin_pk, Some("suspicious"))
        .await
        .expect("decline should succeed");

    assert_eq!(declined.status, "declined");
    assert_eq!(declined.review_note.as_deref(), Some("suspicious"));

    // Verify no refund was issued (stripe_refund_id still None)
    assert!(declined.stripe_refund_id.is_none());
}

#[tokio::test]
async fn test_refund_gate_trigger_blocks_refund_without_request_row() {
    let db = setup_test_db().await;
    let requester_pk = vec![11u8; 32];
    let provider_pk = vec![12u8; 32];
    let contract_id = vec![206u8; 32];

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk,
            provider_pubkey: provider_pk,
            offering_id: "off-trigger".to_string(),
            payment_intent_id: "pi_trigger".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: 0,
            end_timestamp_ns: 1,
        },
    )
    .await;

    // Attempt to set payment_status='refunded' WITHOUT a refund_request row.
    // The trigger must block this — the unbypassable backstop.
    let result = sqlx::query(
        "UPDATE contract_sign_requests SET payment_status = 'refunded' WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await;

    assert!(
        result.is_err(),
        "Trigger must block payment_status='refunded' without a refund_request row"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("refund") || err_msg.contains("Refund") || err_msg.contains("enforce_refund"),
        "Error should mention refund gate, got: {err_msg}"
    );

    // Verify the contract's payment_status is unchanged
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.payment_status, "succeeded");
}

#[tokio::test]
async fn test_refund_gate_trigger_blocks_stripe_refund_id_without_request_row() {
    let db = setup_test_db().await;
    let requester_pk = vec![13u8; 32];
    let provider_pk = vec![14u8; 32];
    let contract_id = vec![207u8; 32];

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk,
            provider_pubkey: provider_pk,
            offering_id: "off-trigger2".to_string(),
            payment_intent_id: "pi_trigger2".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: 0,
            end_timestamp_ns: 1,
        },
    )
    .await;

    // Attempt to set stripe_refund_id without a refund_request row.
    let result = sqlx::query(
        "UPDATE contract_sign_requests SET stripe_refund_id = 're_fake' WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await;

    assert!(
        result.is_err(),
        "Trigger must block setting stripe_refund_id without a refund_request row"
    );
}

#[tokio::test]
async fn test_refund_gate_trigger_allows_with_approved_request() {
    let db = setup_test_db().await;
    let requester_pk = vec![15u8; 32];
    let provider_pk = vec![16u8; 32];
    let contract_id = vec![208u8; 32];
    let admin_pk = vec![17u8; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow");

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-trigger-ok".to_string(),
            payment_intent_id: "pi_trigger_ok".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: 0,
            end_timestamp_ns: 1,
        },
    )
    .await;

    // Insert an APPROVED refund_request (with stripe_refund_id)
    let rr_id = insert_refund_request(
        &db,
        &contract_id,
        &requester_pk,
        1_000_000_000,
        "cancel",
        "approved",
        1_000_000_000,
        false,
        "pi_trigger_ok",
        Some("re_ok"),
    )
    .await;
    // Set reviewed_by
    let now_ns2 = crate::now_ns().unwrap();
    sqlx::query("UPDATE refund_requests SET reviewed_at_ns = $1, reviewed_by = $2 WHERE id = $3")
        .bind(now_ns2)
        .bind(&admin_pk)
        .bind(rr_id)
        .execute(&db.pool)
        .await
        .unwrap();

    // Now the trigger should ALLOW setting payment_status='refunded'
    let result = sqlx::query(
        "UPDATE contract_sign_requests SET payment_status = 'refunded' WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await;

    assert!(result.is_ok(), "Trigger should allow refund with approved request");

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.payment_status, "refunded");
}

#[tokio::test]
async fn test_refund_gate_trigger_rejects_declined_request() {
    let db = setup_test_db().await;
    let requester_pk = vec![18u8; 32];
    let provider_pk = vec![19u8; 32];
    let contract_id = vec![209u8; 32];

    let now_ns = chrono::Utc::now()
        .timestamp_nanos_opt()
        .expect("timestamp overflow");

    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: contract_id.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-trigger-decl".to_string(),
            payment_intent_id: "pi_trigger_decl".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 1_000_000_000,
            start_timestamp_ns: 0,
            end_timestamp_ns: 1,
        },
    )
    .await;

    // Insert a DECLINED refund_request (should NOT allow refund)
    insert_refund_request(
        &db,
        &contract_id,
        &requester_pk,
        1_000_000_000,
        "cancel",
        "declined",
        100_000_000,
        true,
        "pi_trigger_decl",
        None,
    )
    .await;

    // Trigger must STILL block — declined is not auto_issued/approved
    let result = sqlx::query(
        "UPDATE contract_sign_requests SET payment_status = 'refunded' WHERE contract_id = $1",
    )
    .bind(&contract_id)
    .execute(&db.pool)
    .await;

    assert!(
        result.is_err(),
        "Trigger must block refund with a DECLINED request"
    );
}

#[tokio::test]
async fn test_get_user_latest_stripe_payment() {
    let db = setup_test_db().await;
    let requester_pk = vec![20u8; 32];
    let provider_pk = vec![21u8; 32];

    // No contracts → None (cap = 0, all refunds held)
    let latest = db.get_user_latest_stripe_payment(&requester_pk).await.unwrap();
    assert!(latest.is_none());

    // Insert contract with payment = $5
    let cid1 = vec![210u8; 32];
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: cid1.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk.clone(),
            offering_id: "off-latest-1".to_string(),
            payment_intent_id: "pi_latest_1".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 500_000_000, // $5
            start_timestamp_ns: 0,
            end_timestamp_ns: 1,
        },
    )
    .await;
    set_contract_created_at(&db, &cid1, 100).await;

    let latest = db.get_user_latest_stripe_payment(&requester_pk).await.unwrap();
    assert_eq!(latest, Some(500_000_000));

    // Insert newer contract with payment = $2
    let cid2 = vec![211u8; 32];
    insert_stripe_contract_with_timestamps(
        &db,
        StripeContractParams {
            contract_id: cid2.clone(),
            requester_pubkey: requester_pk.clone(),
            provider_pubkey: provider_pk,
            offering_id: "off-latest-2".to_string(),
            payment_intent_id: "pi_latest_2".to_string(),
            payment_status: "succeeded".to_string(),
            payment_amount_e9s: 200_000_000, // $2
            start_timestamp_ns: 0,
            end_timestamp_ns: 1,
        },
    )
    .await;
    set_contract_created_at(&db, &cid2, 200).await;

    // Latest should now be $2 (the most recent, not the largest)
    let latest = db.get_user_latest_stripe_payment(&requester_pk).await.unwrap();
    assert_eq!(latest, Some(200_000_000));
}
