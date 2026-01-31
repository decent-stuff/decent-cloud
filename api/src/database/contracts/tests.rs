use super::*;
use crate::database::test_helpers::setup_test_db;

async fn insert_contract_request(
    db: &Database,
    contract_id: &[u8],
    requester_pubkey: &[u8],
    provider_pubkey: &[u8],
    offering_id: &str,
    created_at_ns: i64,
    status: &str,
) {
    let payment_method = "icpay";
    let payment_status = "succeeded"; // ICPay payments are pre-paid
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
async fn test_create_rental_request_with_icpay_payment_method() {
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
        payment_method: Some("icpay".to_string()),
        buyer_address: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.payment_method, "icpay");
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
        payment_method: Some("icpay".to_string()),
        buyer_address: None,
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
        payment_method: Some("icpay".to_string()),
        buyer_address: None,
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
        payment_method: Some("icpay".to_string()),
        buyer_address: None,
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
    assert_eq!(contract.gateway_ssh_port, None);
    assert_eq!(contract.gateway_port_range_start, None);
    assert_eq!(contract.gateway_port_range_end, None);
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
            .cancel_contract(&contract_id, &requester_pk, None, None, None)
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
        .cancel_contract(&contract_id, &attacker_pk, None, None, None)
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
        .cancel_contract(&contract_id, &provider_pk, None, None, None)
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
            .cancel_contract(&contract_id, &requester_pk, None, None, None)
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
        .cancel_contract(&nonexistent_id, &requester_pk, None, None, None)
        .await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Contract not found"));
    assert!(error_msg.contains(&hex::encode(&nonexistent_id)));
}

#[tokio::test]
async fn test_payment_status_icpay_payment_succeeds_immediately() {
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
        payment_method: Some("icpay".to_string()),
        buyer_address: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    // ICPay payments are pre-paid, so payment_status should be 'succeeded'
    assert_eq!(contract.payment_method, "icpay");
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
    );

    // Should be approximately 90% (900M e9s)
    assert!((899_000_000..=901_000_000).contains(&refund));
}

#[tokio::test]
async fn test_cancel_contract_with_icpay_payment_no_refund() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![100u8; 32];

    // Insert ICPay payment contract
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

    // Cancel without Stripe client (ICPay payment)
    let result = db
        .cancel_contract(&contract_id, &requester_pk, Some("Test cancel"), None, None)
        .await;

    assert!(result.is_ok());

    // Verify contract is cancelled
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    assert_eq!(contract.payment_status, "succeeded"); // ICPay payment status unchanged
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
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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
        .cancel_contract(&contract_id, &requester_pk, Some("Test cancel"), None, None)
        .await;

    assert!(result.is_ok());

    // Verify contract is cancelled with refund amount but no refund ID
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    assert_eq!(contract.payment_status, "refunded");
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
        .cancel_contract(&contract_id, &requester_pk, Some("Test cancel"), None, None)
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

#[tokio::test]
async fn test_cancel_contract_icpay_refund_calculation() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![20u8; 32];

    // Create contract with ICPay payment
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        1_000_000_000, // 1 ICP in e9s
        "requested",
    )
    .await;

    // Set up ICPay payment details
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let start_ns = now_ns - (10 * 24 * 3600 * 1_000_000_000i64); // Started 10 days ago
    let end_ns = start_ns + (30 * 24 * 3600 * 1_000_000_000i64); // 30 day contract

    sqlx::query!(
        "UPDATE contract_sign_requests SET payment_method = $1, payment_status = $2, icpay_payment_id = $3, start_timestamp_ns = $4, end_timestamp_ns = $5 WHERE contract_id = $6",
        "icpay",
        "succeeded",
        "pay_test_123",
        start_ns,
        end_ns,
        contract_id
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Cancel the contract
    db.cancel_contract(&contract_id, &requester_pk, Some("Test cancel"), None, None)
        .await
        .unwrap();

    // Verify refund was calculated
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    assert_eq!(contract.payment_status, "refunded");
    assert!(contract.refund_amount_e9s.is_some());
    let refund = contract.refund_amount_e9s.unwrap();
    // Should be prorated (2/3 of amount since 10/30 days used)
    assert!(refund > 0);
    assert!(refund < 1_000_000_000); // Less than full amount
}

#[tokio::test]
async fn test_cancel_contract_icpay_no_payment_id() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![21u8; 32];

    // Create contract without ICPay payment ID
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        1_000_000_000,
        "requested",
    )
    .await;

    sqlx::query!(
        "UPDATE contract_sign_requests SET payment_method = $1, payment_status = $2 WHERE contract_id = $3",
        "icpay",
        "succeeded",
        contract_id
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Cancel should succeed but not calculate refund
    db.cancel_contract(&contract_id, &requester_pk, None, None, None)
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // No refund since no payment ID
    assert!(contract.refund_amount_e9s.is_none());
    assert!(contract.icpay_refund_id.is_none());
}

#[tokio::test]
async fn test_cancel_contract_icpay_with_released_amount() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![22u8; 32];

    // Create contract with ICPay payment
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        1_000_000_000,
        "requested",
    )
    .await;

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    let start_ns = now_ns - (10 * 24 * 3600 * 1_000_000_000i64);
    let end_ns = start_ns + (30 * 24 * 3600 * 1_000_000_000i64);

    // Set ICPay payment with some amount already released to provider
    sqlx::query!(
        "UPDATE contract_sign_requests SET payment_method = $1, payment_status = $2, icpay_payment_id = $3, start_timestamp_ns = $4, end_timestamp_ns = $5, total_released_e9s = $6 WHERE contract_id = $7",
        "icpay",
        "succeeded",
        "pay_test_456",
        start_ns,
        end_ns,
        300_000_000i64, // 0.3 ICP already released
        contract_id
    )
    .execute(&db.pool)
    .await
    .unwrap();

    // Cancel the contract
    db.cancel_contract(&contract_id, &requester_pk, None, None, None)
        .await
        .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    // Refund should be prorated amount minus already released
    // Expected: 1B * (20/30) - 300M = 666.67M - 300M = 366.67M
    if let Some(refund) = contract.refund_amount_e9s {
        assert_eq!(contract.payment_status, "refunded");
        assert!(refund > 0);
        // Should be less than what it would be without released amount (2/3 of 1 ICP = ~667M)
        assert!(refund < 667_000_000);
        // Should be more than 300M (since we released 300M and there's still 666M prorated)
        assert!(refund > 300_000_000);
    } else {
        // If no refund, the released amount exceeded the prorated amount
        assert_eq!(contract.payment_status, "succeeded");
    }
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

    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
    // Use recent start time and future end time for clearer refund calculation
    let start_ns = now_ns - (3600 * 1_000_000_000i64); // Started 1 hour ago
    let end_ns = now_ns + (23 * 3600 * 1_000_000_000i64); // 24 hour contract, 23 hours left

    // Set ICPay payment and instance details
    let instance_details =
        r#"{"external_id":"vm-12345","ip_address":"192.168.1.100","ssh_port":22}"#;
    sqlx::query!(
        "UPDATE contract_sign_requests SET payment_method = $1, payment_status = $2, icpay_payment_id = $3, provisioning_instance_details = $4, provisioning_completed_at_ns = $5, start_timestamp_ns = $6, end_timestamp_ns = $7 WHERE contract_id = $8",
        "icpay",
        "succeeded",
        "pay_test_active",
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
        None,
    )
    .await
    .unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.status, "cancelled");
    assert_eq!(contract.payment_status, "refunded");
    assert!(contract.refund_amount_e9s.is_some());
    // Prorated refund should be present (23/24 hours remaining = ~96% of 1 ICP)
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

    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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

    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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
    let past_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) - 1_000_000_000;
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
    let past_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) - 1_000_000_000;
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

    let checked_at = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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

    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

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

    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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
    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

    db.record_health_check(&contract_id, now, "healthy", Some(25), Some(details))
        .await
        .unwrap();

    let checks = db.get_recent_health_checks(&contract_id, 1).await.unwrap();

    assert_eq!(checks.len(), 1);
    assert_eq!(checks[0].details, Some(details.to_string()));
    assert_eq!(checks[0].latency_ms, Some(25));
}

// === Subscription Management Tests ===
