use crate::database::email::EmailType;
use crate::database::test_helpers::setup_test_db;

#[tokio::test]
async fn test_queue_email() {
    let db = setup_test_db().await;

    let id = db
        .queue_email(
            "Test User <test@example.com>",
            "Sender <sender@example.com>",
            "Test Subject",
            "Test body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    assert_eq!(id.len(), 16);

    let emails = db.get_pending_emails(10).await.unwrap();
    assert_eq!(emails.len(), 1);
    let email = &emails[0];
    assert_eq!(email.to_addr, "Test User <test@example.com>");
    assert_eq!(email.subject, "Test Subject");
    assert_eq!(email.email_type, "general");
    assert_eq!(email.status, "pending");
    assert_eq!(email.attempts, 0);
    assert_eq!(email.max_attempts, 6);
    assert!(email.last_attempted_at.is_none());
}

#[tokio::test]
async fn test_queue_html_email() {
    let db = setup_test_db().await;

    db.queue_email(
        "test@example.com",
        "sender@example.com",
        "HTML Test",
        "<h1>HTML Body</h1>",
        true,
        EmailType::Welcome,
    )
    .await
    .unwrap();

    let emails = db.get_pending_emails(10).await.unwrap();
    assert_eq!(emails.len(), 1);
    let email = &emails[0];
    assert_eq!(email.is_html, 1);
    assert_eq!(email.email_type, "welcome");
    assert_eq!(email.max_attempts, 12);
    assert_eq!(email.body, "<h1>HTML Body</h1>");
}

#[tokio::test]
async fn test_get_pending_emails_empty() {
    let db = setup_test_db().await;
    let emails = db.get_pending_emails(10).await.unwrap();
    assert_eq!(emails.len(), 0);
}

#[tokio::test]
async fn test_get_pending_emails() {
    let db = setup_test_db().await;

    db.queue_email(
        "test1@example.com",
        "sender@example.com",
        "Subject 1",
        "Body 1",
        false,
        EmailType::General,
    )
    .await
    .unwrap();

    db.queue_email(
        "test2@example.com",
        "sender@example.com",
        "Subject 2",
        "Body 2",
        false,
        EmailType::General,
    )
    .await
    .unwrap();

    let emails = db.get_pending_emails(10).await.unwrap();
    assert_eq!(emails.len(), 2);
    assert_eq!(emails[0].to_addr, "test1@example.com");
    assert_eq!(emails[1].to_addr, "test2@example.com");
}

#[tokio::test]
async fn test_get_pending_emails_limit() {
    let db = setup_test_db().await;

    for i in 0..5 {
        db.queue_email(
            &format!("test{}@example.com", i),
            "sender@example.com",
            "Subject",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();
    }

    let emails = db.get_pending_emails(3).await.unwrap();
    assert_eq!(emails.len(), 3);
}

#[tokio::test]
async fn test_mark_email_sent() {
    let db = setup_test_db().await;

    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "Subject",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    db.mark_email_sent(&id).await.unwrap();

    // Verify email is no longer in pending queue
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[tokio::test]
async fn test_mark_email_failed() {
    let db = setup_test_db().await;

    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "Subject",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    db.mark_email_failed(&id, "Connection timeout")
        .await
        .unwrap();

    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 1);
    let email = &pending[0];
    assert_eq!(email.attempts, 1);
    assert_eq!(email.last_error.as_ref().unwrap(), "Connection timeout");
    assert_eq!(email.status, "pending");
    assert!(email.last_attempted_at.is_some());
}

#[tokio::test]
async fn test_mark_email_failed_max_attempts() {
    let db = setup_test_db().await;

    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "Subject",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    for _ in 0..6 {
        // General emails have 6 max attempts
        db.mark_email_failed(&id, "Failed").await.unwrap();
    }

    // Should not be in pending queue anymore (marked as failed)
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[tokio::test]
async fn test_get_pending_emails_excludes_failed() {
    let db = setup_test_db().await;

    let id1 = db
        .queue_email(
            "pending@example.com",
            "sender@example.com",
            "Pending",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    let id2 = db
        .queue_email(
            "failed@example.com",
            "sender@example.com",
            "Failed",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    for _ in 0..6 {
        // General emails have 6 max attempts
        db.mark_email_failed(&id2, "Error").await.unwrap();
    }

    let emails = db.get_pending_emails(10).await.unwrap();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].id, id1);
}

#[tokio::test]
async fn test_queue_email_safe_with_valid_address() {
    let db = setup_test_db().await;

    let result = db
        .queue_email_safe(
            Some("test@example.com"),
            "sender@example.com",
            "Test Subject",
            "Test Body",
            false,
            EmailType::General,
        )
        .await;

    assert!(result);

    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].to_addr, "test@example.com");
}

#[tokio::test]
async fn test_queue_email_safe_with_none_address() {
    let db = setup_test_db().await;

    let result = db
        .queue_email_safe(
            None,
            "sender@example.com",
            "Test Subject",
            "Test Body",
            false,
            EmailType::General,
        )
        .await;

    assert!(!result);

    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 0);
}

#[tokio::test]
async fn test_reset_email_for_retry_success() {
    let db = setup_test_db().await;

    // Create and fail an email
    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "Subject",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();

    for _ in 0..6 {
        db.mark_email_failed(&id, "Test error").await.unwrap();
    }

    // Verify it's failed
    let failed = db.get_failed_emails(10).await.unwrap();
    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].status, "failed");
    assert_eq!(failed[0].attempts, 6);

    // Reset it
    let result = db.reset_email_for_retry(&id).await.unwrap();
    assert!(result);

    // Verify it's back in pending queue with reset attempts
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, "pending");
    assert_eq!(pending[0].attempts, 0);
    assert!(pending[0].last_error.is_none());
}

#[tokio::test]
async fn test_reset_email_for_retry_not_found() {
    let db = setup_test_db().await;

    let nonexistent_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let result = db.reset_email_for_retry(&nonexistent_id).await.unwrap();
    assert!(!result);
}

#[tokio::test]
async fn test_retry_all_failed_emails_none() {
    let db = setup_test_db().await;

    let count = db.retry_all_failed_emails().await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_retry_all_failed_emails_multiple() {
    let db = setup_test_db().await;

    // Create and fail 3 emails
    for i in 0..3 {
        let id = db
            .queue_email(
                &format!("test{}@example.com", i),
                "sender@example.com",
                "Subject",
                "Body",
                false,
                EmailType::General,
            )
            .await
            .unwrap();

        for _ in 0..6 {
            db.mark_email_failed(&id, "Test error").await.unwrap();
        }
    }

    // Verify all are failed
    let failed = db.get_failed_emails(10).await.unwrap();
    assert_eq!(failed.len(), 3);

    // Reset all failed
    let count = db.retry_all_failed_emails().await.unwrap();
    assert_eq!(count, 3);

    // Verify all are back in pending
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 3);
    for email in &pending {
        assert_eq!(email.status, "pending");
        assert_eq!(email.attempts, 0);
        assert!(email.last_error.is_none());
    }

    // Verify failed queue is empty
    let failed = db.get_failed_emails(10).await.unwrap();
    assert_eq!(failed.len(), 0);
}

#[tokio::test]
async fn test_retry_all_failed_emails_excludes_pending_and_sent() {
    let db = setup_test_db().await;

    // Create one pending email
    db.queue_email(
        "pending@example.com",
        "sender@example.com",
        "Pending",
        "Body",
        false,
        EmailType::General,
    )
    .await
    .unwrap();

    // Create and send one email
    let sent_id = db
        .queue_email(
            "sent@example.com",
            "sender@example.com",
            "Sent",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();
    db.mark_email_sent(&sent_id).await.unwrap();

    // Create and fail one email
    let failed_id = db
        .queue_email(
            "failed@example.com",
            "sender@example.com",
            "Failed",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();
    for _ in 0..6 {
        db.mark_email_failed(&failed_id, "Error").await.unwrap();
    }

    // Retry all failed - should only affect the 1 failed email
    let count = db.retry_all_failed_emails().await.unwrap();
    assert_eq!(count, 1);

    // Verify pending queue now has 2 emails (original pending + reset failed)
    let pending = db.get_pending_emails(10).await.unwrap();
    assert_eq!(pending.len(), 2);
}

#[tokio::test]
async fn test_get_email_stats_empty() {
    let db = setup_test_db().await;

    let stats = db.get_email_stats().await.unwrap();
    assert_eq!(stats.pending, 0);
    assert_eq!(stats.sent, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(stats.total, 0);
}

#[tokio::test]
async fn test_get_email_stats_accuracy() {
    let db = setup_test_db().await;

    // Create 2 pending emails
    db.queue_email(
        "pending1@example.com",
        "sender@example.com",
        "Pending 1",
        "Body",
        false,
        EmailType::General,
    )
    .await
    .unwrap();

    db.queue_email(
        "pending2@example.com",
        "sender@example.com",
        "Pending 2",
        "Body",
        false,
        EmailType::General,
    )
    .await
    .unwrap();

    // Create and send 3 emails
    for i in 0..3 {
        let id = db
            .queue_email(
                &format!("sent{}@example.com", i),
                "sender@example.com",
                "Sent",
                "Body",
                false,
                EmailType::General,
            )
            .await
            .unwrap();
        db.mark_email_sent(&id).await.unwrap();
    }

    // Create and fail 1 email
    let failed_id = db
        .queue_email(
            "failed@example.com",
            "sender@example.com",
            "Failed",
            "Body",
            false,
            EmailType::General,
        )
        .await
        .unwrap();
    for _ in 0..6 {
        db.mark_email_failed(&failed_id, "Error").await.unwrap();
    }

    // Verify stats
    let stats = db.get_email_stats().await.unwrap();
    assert_eq!(stats.pending, 2);
    assert_eq!(stats.sent, 3);
    assert_eq!(stats.failed, 1);
    assert_eq!(stats.total, 6);
}
