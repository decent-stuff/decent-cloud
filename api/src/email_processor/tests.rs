use super::*;
use crate::database::email::EmailType;
use crate::database::test_helpers::setup_test_db;
use crate::database::Database;
use crate::email_service::EmailService;

#[tokio::test]
async fn test_email_processor_creation() {
    let db = setup_test_db().await;
    let email_service = Arc::new(EmailService::new("test-key".to_string(), None, None, None));

    let processor = EmailProcessor::new(Arc::new(db), email_service, 30, 10);
    assert_eq!(processor.batch_size, 10);
}

#[tokio::test]
async fn test_process_batch_empty() {
    let db = setup_test_db().await;
    let email_service = Arc::new(EmailService::new("test-key".to_string(), None, None, None));

    let processor = EmailProcessor::new(Arc::new(db), email_service, 30, 10);
    let result = processor.process_batch().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_backoff_calculation() {
    use crate::database::email::calculate_backoff_secs;

    // Test the new backoff schedule:
    // immediate, 1min, 2min, 4min, 8min, 16min, 32min, then 1h
    assert_eq!(calculate_backoff_secs(0), 0); // immediate
    assert_eq!(calculate_backoff_secs(1), 60); // 1 min
    assert_eq!(calculate_backoff_secs(2), 120); // 2 min
    assert_eq!(calculate_backoff_secs(3), 240); // 4 min
    assert_eq!(calculate_backoff_secs(4), 480); // 8 min
    assert_eq!(calculate_backoff_secs(5), 960); // 16 min
    assert_eq!(calculate_backoff_secs(6), 1920); // 32 min
    assert_eq!(calculate_backoff_secs(7), 3600); // 1 hour
    assert_eq!(calculate_backoff_secs(100), 3600); // 1 hour (capped)

    let db: Database = setup_test_db().await;

    // Queue an email
    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "Test",
            "Body",
            false,
            EmailType::Recovery,
        )
        .await
        .unwrap();

    // First attempt (should be retried immediately since attempts=0)
    db.mark_email_failed(&id, "Error 1").await.unwrap();
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].attempts, 1);
}

#[tokio::test]
async fn test_permanent_failure() {
    let db: Arc<Database> = Arc::new(setup_test_db().await);
    let _email_service = Arc::new(EmailService::new(
        "invalid-key".to_string(),
        None,
        None,
        None,
    ));

    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "Test",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    // Mark email as permanently failed (simulates 7-day expiration)
    db.mark_email_permanently_failed(&id).await.unwrap();

    // Should not be in pending emails anymore (status changed to failed)
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 0);

    // Should be in failed emails
    let failed = db.get_failed_emails(10).await.unwrap();
    assert_eq!(failed.len(), 1);
}

#[tokio::test]
async fn test_batch_size_limit() {
    let db: Arc<Database> = Arc::new(setup_test_db().await);
    let email_service = Arc::new(EmailService::new("test-key".to_string(), None, None, None));

    // Queue 5 emails
    for i in 0..5 {
        db.queue_email(
            &format!("test{}@example.com", i),
            "sender@example.com",
            "Test",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();
    }

    // Create processor with batch size of 3
    let processor = EmailProcessor::new(db.clone(), email_service, 30, 3);

    // Get pending should respect batch size
    let pending = db.get_pending_emails(processor.batch_size).await.unwrap();
    assert_eq!(pending.len(), 3);
}
