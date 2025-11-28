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
        )
        .await
        .unwrap();

    assert_eq!(id.len(), 16);

    let email = db.get_email_by_id(&id).await.unwrap().unwrap();
    assert_eq!(email.to_addr, "Test User <test@example.com>");
    assert_eq!(email.subject, "Test Subject");
    assert_eq!(email.status, "pending");
    assert_eq!(email.attempts, 0);
    assert!(email.last_attempted_at.is_none());
}

#[tokio::test]
async fn test_queue_html_email() {
    let db = setup_test_db().await;

    let id = db
        .queue_email(
            "test@example.com",
            "sender@example.com",
            "HTML Test",
            "<h1>HTML Body</h1>",
            true,
        )
        .await
        .unwrap();

    let email = db.get_email_by_id(&id).await.unwrap().unwrap();
    assert_eq!(email.is_html, 1);
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
    )
    .await
    .unwrap();

    db.queue_email(
        "test2@example.com",
        "sender@example.com",
        "Subject 2",
        "Body 2",
        false,
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
        )
        .await
        .unwrap();

    db.mark_email_sent(&id).await.unwrap();

    let email = db.get_email_by_id(&id).await.unwrap().unwrap();
    assert_eq!(email.status, "sent");
    assert!(email.sent_at.is_some());
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
        )
        .await
        .unwrap();

    db.mark_email_failed(&id, "Connection timeout")
        .await
        .unwrap();

    let email = db.get_email_by_id(&id).await.unwrap().unwrap();
    assert_eq!(email.attempts, 1);
    assert_eq!(email.last_error.unwrap(), "Connection timeout");
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
        )
        .await
        .unwrap();

    for _ in 0..3 {
        db.mark_email_failed(&id, "Failed").await.unwrap();
    }

    let email = db.get_email_by_id(&id).await.unwrap().unwrap();
    assert_eq!(email.attempts, 3);
    assert_eq!(email.status, "failed");
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
        )
        .await
        .unwrap();

    for _ in 0..3 {
        db.mark_email_failed(&id2, "Error").await.unwrap();
    }

    let emails = db.get_pending_emails(10).await.unwrap();
    assert_eq!(emails.len(), 1);
    assert_eq!(emails[0].id, id1);
}

#[tokio::test]
async fn test_get_email_by_id_not_found() {
    let db = setup_test_db().await;
    let id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let email = db.get_email_by_id(&id).await.unwrap();
    assert!(email.is_none());
}
