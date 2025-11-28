use crate::database::test_helpers::setup_test_db;
use crate::database::email::EmailType;

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

    for _ in 0..6 {  // General emails have 6 max attempts
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

    for _ in 0..6 {  // General emails have 6 max attempts
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
