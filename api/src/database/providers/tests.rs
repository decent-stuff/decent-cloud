use super::*;
use crate::database::test_helpers::setup_test_db;

#[test]
fn export_typescript_types() {
    ProviderProfile::export().expect("Failed to export ProviderProfile type");
    Validator::export().expect("Failed to export Validator type");
}

#[tokio::test]
async fn test_create_external_provider() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    db.create_or_update_external_provider(
        &pubkey,
        "Hetzner",
        "hetzner.com",
        "https://www.hetzner.com",
        "scraper",
    )
    .await
    .unwrap();

    // Verify record exists
    let result = sqlx::query!(
        "SELECT name, domain, website_url, data_source FROM external_providers WHERE pubkey = $1",
        pubkey
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(result.name, "Hetzner");
    assert_eq!(result.domain, "hetzner.com");
    assert_eq!(result.website_url, "https://www.hetzner.com");
    assert_eq!(result.data_source, "scraper");
}

#[tokio::test]
async fn test_update_external_provider() {
    let db = setup_test_db().await;
    let pubkey = vec![1u8; 32];

    // Create initial record
    db.create_or_update_external_provider(
        &pubkey,
        "Hetzner",
        "hetzner.com",
        "https://www.hetzner.com",
        "scraper",
    )
    .await
    .unwrap();

    // Update with new data
    db.create_or_update_external_provider(
        &pubkey,
        "Hetzner Updated",
        "hetzner.de",
        "https://www.hetzner.de",
        "manual_curation",
    )
    .await
    .unwrap();

    // Verify update
    let result = sqlx::query!(
        "SELECT name, domain, website_url, data_source FROM external_providers WHERE pubkey = $1",
        pubkey
    )
    .fetch_one(&db.pool)
    .await
    .unwrap();

    assert_eq!(result.name, "Hetzner Updated");
    assert_eq!(result.domain, "hetzner.de");
    assert_eq!(result.website_url, "https://www.hetzner.de");
    assert_eq!(result.data_source, "manual_curation");
}

#[tokio::test]
async fn test_external_provider_unique_domain() {
    let db = setup_test_db().await;
    let pubkey1 = vec![1u8; 32];
    let pubkey2 = vec![2u8; 32];

    // Create first provider with domain
    db.create_or_update_external_provider(
        &pubkey1,
        "Hetzner",
        "hetzner.com",
        "https://www.hetzner.com",
        "scraper",
    )
    .await
    .unwrap();

    // Try to create another provider with same domain (should fail due to UNIQUE constraint)
    let result = db
        .create_or_update_external_provider(
            &pubkey2,
            "Other Provider",
            "hetzner.com",
            "https://other.com",
            "scraper",
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_provider_onboarding_creates_new_profile() {
    let db = setup_test_db().await;
    let pubkey = vec![42u8; 32];

    // Verify no profile exists initially
    assert!(db.get_provider_profile(&pubkey).await.unwrap().is_none());

    // Create onboarding data
    let onboarding = ProviderOnboarding {
        support_email: Some("test@example.com".to_string()),
        support_hours: Some("24/7".to_string()),
        support_channels: Some(r#"["Email","Chat"]"#.to_string()),
        regions: Some(r#"["Europe"]"#.to_string()),
        payment_methods: Some(r#"["Crypto"]"#.to_string()),
        refund_policy: Some("30-day".to_string()),
        sla_guarantee: Some("99.9%".to_string()),
        unique_selling_points: None,
        common_issues: None,
        onboarding_completed_at: None,
    };

    // Call update_provider_onboarding - should create new profile
    db.update_provider_onboarding(&pubkey, &onboarding, "New Provider")
        .await
        .unwrap();

    // Verify profile was created with correct name
    let profile = db.get_provider_profile(&pubkey).await.unwrap().unwrap();
    assert_eq!(profile.name, "New Provider");
    assert_eq!(profile.support_email, Some("test@example.com".to_string()));
    assert_eq!(profile.support_hours, Some("24/7".to_string()));
}

#[tokio::test]
async fn test_update_provider_onboarding_updates_existing_profile() {
    let db = setup_test_db().await;
    let pubkey = vec![43u8; 32];

    // Create initial profile directly in DB
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().expect("timestamp overflow (year > 2262)");
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&pubkey)
    .bind("Existing Provider")
    .bind("v1")
    .bind("1.0")
    .bind(now_ns)
    .execute(&db.pool)
    .await
    .unwrap();

    // Verify existing profile
    let profile = db.get_provider_profile(&pubkey).await.unwrap().unwrap();
    assert_eq!(profile.name, "Existing Provider");
    assert!(profile.support_email.is_none());

    // Update onboarding data
    let onboarding = ProviderOnboarding {
        support_email: Some("updated@example.com".to_string()),
        support_hours: Some("Business hours".to_string()),
        support_channels: Some(r#"["Phone"]"#.to_string()),
        regions: Some(r#"["US"]"#.to_string()),
        payment_methods: Some(r#"["Card"]"#.to_string()),
        refund_policy: None,
        sla_guarantee: None,
        unique_selling_points: None,
        common_issues: None,
        onboarding_completed_at: None,
    };

    // Update should NOT change the name (ON CONFLICT DO UPDATE doesn't touch name)
    db.update_provider_onboarding(&pubkey, &onboarding, "Ignored Name")
        .await
        .unwrap();

    // Verify onboarding data was updated but name preserved
    let profile = db.get_provider_profile(&pubkey).await.unwrap().unwrap();
    assert_eq!(profile.name, "Existing Provider"); // Name preserved
    assert_eq!(
        profile.support_email,
        Some("updated@example.com".to_string())
    );
    assert_eq!(profile.support_hours, Some("Business hours".to_string()));
}

async fn insert_provider_profile(db: &super::Database, pubkey: &[u8]) {
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().expect("timestamp overflow (year > 2262)");
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(pubkey)
    .bind("Test Provider")
    .bind("v1")
    .bind("1.0")
    .bind(now_ns)
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_add_and_get_provider_contacts() {
    let db = setup_test_db().await;
    let pubkey = vec![50u8; 32];
    insert_provider_profile(&db, &pubkey).await;

    // Initially empty
    let contacts = db.get_provider_contacts(&pubkey).await.unwrap();
    assert!(contacts.is_empty());

    // Add two contacts
    db.add_provider_contact(&pubkey, "telegram", "@alice")
        .await
        .unwrap();
    db.add_provider_contact(&pubkey, "phone", "+1234567890")
        .await
        .unwrap();

    let contacts = db.get_provider_contacts(&pubkey).await.unwrap();
    assert_eq!(contacts.len(), 2);

    // Verify each contact has a valid id and correct fields
    let telegram = contacts
        .iter()
        .find(|c| c.contact_type == "telegram")
        .unwrap();
    assert_eq!(telegram.contact_value, "@alice");
    assert!(telegram.id > 0);

    let phone = contacts
        .iter()
        .find(|c| c.contact_type == "phone")
        .unwrap();
    assert_eq!(phone.contact_value, "+1234567890");
    assert!(phone.id > 0);
}

#[tokio::test]
async fn test_delete_provider_contact() {
    let db = setup_test_db().await;
    let pubkey = vec![51u8; 32];
    insert_provider_profile(&db, &pubkey).await;

    db.add_provider_contact(&pubkey, "telegram", "@bob")
        .await
        .unwrap();
    db.add_provider_contact(&pubkey, "phone", "+9876543210")
        .await
        .unwrap();

    let contacts = db.get_provider_contacts(&pubkey).await.unwrap();
    assert_eq!(contacts.len(), 2);
    let telegram_id = contacts
        .iter()
        .find(|c| c.contact_type == "telegram")
        .unwrap()
        .id;

    // Delete the telegram contact
    db.delete_provider_contact(&pubkey, telegram_id)
        .await
        .unwrap();

    let contacts = db.get_provider_contacts(&pubkey).await.unwrap();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].contact_type, "phone");
}

#[tokio::test]
async fn test_delete_provider_contact_wrong_pubkey_is_noop() {
    let db = setup_test_db().await;
    let pubkey = vec![52u8; 32];
    let other_pubkey = vec![53u8; 32];
    insert_provider_profile(&db, &pubkey).await;
    insert_provider_profile(&db, &other_pubkey).await;

    db.add_provider_contact(&pubkey, "telegram", "@carol")
        .await
        .unwrap();

    let contacts = db.get_provider_contacts(&pubkey).await.unwrap();
    let contact_id = contacts[0].id;

    // Delete with wrong pubkey — should be a no-op (no rows deleted, no error)
    db.delete_provider_contact(&other_pubkey, contact_id)
        .await
        .unwrap();

    // Contact should still exist for the correct provider
    let contacts = db.get_provider_contacts(&pubkey).await.unwrap();
    assert_eq!(contacts.len(), 1);
    assert_eq!(contacts[0].contact_value, "@carol");
}

/// Insert a provider profile with a recent created_at (within the last 90 days).
async fn insert_new_provider_with_offering(db: &super::Database, pubkey: &[u8], name: &str) {
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().expect("timestamp overflow (year > 2262)");
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, created_at, has_critical_flags) VALUES ($1, $2, $3, $4, $5, NOW(), FALSE)",
    )
    .bind(pubkey)
    .bind(name)
    .bind("v1")
    .bind("1.0")
    .bind(now_ns)
    .execute(&db.pool)
    .await
    .unwrap();

    let offering_id = hex::encode(pubkey);
    sqlx::query(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns, is_draft) VALUES ($1, $2, 'Test Offer', 'USD', 10.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 0, FALSE)",
    )
    .bind(pubkey)
    .bind(offering_id)
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_get_new_providers_returns_recent_with_offerings() {
    let db = setup_test_db().await;
    let pubkey1 = vec![60u8; 32];
    let pubkey2 = vec![61u8; 32];

    insert_new_provider_with_offering(&db, &pubkey1, "New Provider A").await;
    insert_new_provider_with_offering(&db, &pubkey2, "New Provider B").await;

    let results = db.get_new_providers(10).await.unwrap();
    let names: Vec<&str> = results.iter().map(|p| p.name.as_str()).collect();

    assert!(names.contains(&"New Provider A"), "Expected 'New Provider A' in results");
    assert!(names.contains(&"New Provider B"), "Expected 'New Provider B' in results");
    // Each result must have at least 1 offering
    for p in &results {
        assert!(p.offerings_count > 0, "Provider {} should have offerings", p.name);
        assert!(p.joined_days_ago >= 0, "joined_days_ago must be non-negative");
    }
}

#[tokio::test]
async fn test_get_new_providers_excludes_old_providers() {
    let db = setup_test_db().await;
    let pubkey = vec![62u8; 32];
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().expect("timestamp overflow (year > 2262)");

    // Insert provider with created_at > 90 days ago — should be excluded
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, created_at, has_critical_flags) VALUES ($1, $2, $3, $4, $5, NOW() - INTERVAL '100 days', FALSE)",
    )
    .bind(&pubkey)
    .bind("Old Provider")
    .bind("v1")
    .bind("1.0")
    .bind(now_ns)
    .execute(&db.pool)
    .await
    .unwrap();

    let offering_id = hex::encode(&pubkey);
    sqlx::query(
        "INSERT INTO provider_offerings (pubkey, offering_id, offer_name, currency, monthly_price, setup_fee, visibility, product_type, billing_interval, stock_status, datacenter_country, datacenter_city, unmetered_bandwidth, created_at_ns, is_draft) VALUES ($1, $2, 'Old Offer', 'USD', 10.0, 0, 'public', 'compute', 'monthly', 'in_stock', 'US', 'City', FALSE, 0, FALSE)",
    )
    .bind(&pubkey)
    .bind(offering_id)
    .execute(&db.pool)
    .await
    .unwrap();

    let results = db.get_new_providers(10).await.unwrap();
    assert!(
        results.iter().all(|p| p.name != "Old Provider"),
        "Old provider must not appear in new providers list"
    );
}

#[tokio::test]
async fn test_get_new_providers_excludes_providers_without_public_offerings() {
    let db = setup_test_db().await;
    let pubkey = vec![63u8; 32];
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().expect("timestamp overflow (year > 2262)");

    // Insert provider with no offerings
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns, created_at, has_critical_flags) VALUES ($1, $2, $3, $4, $5, NOW(), FALSE)",
    )
    .bind(&pubkey)
    .bind("No-Offering Provider")
    .bind("v1")
    .bind("1.0")
    .bind(now_ns)
    .execute(&db.pool)
    .await
    .unwrap();

    let results = db.get_new_providers(10).await.unwrap();
    assert!(
        results.iter().all(|p| p.name != "No-Offering Provider"),
        "Provider with no offerings must not appear in new providers list"
    );
}

#[tokio::test]
async fn test_get_new_providers_respects_limit() {
    let db = setup_test_db().await;

    // Insert 3 new providers with offerings
    for i in 64u8..67 {
        insert_new_provider_with_offering(&db, &[i; 32], &format!("Limit Provider {i}")).await;
    }

    let results = db.get_new_providers(2).await.unwrap();
    // The limit is applied; we inserted 3 but asked for 2, so at most 2 returned
    assert!(results.len() <= 2, "Result count must not exceed requested limit");
}

#[test]
fn export_new_provider_typescript_type() {
    NewProvider::export().expect("Failed to export NewProvider type");
}

#[test]
fn export_auto_accept_rule_typescript_type() {
    AutoAcceptRule::export().expect("Failed to export AutoAcceptRule type");
}

// ── Auto-accept rules ────────────────────────────────────────────────────────

/// Insert a minimal provider profile sufficient for FK constraints.
async fn insert_provider(db: &super::Database, pubkey: &[u8]) {
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().expect("timestamp overflow (year > 2262)");
    sqlx::query(
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES ($1, $2, 'v1', '1.0', $3)",
    )
    .bind(pubkey)
    .bind("Test Provider")
    .bind(now_ns)
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_create_auto_accept_rule_and_list() {
    let db = setup_test_db().await;
    let pubkey = vec![70u8; 32];
    insert_provider(&db, &pubkey).await;

    let rule = db
        .create_auto_accept_rule(&pubkey, "offer-1", Some(24), Some(720))
        .await
        .unwrap();

    assert_eq!(rule.offering_id, "offer-1");
    assert_eq!(rule.min_duration_hours, Some(24));
    assert_eq!(rule.max_duration_hours, Some(720));
    assert!(rule.enabled);

    let rules = db.list_auto_accept_rules(&pubkey).await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].offering_id, "offer-1");
}

#[tokio::test]
async fn test_create_auto_accept_rule_invalid_range() {
    let db = setup_test_db().await;
    let pubkey = vec![71u8; 32];
    insert_provider(&db, &pubkey).await;

    let result = db
        .create_auto_accept_rule(&pubkey, "offer-1", Some(720), Some(24))
        .await;
    assert!(result.is_err(), "min > max must be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("must not exceed"), "Error: {msg}");
}

#[tokio::test]
async fn test_create_auto_accept_rule_duplicate_offering_fails() {
    let db = setup_test_db().await;
    let pubkey = vec![72u8; 32];
    insert_provider(&db, &pubkey).await;

    db.create_auto_accept_rule(&pubkey, "offer-dup", None, None)
        .await
        .unwrap();

    let result = db
        .create_auto_accept_rule(&pubkey, "offer-dup", None, None)
        .await;
    assert!(result.is_err(), "Duplicate (pubkey, offering_id) must be rejected");
}

#[tokio::test]
async fn test_update_auto_accept_rule() {
    let db = setup_test_db().await;
    let pubkey = vec![73u8; 32];
    insert_provider(&db, &pubkey).await;

    let rule = db
        .create_auto_accept_rule(&pubkey, "offer-upd", Some(10), Some(100))
        .await
        .unwrap();

    let updated = db
        .update_auto_accept_rule(&pubkey, rule.id, Some(5), Some(200), false)
        .await
        .unwrap();

    assert_eq!(updated.min_duration_hours, Some(5));
    assert_eq!(updated.max_duration_hours, Some(200));
    assert!(!updated.enabled);
}

#[tokio::test]
async fn test_update_auto_accept_rule_wrong_provider_fails() {
    let db = setup_test_db().await;
    let pubkey = vec![74u8; 32];
    let other = vec![75u8; 32];
    insert_provider(&db, &pubkey).await;

    let rule = db
        .create_auto_accept_rule(&pubkey, "offer-x", None, None)
        .await
        .unwrap();

    let result = db
        .update_auto_accept_rule(&other, rule.id, None, None, false)
        .await;
    assert!(result.is_err(), "Wrong provider must not update another's rule");
}

#[tokio::test]
async fn test_delete_auto_accept_rule() {
    let db = setup_test_db().await;
    let pubkey = vec![76u8; 32];
    insert_provider(&db, &pubkey).await;

    let rule = db
        .create_auto_accept_rule(&pubkey, "offer-del", None, None)
        .await
        .unwrap();

    db.delete_auto_accept_rule(&pubkey, rule.id)
        .await
        .unwrap();

    let rules = db.list_auto_accept_rules(&pubkey).await.unwrap();
    assert!(rules.is_empty());
}

#[tokio::test]
async fn test_delete_auto_accept_rule_not_found() {
    let db = setup_test_db().await;
    let pubkey = vec![77u8; 32];
    insert_provider(&db, &pubkey).await;

    let result = db.delete_auto_accept_rule(&pubkey, 999999).await;
    assert!(result.is_err(), "Deleting nonexistent rule must fail");
}

#[tokio::test]
async fn test_check_auto_accept_rule_no_rule_matches() {
    let db = setup_test_db().await;
    let pubkey = vec![78u8; 32];
    insert_provider(&db, &pubkey).await;

    // No rule for this offering → always true (backward compatible)
    let ok = db
        .check_auto_accept_rule_matches(&pubkey, "offer-none", Some(48))
        .await
        .unwrap();
    assert!(ok, "No rule should mean auto-accept (backward compatible)");
}

#[tokio::test]
async fn test_check_auto_accept_rule_within_range() {
    let db = setup_test_db().await;
    let pubkey = vec![79u8; 32];
    insert_provider(&db, &pubkey).await;

    db.create_auto_accept_rule(&pubkey, "offer-rng", Some(24), Some(720))
        .await
        .unwrap();

    // Within range
    assert!(db.check_auto_accept_rule_matches(&pubkey, "offer-rng", Some(48)).await.unwrap());
    // Exactly at min
    assert!(db.check_auto_accept_rule_matches(&pubkey, "offer-rng", Some(24)).await.unwrap());
    // Exactly at max
    assert!(db.check_auto_accept_rule_matches(&pubkey, "offer-rng", Some(720)).await.unwrap());
}

#[tokio::test]
async fn test_check_auto_accept_rule_outside_range() {
    let db = setup_test_db().await;
    let pubkey = vec![80u8; 32];
    insert_provider(&db, &pubkey).await;

    db.create_auto_accept_rule(&pubkey, "offer-out", Some(24), Some(720))
        .await
        .unwrap();

    // Below min
    assert!(!db.check_auto_accept_rule_matches(&pubkey, "offer-out", Some(1)).await.unwrap());
    // Above max
    assert!(!db.check_auto_accept_rule_matches(&pubkey, "offer-out", Some(1000)).await.unwrap());
    // No duration provided (defaults to 0 which is below min)
    assert!(!db.check_auto_accept_rule_matches(&pubkey, "offer-out", None).await.unwrap());
}

#[tokio::test]
async fn test_check_auto_accept_rule_disabled_never_matches() {
    let db = setup_test_db().await;
    let pubkey = vec![81u8; 32];
    insert_provider(&db, &pubkey).await;

    let rule = db
        .create_auto_accept_rule(&pubkey, "offer-dis", None, None)
        .await
        .unwrap();

    // Disable the rule
    db.update_auto_accept_rule(&pubkey, rule.id, None, None, false)
        .await
        .unwrap();

    assert!(!db.check_auto_accept_rule_matches(&pubkey, "offer-dis", Some(48)).await.unwrap(),
        "Disabled rule must never match");
}

#[tokio::test]
async fn test_check_auto_accept_rule_delete_reverts_to_accept_all() {
    let db = setup_test_db().await;
    let pubkey = vec![82u8; 32];
    insert_provider(&db, &pubkey).await;

    let rule = db
        .create_auto_accept_rule(&pubkey, "offer-rev", Some(100), Some(200))
        .await
        .unwrap();

    // Outside range → no match
    assert!(!db.check_auto_accept_rule_matches(&pubkey, "offer-rev", Some(50)).await.unwrap());

    // Delete the rule
    db.delete_auto_accept_rule(&pubkey, rule.id).await.unwrap();

    // After deletion, no rule → accept all
    assert!(db.check_auto_accept_rule_matches(&pubkey, "offer-rev", Some(50)).await.unwrap(),
        "After rule deletion, should revert to accept-all");
}
