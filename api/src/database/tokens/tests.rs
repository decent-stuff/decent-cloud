use crate::database::test_helpers::setup_test_db;

#[tokio::test]
async fn test_get_account_approvals_empty() {
    let db = setup_test_db().await;
    let approvals = db.get_account_approvals("alice").await.unwrap();
    assert_eq!(approvals.len(), 0);
}

#[tokio::test]
async fn test_get_account_approvals() {
    let db = setup_test_db().await;

    sqlx::query!(
        "INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES ('alice', 'bob', 1000, NULL, 0)"
    )
    .execute(&db.pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES ('bob', 'alice', 500, NULL, 1000)"
    )
    .execute(&db.pool)
    .await
    .unwrap();

    let approvals = db.get_account_approvals("alice").await.unwrap();
    assert_eq!(approvals.len(), 2);
}
