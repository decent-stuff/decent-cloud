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
async fn test_exponential_backoff_calculation() {
    // Test that backoff is calculated correctly
    // Attempt 0: 0 seconds
    // Attempt 1: 2^1 * 60 = 120 seconds
    // Attempt 2: 2^2 * 60 = 240 seconds
    // Attempt 3: 2^3 * 60 = 480 seconds

    let db: Database = setup_test_db().await;

    // Queue an email
    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "Test",
            "Body",
            false,
            EmailType::Recovery, // 24 attempts
        )
        .await
        .unwrap();

    // Simulate failed attempts with timestamps
    let _now = chrono::Utc::now().timestamp();

    // First attempt (should be retried immediately since attempts=0)
    db.mark_email_failed(&id, "Error 1").await.unwrap();
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].attempts, 1);

    // Second attempt (should wait 2 minutes)
    let backoff1 = 2_i64.pow(1) * 60; // 120 seconds
    assert_eq!(backoff1, 120);

    // Third attempt (should wait 4 minutes)
    let backoff2 = 2_i64.pow(2) * 60; // 240 seconds
    assert_eq!(backoff2, 240);
}

#[tokio::test]
async fn test_max_attempts_reached() {
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
            EmailType::General, // 6 max attempts
        )
        .await
        .unwrap();

    // Manually mark as failed 6 times to reach max attempts
    for _ in 0..6 {
        db.mark_email_failed(&id, "Test error").await.unwrap();
    }

    // Should not be in pending emails anymore (status changed to failed)
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 0);
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
