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
