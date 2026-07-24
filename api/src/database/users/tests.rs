use super::*;

#[test]
fn export_typescript_types() {
    UserActivity::export().expect("Failed to export UserActivity type");
    PublicUserActivity::export().expect("Failed to export PublicUserActivity type");
    PublicContractSummary::export().expect("Failed to export PublicContractSummary type");
    AccountContact::export().expect("Failed to export AccountContact type");
    AccountSocial::export().expect("Failed to export AccountSocial type");
    AccountExternalKey::export().expect("Failed to export AccountExternalKey type");
    OfferingStatsWeek::export().expect("Failed to export OfferingStatsWeek type");
}

#[tokio::test]
async fn test_public_user_activity_strips_sensitive_contract_fields() {
    let db = crate::database::test_helpers::setup_test_db().await;

    // Distinct, recognizable pubkeys so this test can't collide with others.
    let requester_pk = vec![0xa1u8; 32];
    let provider_pk = vec![0xb2u8; 32];
    let contract_id = vec![0xc3u8; 32];

    // Insert a contract carrying sensitive data: an SSH key, gateway config, and
    // a payment amount. The public summary must expose NONE of these.
    // (Plain `sqlx::query` — runtime-checked — so this test-only INSERT doesn't
    // require regenerating the production sqlx offline cache.)
    sqlx::query(
        r#"INSERT INTO contract_sign_requests
           (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact,
            provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns,
            payment_method, payment_status, currency, duration_hours, status,
            gateway_subdomain, gateway_ssh_port)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)"#,
    )
    .bind(contract_id.as_slice())
    .bind(requester_pk.as_slice())
    .bind("ssh-ed25519 AAAASECRETKEY")
    .bind("email:secret@example.com")
    .bind(provider_pk.as_slice())
    .bind("offer-42")
    .bind(9_000_000_000i64)
    .bind("test")
    .bind(1_700_000_000_000_000_000i64)
    .bind("stripe")
    .bind("succeeded")
    .bind("USD")
    .bind(24i64)
    .bind("active")
    .bind("k7m2p4.dc-lk.dev-gw.example.org")
    .bind(20001i32)
    .execute(&db.pool)
    .await
    .unwrap();

    let activity = db.get_public_user_activity(&requester_pk).await.unwrap();

    // The requester sees their rental, summarized.
    assert_eq!(activity.rentals_as_requester.len(), 1);
    let summary = &activity.rentals_as_requester[0];
    assert_eq!(summary.status, "active");
    assert_eq!(summary.offering_id, "offer-42");
    assert_eq!(summary.duration_hours, Some(24));

    // Serialize and confirm ONLY non-sensitive fields are present.
    let json = serde_json::to_value(summary).unwrap();
    for key in &[
        "payment_amount_e9s",
        "currency",
        "requester_ssh_pubkey",
        "requester_contact",
        "gateway_subdomain",
        "gateway_ssh_port",
        "stripe_checkout_session_id",
        "instance_config",
    ] {
        assert!(
            json.get(key).is_none(),
            "public contract summary must not expose sensitive field `{key}`"
        );
    }
    // And the expected public fields ARE present.
    for key in &["contract_id", "offering_id", "status", "created_at_ns", "provider_pubkey"] {
        assert!(json.get(key).is_some(), "public summary missing field `{key}`");
    }
}

#[test]
fn test_offering_stats_week_serializes_camelcase() {
    let row = OfferingStatsWeek {
        week_start: "2024-01-08".to_string(),
        offering_id: "pool-small".to_string(),
        total_requests: 5,
        active_count: 2,
        revenue_e9s: 3_000_000_000,
    };
    let json = serde_json::to_value(&row).unwrap();
    assert_eq!(json["weekStart"], "2024-01-08");
    assert_eq!(json["offeringId"], "pool-small");
    assert_eq!(json["totalRequests"], 5_i64);
    assert_eq!(json["activeCount"], 2_i64);
    assert_eq!(json["revenueE9s"], 3_000_000_000_i64);
    // Ensure no snake_case keys leaked
    assert!(json.get("week_start").is_none());
    assert!(json.get("offering_id").is_none());
}
