use super::*;
use sqlx::SqlitePool;

async fn setup_test_db() -> Database {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    let migration_sql = include_str!("../../../migrations/001_original_schema.sql");
    sqlx::query(migration_sql).execute(&pool).await.unwrap();
    Database { pool }
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

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
            .bind(&contract_id).bind(&user_pk).bind(&provider_pk).execute(&db.pool).await.unwrap();

    let contracts = db.get_user_contracts(&user_pk).await.unwrap();
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].contract_id, contract_id);
}

#[tokio::test]
async fn test_get_provider_contracts() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let contract_id = vec![3u8; 32];

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
            .bind(&contract_id).bind(&user_pk).bind(&provider_pk).execute(&db.pool).await.unwrap();

    let contracts = db.get_provider_contracts(&provider_pk).await.unwrap();
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].provider_pubkey, provider_pk);
}

#[tokio::test]
async fn test_get_pending_provider_contracts() {
    let db = setup_test_db().await;
    let provider_pk = vec![2u8; 32];

    // Insert contracts with different statuses
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'requested')")
            .bind(vec![1u8; 32]).bind(vec![1u8; 32]).bind(&provider_pk).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 500, 'pending')")
            .bind(vec![2u8; 32]).bind(vec![1u8; 32]).bind(&provider_pk).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 1000, 'active')")
            .bind(vec![3u8; 32]).bind(vec![1u8; 32]).bind(&provider_pk).execute(&db.pool).await.unwrap();

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

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
            .bind(&contract_id).bind(vec![1u8; 32]).bind(vec![2u8; 32]).execute(&db.pool).await.unwrap();

    let contract = db.get_contract(&contract_id).await.unwrap();
    assert!(contract.is_some());
    assert_eq!(contract.unwrap().contract_id, contract_id);
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
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
            .bind(&contract_id).bind(vec![1u8; 32]).bind(vec![2u8; 32]).execute(&db.pool).await.unwrap();

    sqlx::query("INSERT INTO contract_sign_replies (contract_id, provider_pubkey, reply_status, reply_memo, instance_details, created_at_ns) VALUES (?, ?, 'accepted', 'ok', 'details', 0)")
            .bind(&contract_id).bind(vec![2u8; 32]).execute(&db.pool).await.unwrap();

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
    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
            .bind(&contract_id).bind(vec![1u8; 32]).bind(vec![2u8; 32]).execute(&db.pool).await.unwrap();

    sqlx::query("INSERT INTO contract_payment_entries (contract_id, pricing_model, time_period_unit, quantity, amount_e9s) VALUES (?, 'fixed', 'month', 1, 1000)")
            .bind(&contract_id).execute(&db.pool).await.unwrap();
    sqlx::query("INSERT INTO contract_payment_entries (contract_id, pricing_model, time_period_unit, quantity, amount_e9s) VALUES (?, 'usage', 'hour', 10, 500)")
            .bind(&contract_id).execute(&db.pool).await.unwrap();

    let payments = db.get_contract_payments(&contract_id).await.unwrap();
    assert_eq!(payments.len(), 2);
    assert_eq!(payments[0].amount_e9s, 1000);
}

#[tokio::test]
async fn test_list_contracts_pagination() {
    let db = setup_test_db().await;

    for i in 0..5 {
        sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', ?, 'pending')")
                .bind(vec![i as u8; 32]).bind(vec![1u8; 32]).bind(vec![2u8; 32]).bind(i * 1000).execute(&db.pool).await.unwrap();
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
    let offering_id = sqlx::query_scalar::<_, i64>("INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-rental-1', 'Test Server', 'USD', 100.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id")
            .bind(&provider_pk).fetch_one(&db.pool).await.unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-rsa AAAAB3...".to_string()),
        contact_method: Some("email:test@example.com".to_string()),
        request_memo: Some("Test rental".to_string()),
        duration_hours: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    assert_eq!(contract_id.len(), 32);

    // Verify contract was created
    let contract = db.get_contract(&contract_id).await.unwrap();
    assert!(contract.is_some());
    let contract = contract.unwrap();
    assert_eq!(contract.requester_pubkey, user_pk);
    assert_eq!(contract.provider_pubkey, provider_pk);
    assert_eq!(contract.offering_id, "off-rental-1");
    assert_eq!(contract.status, "requested");
    assert_eq!(contract.requester_ssh_pubkey, "ssh-rsa AAAAB3...");
    assert_eq!(contract.requester_contact, "email:test@example.com");
    assert_eq!(contract.request_memo, "Test rental");
    assert_eq!(contract.payment_amount_e9s, 100_000_000_000);
}

#[tokio::test]
async fn test_create_rental_request_with_defaults() {
    let db = setup_test_db().await;
    let user_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    // Create user registration first
    sqlx::query(
        "INSERT INTO user_registrations (pubkey, signature, created_at_ns) VALUES (?, ?, 0)",
    )
    .bind(&user_pk)
    .bind(&user_pk)
    .execute(&db.pool)
    .await
    .unwrap();

    // Create offering (no explicit id)
    let offering_id = sqlx::query_scalar::<_, i64>("INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-rental-2', 'Test Server', 'USD', 50.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id")
            .bind(&provider_pk).fetch_one(&db.pool).await.unwrap();

    // Create user SSH key
    sqlx::query("INSERT INTO user_public_keys (user_pubkey, key_type, key_data, created_at_ns) VALUES (?, 'ssh-ed25519', 'AAAAC3...user-key', 0)")
            .bind(&user_pk).execute(&db.pool).await.unwrap();

    // Create user contact
    sqlx::query("INSERT INTO user_contacts (user_pubkey, contact_type, contact_value, verified, created_at_ns) VALUES (?, 'email', 'user@example.com', 1, 0)")
            .bind(&user_pk).execute(&db.pool).await.unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: None,
        contact_method: None,
        request_memo: None,
        duration_hours: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();

    // Verify defaults were used
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();
    assert_eq!(contract.requester_ssh_pubkey, "AAAAC3...user-key");
    assert_eq!(contract.requester_contact, "email:user@example.com");
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
    let offering_id = sqlx::query_scalar::<_, i64>("INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns) VALUES (?, 'off-rental-3', 'Expensive Server', 'USD', 499.99, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'NYC', 0, 0) RETURNING id")
            .bind(&provider_pk).fetch_one(&db.pool).await.unwrap();

    let params = RentalRequestParams {
        offering_db_id: offering_id,
        ssh_pubkey: Some("ssh-key".to_string()),
        contact_method: Some("contact".to_string()),
        request_memo: None,
        duration_hours: None,
    };

    let contract_id = db.create_rental_request(&user_pk, params).await.unwrap();
    let contract = db.get_contract(&contract_id).await.unwrap().unwrap();

    // 499.99 * 1_000_000_000 = 499_990_000_000
    assert_eq!(contract.payment_amount_e9s, 499_990_000_000);
}

#[tokio::test]
async fn test_update_contract_status_records_history() {
    let db = setup_test_db().await;
    let contract_id = vec![9u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind(&provider_pk)
        .execute(&db.pool)
        .await
        .unwrap();

    db.update_contract_status(&contract_id, "accepted", &provider_pk, Some("all good"))
        .await
        .unwrap();

    let status: String =
        sqlx::query_scalar("SELECT status FROM contract_sign_requests WHERE contract_id = ?")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(status, "accepted");

    let history = sqlx::query_as::<_, (String, String, Option<String>)>("SELECT old_status, new_status, change_memo FROM contract_status_history WHERE contract_id = ? ORDER BY changed_at_ns DESC LIMIT 1")
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(history.0, "pending");
    assert_eq!(history.1, "accepted");
    assert_eq!(history.2.as_deref(), Some("all good"));
}

#[tokio::test]
async fn test_update_contract_status_rejects_non_provider() {
    let db = setup_test_db().await;
    let contract_id = vec![5u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];
    let attacker_pk = vec![3u8; 32];

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-2', 1000, 'memo', 0, 'requested')")
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind(&provider_pk)
        .execute(&db.pool)
        .await
        .unwrap();

    let result = db
        .update_contract_status(&contract_id, "accepted", &attacker_pk, None)
        .await;
    assert!(result.is_err());

    let history_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM contract_status_history WHERE contract_id = ?")
            .bind(&contract_id)
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

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-3', 1000, 'memo', 0, 'accepted')")
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind(&provider_pk)
        .execute(&db.pool)
        .await
        .unwrap();

    db.add_provisioning_details(&contract_id, "ip:1.2.3.4\nuser:root")
        .await
        .unwrap();

    let provisioning: Option<String> = sqlx::query_scalar(
        "SELECT provisioning_instance_details FROM contract_sign_requests WHERE contract_id = ?",
    )
    .bind(&contract_id)
    .fetch_one(&db.pool)
    .await
    .unwrap();
    assert_eq!(provisioning.as_deref(), Some("ip:1.2.3.4\nuser:root"));

    let detail_row = sqlx::query_as::<_, (Vec<u8>, Option<String>, Option<String>, Option<String>, i64)>("SELECT contract_id, instance_ip, instance_credentials, connection_instructions, provisioned_at_ns FROM contract_provisioning_details WHERE contract_id = ?")
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(detail_row.0, contract_id);
    assert_eq!(detail_row.1, None);
    assert_eq!(detail_row.2, None);
    assert_eq!(detail_row.3.as_deref(), Some("ip:1.2.3.4\nuser:root"));
    assert!(detail_row.4 > 0);
}

#[tokio::test]
async fn test_cancel_contract_success_requested() {
    let db = setup_test_db().await;
    let contract_id = vec![10u8; 32];
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'requested')")
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind(&provider_pk)
        .execute(&db.pool)
        .await
        .unwrap();

    db.cancel_contract(
        &contract_id,
        &requester_pk,
        Some("User requested cancellation"),
    )
    .await
    .unwrap();

    let status: String =
        sqlx::query_scalar("SELECT status FROM contract_sign_requests WHERE contract_id = ?")
            .bind(&contract_id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
    assert_eq!(status, "cancelled");

    let history = sqlx::query_as::<_, (String, String, Option<String>)>("SELECT old_status, new_status, change_memo FROM contract_status_history WHERE contract_id = ? ORDER BY changed_at_ns DESC LIMIT 1")
        .bind(&contract_id)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(history.0, "requested");
    assert_eq!(history.1, "cancelled");
    assert_eq!(history.2.as_deref(), Some("User requested cancellation"));
}

#[tokio::test]
async fn test_cancel_contract_success_all_cancellable_statuses() {
    let db = setup_test_db().await;
    let requester_pk = vec![1u8; 32];
    let provider_pk = vec![2u8; 32];

    let cancellable_statuses = ["requested", "pending", "accepted", "provisioning"];

    for (i, status) in cancellable_statuses.iter().enumerate() {
        let contract_id = vec![10 + i as u8; 32];

        sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, ?)")
            .bind(&contract_id)
            .bind(&requester_pk)
            .bind(&provider_pk)
            .bind(status)
            .execute(&db.pool)
            .await
            .unwrap();

        let result = db.cancel_contract(&contract_id, &requester_pk, None).await;
        assert!(
            result.is_ok(),
            "Cancellation should succeed for status '{}', but got error: {:?}",
            status,
            result.err()
        );

        let new_status: String =
            sqlx::query_scalar("SELECT status FROM contract_sign_requests WHERE contract_id = ?")
                .bind(&contract_id)
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

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'requested')")
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind(&provider_pk)
        .execute(&db.pool)
        .await
        .unwrap();

    let result = db.cancel_contract(&contract_id, &attacker_pk, None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("only the requester can cancel"));

    let status: String =
        sqlx::query_scalar("SELECT status FROM contract_sign_requests WHERE contract_id = ?")
            .bind(&contract_id)
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

    sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, 'pending')")
        .bind(&contract_id)
        .bind(&requester_pk)
        .bind(&provider_pk)
        .execute(&db.pool)
        .await
        .unwrap();

    let result = db.cancel_contract(&contract_id, &provider_pk, None).await;
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

        sqlx::query("INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status) VALUES (?, ?, 'ssh-key', 'contact', ?, 'off-1', 1000, 'memo', 0, ?)")
            .bind(&contract_id)
            .bind(&requester_pk)
            .bind(&provider_pk)
            .bind(status)
            .execute(&db.pool)
            .await
            .unwrap();

        let result = db.cancel_contract(&contract_id, &requester_pk, None).await;
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
        .cancel_contract(&nonexistent_id, &requester_pk, None)
        .await;
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("Contract not found"));
    assert!(error_msg.contains(&hex::encode(&nonexistent_id)));
}
