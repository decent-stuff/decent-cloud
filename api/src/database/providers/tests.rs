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
        "SELECT name, domain, website_url, data_source FROM external_providers WHERE pubkey = ?",
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
        "SELECT name, domain, website_url, data_source FROM external_providers WHERE pubkey = ?",
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
        "INSERT INTO provider_profiles (pubkey, name, api_version, profile_version, updated_at_ns) VALUES (?, ?, ?, ?, ?)",
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
