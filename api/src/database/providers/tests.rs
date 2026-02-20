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
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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
    let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
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
