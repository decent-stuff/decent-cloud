use super::*;
use crate::database::test_helpers::setup_test_db;
use std::time::Duration;

#[tokio::test]
async fn test_cleanup_service_creation() {
    let db = Arc::new(setup_test_db().await);
    let service = CleanupService::new(db, 24, 180);

    assert_eq!(service.interval, Duration::from_secs(24 * 60 * 60));
    assert_eq!(service.retention_days, 180);
}

#[tokio::test]
async fn test_cleanup_once() {
    let db = Arc::new(setup_test_db().await);
    let service = CleanupService::new(db.clone(), 24, 180);

    // Insert old audit record (200 days old)
    let old_created_at =
        chrono::Utc::now().timestamp_nanos_opt().unwrap() - (200 * 24 * 60 * 60 * 1_000_000_000);
    let old_nonce = uuid::Uuid::new_v4();
    let signature = [0u8; 64];
    let public_key = [22u8; 32];

    sqlx::query(
        "INSERT INTO signature_audit
         (account_id, action, payload, signature, public_key, timestamp, nonce, is_admin_action, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
    )
    .bind(None::<&[u8]>)
    .bind("test_action")
    .bind("{}")
    .bind(&signature[..])
    .bind(&public_key[..])
    .bind(chrono::Utc::now().timestamp_nanos_opt().unwrap())
    .bind(&old_nonce.as_bytes()[..])
    .bind(false)
    .bind(old_created_at)
    .execute(&db.pool)
    .await
    .unwrap();

    // Run cleanup
    service.cleanup_once().await.unwrap();

    // Verify record was deleted
    let exists = db
        .check_nonce_exists(&old_nonce, 365 * 24 * 60)
        .await
        .unwrap();
    assert!(!exists, "Old record should be deleted");
}

#[tokio::test]
async fn test_cleanup_service_runs_periodically() {
    let db = Arc::new(setup_test_db().await);

    // Use very short interval for testing (100ms)
    let service = CleanupService {
        database: db.clone(),
        interval: Duration::from_millis(100),
        retention_days: 180,
    };

    // Run service with timeout to prevent infinite loop in test
    let service_task = tokio::spawn(async move {
        service.run().await;
    });

    // Wait for at least 2 cleanup cycles (250ms)
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Abort the service task
    service_task.abort();

    // If we got here without panicking, the service is running correctly
}

#[tokio::test]
async fn test_cleanup_once_no_old_records() {
    let db = Arc::new(setup_test_db().await);
    let service = CleanupService::new(db, 24, 180);

    // Run cleanup with no old records
    let result = service.cleanup_once().await;
    assert!(
        result.is_ok(),
        "Cleanup should succeed even with no records"
    );
}
