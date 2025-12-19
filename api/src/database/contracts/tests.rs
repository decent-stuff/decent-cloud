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
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency) VALUES (?, ?, 'ssh-key', 'contact', ?, ?, 1000, 'memo', ?, ?, ?, ?, ?, ?, 'usd')",
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
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'usd')",
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
async fn test_get_contract_reply() {
    let db = setup_test_db().await;
    let contract_id = vec![3u8; 32];

    // Insert contract first (foreign key requirement)
    let requester = vec![1u8; 32];
    let provider = vec![2u8; 32];
    insert_contract_request(
        &db,
        &contract_id,
        &requester,
        &provider,
        "off-1",
        0,
        "pending",
    )
    .await;

    {
        let contract_id_slice = contract_id.as_slice();
        let provider_slice = provider.as_slice();
        sqlx::query!(
            "INSERT INTO contract_sign_replies (contract_id, provider_pubkey, reply_status, reply_memo, instance_details, created_at_ns) VALUES (?, ?, 'accepted', 'ok', 'details', 0)",
            contract_id_slice,
            provider_slice
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let reply = db.get_contract_reply(&contract_id).await.unwrap();
    assert!(reply.is_some());
    let reply = reply.unwrap();
    assert_eq!(reply.reply_status, "accepted");
}

#[tokio::test]
async fn test_get_contract_payments() {
    let db = setup_test_db().await;
    let contract_id = vec![3u8; 32];

    // Insert contract first (foreign key requirement)
    let requester = vec![1u8; 32];
    let provider = vec![2u8; 32];
    insert_contract_request(
        &db,
        &contract_id,
        &requester,
        &provider,
        "off-1",
        0,
        "pending",
    )
    .await;

    {
        let contract_id_slice = contract_id.as_slice();
        sqlx::query!(
            "INSERT INTO contract_payment_entries (contract_id, pricing_model, time_period_unit, quantity, amount_e9s) VALUES (?, 'fixed', 'month', 1, 1000)",
            contract_id_slice
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }
    {
        let contract_id_slice = contract_id.as_slice();
        sqlx::query!(
            "INSERT INTO contract_payment_entries (contract_id, pricing_model, time_period_unit, quantity, amount_e9s) VALUES (?, 'usage', 'hour', 10, 500)",
            contract_id_slice
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    let payments = db.get_contract_payments(&contract_id).await.unwrap();
    assert_eq!(payments.len(), 2);
    assert_eq!(payments[0].amount_e9s, 1000);
}

#[tokio::test]
async fn test_create_rental_request_with_icpay_payment_method() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create offering
    let provider_pk_clone = provider_pk.clone();
    let offering_id = sqlx::query_scalar!(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-payment-1', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-payment-2', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-payment-3', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-rental-1', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-rental-2', 'Test Server', 'USD', 50.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-rental-3', 'Expensive Server', 'USD', 499.99, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-eur-1', 'EU Server', 'EUR', 89.99, 0, 'public', 'compute', 'monthly', 'in_stock', 'DE', 'Berlin', 0, 0) RETURNING id",
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
        r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = ?"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(status, "accepted");

    let contract_id_param = contract_id.clone();
    let history = sqlx::query!(
        r#"SELECT old_status as "old_status!", new_status as "new_status!", change_memo FROM contract_status_history WHERE contract_id = ? ORDER BY changed_at_ns DESC LIMIT 1"#,
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
        r#"SELECT COUNT(*) as "count!: i64" FROM contract_status_history WHERE contract_id = ?"#,
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
        "SELECT provisioning_instance_details FROM contract_sign_requests WHERE contract_id = ?",
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
        r#"SELECT contract_id as "contract_id!", instance_ip, instance_credentials, connection_instructions, provisioned_at_ns as "provisioned_at_ns!" FROM contract_provisioning_details WHERE contract_id = ?"#,
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
        r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = ?"#,
        contract_id_param
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(status, "cancelled");

    let contract_id_param = contract_id.clone();
    let history = sqlx::query!(
        r#"SELECT old_status as "old_status!", new_status as "new_status!", change_memo FROM contract_status_history WHERE contract_id = ? ORDER BY changed_at_ns DESC LIMIT 1"#,
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

    let cancellable_statuses = ["requested", "pending", "accepted", "provisioning"];

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
            r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = ?"#,
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
        r#"SELECT status as "status!: String" FROM contract_sign_requests WHERE contract_id = ?"#,
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

    let non_cancellable_statuses = ["provisioned", "active", "rejected", "cancelled"];

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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-payment-status-1', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-payment-status-2', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id",
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

    // Insert contract in non-cancellable status
    insert_contract_request(
        &db,
        &contract_id,
        &requester_pk,
        &provider_pk,
        "off-1",
        0,
        "provisioned", // Not cancellable
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
    assert_eq!(contract.status, "provisioned");
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
        "UPDATE contract_sign_requests SET payment_method = ?, payment_status = ?, icpay_payment_id = ?, start_timestamp_ns = ?, end_timestamp_ns = ? WHERE contract_id = ?",
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
        "UPDATE contract_sign_requests SET payment_method = ?, payment_status = ? WHERE contract_id = ?",
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
        "UPDATE contract_sign_requests SET payment_method = ?, payment_status = ?, icpay_payment_id = ?, start_timestamp_ns = ?, end_timestamp_ns = ?, total_released_e9s = ? WHERE contract_id = ?",
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
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals) VALUES (?, 'Test Provider', 'v1', '1.0', 0, 1)",
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
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals) VALUES (?, 'Test Provider', 'v1', '1.0', 0, 0)",
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
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, auto_accept_rentals) VALUES (?, 'Test Provider', 'v1', '1.0', 0, 1)",
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
