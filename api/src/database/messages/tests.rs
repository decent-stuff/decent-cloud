use crate::database::test_helpers::setup_test_db;

async fn insert_test_contract(
    db: &crate::database::Database,
    contract_id: &[u8],
    requester_pubkey: &[u8],
    provider_pubkey: &[u8],
) {
    let payment_method = "dct";
    let payment_status = "succeeded";
    let stripe_payment_intent_id: Option<&str> = None;
    let stripe_customer_id: Option<&str> = None;
    sqlx::query!(
        "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency) VALUES (?, ?, 'ssh-key', 'contact', ?, 'test-offering', 1000, 'test', 0, 'active', ?, ?, ?, ?, 'usd')",
        contract_id,
        requester_pubkey,
        provider_pubkey,
        payment_method,
        stripe_payment_intent_id,
        stripe_customer_id,
        payment_status
    )
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_create_thread() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Subject", &requester, &provider)
        .await
        .unwrap();

    assert_eq!(thread_id.len(), 16);

    let thread = db.get_thread_by_contract(&contract_id).await.unwrap();
    assert!(thread.is_some());
    let thread = thread.unwrap();
    assert_eq!(thread.subject, "Test Subject");
    assert_eq!(thread.status, "open");
}

#[tokio::test]
async fn test_create_thread_duplicate_fails() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    db.create_thread(&contract_id, "First Thread", &requester, &provider)
        .await
        .unwrap();

    let result = db
        .create_thread(&contract_id, "Second Thread", &requester, &provider)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_thread_by_contract_not_found() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();

    let thread = db.get_thread_by_contract(&contract_id).await.unwrap();
    assert!(thread.is_none());
}

#[tokio::test]
async fn test_create_message() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let message_id = db
        .create_message(&thread_id, &requester, "user", "Hello, Provider!")
        .await
        .unwrap();

    assert_eq!(message_id.len(), 16);

    let messages = db
        .get_messages_for_thread(&thread_id, &requester)
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].body, "Hello, Provider!");
    assert_eq!(messages[0].sender_role, "user");
}

#[tokio::test]
async fn test_create_message_updates_thread_timestamp() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let thread_before = db
        .get_thread_by_contract(&contract_id)
        .await
        .unwrap()
        .unwrap();
    let initial_timestamp = thread_before.last_message_at_ns;

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    db.create_message(&thread_id, &requester, "user", "Message")
        .await
        .unwrap();

    let thread_after = db
        .get_thread_by_contract(&contract_id)
        .await
        .unwrap()
        .unwrap();
    assert!(thread_after.last_message_at_ns > initial_timestamp);
}

#[tokio::test]
async fn test_get_messages_for_thread_empty() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let messages = db
        .get_messages_for_thread(&thread_id, &requester)
        .await
        .unwrap();
    assert_eq!(messages.len(), 0);
}

#[tokio::test]
async fn test_get_messages_for_thread_multiple() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    db.create_message(&thread_id, &requester, "user", "First message")
        .await
        .unwrap();
    db.create_message(&thread_id, &provider, "user", "Second message")
        .await
        .unwrap();
    db.create_message(&thread_id, &requester, "user", "Third message")
        .await
        .unwrap();

    let messages = db
        .get_messages_for_thread(&thread_id, &requester)
        .await
        .unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].body, "First message");
    assert_eq!(messages[1].body, "Second message");
    assert_eq!(messages[2].body, "Third message");
}

#[tokio::test]
async fn test_mark_message_read() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let message_id = db
        .create_message(&thread_id, &requester, "user", "Test message")
        .await
        .unwrap();

    let messages_before = db
        .get_messages_for_thread(&thread_id, &provider)
        .await
        .unwrap();
    assert!(!messages_before[0].is_read);

    db.mark_message_read(&message_id, &provider).await.unwrap();

    let messages_after = db
        .get_messages_for_thread(&thread_id, &provider)
        .await
        .unwrap();
    assert!(messages_after[0].is_read);
}

#[tokio::test]
async fn test_mark_message_read_idempotent() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let message_id = db
        .create_message(&thread_id, &requester, "user", "Test message")
        .await
        .unwrap();

    db.mark_message_read(&message_id, &provider).await.unwrap();
    let result = db.mark_message_read(&message_id, &provider).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_unread_count_zero() {
    let db = setup_test_db().await;
    let user = hex::encode(uuid::Uuid::new_v4().as_bytes());

    let count = db.get_unread_count(&user).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_get_unread_count_excludes_own_messages() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    db.create_message(&thread_id, &requester, "user", "My message")
        .await
        .unwrap();

    let count = db.get_unread_count(&requester).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn test_get_unread_count_with_unread_messages() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    db.create_message(&thread_id, &provider, "user", "Message 1")
        .await
        .unwrap();
    db.create_message(&thread_id, &provider, "user", "Message 2")
        .await
        .unwrap();

    let count = db.get_unread_count(&requester).await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_get_unread_count_after_marking_read() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let msg1 = db
        .create_message(&thread_id, &provider, "user", "Message 1")
        .await
        .unwrap();
    let msg2 = db
        .create_message(&thread_id, &provider, "user", "Message 2")
        .await
        .unwrap();

    let count_before = db.get_unread_count(&requester).await.unwrap();
    assert_eq!(count_before, 2);

    db.mark_message_read(&msg1, &requester).await.unwrap();

    let count_after = db.get_unread_count(&requester).await.unwrap();
    assert_eq!(count_after, 1);

    db.mark_message_read(&msg2, &requester).await.unwrap();

    let count_final = db.get_unread_count(&requester).await.unwrap();
    assert_eq!(count_final, 0);
}

#[tokio::test]
async fn test_get_threads_for_user_empty() {
    let db = setup_test_db().await;
    let user = hex::encode(uuid::Uuid::new_v4().as_bytes());

    let threads = db.get_threads_for_user(&user).await.unwrap();
    assert_eq!(threads.len(), 0);
}

#[tokio::test]
async fn test_get_threads_for_user_as_requester() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    db.create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let threads = db.get_threads_for_user(&requester).await.unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].subject, "Test Thread");
}

#[tokio::test]
async fn test_get_threads_for_user_as_provider() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    db.create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let threads = db.get_threads_for_user(&provider).await.unwrap();
    assert_eq!(threads.len(), 1);
    assert_eq!(threads[0].subject, "Test Thread");
}

#[tokio::test]
async fn test_get_threads_for_user_sorted_by_last_message() {
    let db = setup_test_db().await;
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    let contract1_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let contract2_id = uuid::Uuid::new_v4().as_bytes().to_vec();

    insert_test_contract(
        &db,
        &contract1_id,
        requester.as_bytes(),
        provider.as_bytes(),
    )
    .await;
    insert_test_contract(
        &db,
        &contract2_id,
        requester.as_bytes(),
        provider.as_bytes(),
    )
    .await;

    let thread1_id = db
        .create_thread(&contract1_id, "Thread 1", &requester, &provider)
        .await
        .unwrap();
    let _thread2_id = db
        .create_thread(&contract2_id, "Thread 2", &requester, &provider)
        .await
        .unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    db.create_message(&thread1_id, &requester, "user", "Latest message")
        .await
        .unwrap();

    let threads = db.get_threads_for_user(&requester).await.unwrap();
    assert_eq!(threads.len(), 2);
    assert_eq!(threads[0].subject, "Thread 1");
    assert_eq!(threads[1].subject, "Thread 2");
}

#[tokio::test]
async fn test_queue_message_notification() {
    let db = setup_test_db().await;
    let contract_id = uuid::Uuid::new_v4().as_bytes().to_vec();
    let requester = hex::encode(uuid::Uuid::new_v4().as_bytes());
    let provider = hex::encode(uuid::Uuid::new_v4().as_bytes());

    insert_test_contract(&db, &contract_id, requester.as_bytes(), provider.as_bytes()).await;

    let thread_id = db
        .create_thread(&contract_id, "Test Thread", &requester, &provider)
        .await
        .unwrap();

    let message_id = db
        .create_message(&thread_id, &requester, "user", "Test message")
        .await
        .unwrap();

    let notification_id = db
        .queue_message_notification(&message_id, &provider)
        .await
        .unwrap();

    assert_eq!(notification_id.len(), 16);
}
