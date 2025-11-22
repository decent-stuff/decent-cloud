use super::*;
use crate::database::test_helpers::setup_test_db;

async fn insert_transfer(
    db: &Database,
    from: &str,
    to: &str,
    amount: i64,
    fee: i64,
    timestamp: i64,
) {
    sqlx::query!(
        "INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns) VALUES (?, ?, ?, ?, '', ?)",
        from,
        to,
        amount,
        fee,
        timestamp
    )
    .execute(&db.pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn test_get_account_transfers_empty() {
    let db = setup_test_db().await;
    let transfers = db.get_account_transfers("alice", 10).await.unwrap();
    assert_eq!(transfers.len(), 0);
}

#[tokio::test]
async fn test_get_account_transfers() {
    let db = setup_test_db().await;

    insert_transfer(&db, "alice", "bob", 100, 1, 1000).await;
    insert_transfer(&db, "bob", "alice", 50, 1, 2000).await;
    insert_transfer(&db, "charlie", "dave", 200, 1, 3000).await;

    let transfers = db.get_account_transfers("alice", 10).await.unwrap();
    assert_eq!(transfers.len(), 2);
}

#[tokio::test]
async fn test_get_account_transfers_limit() {
    let db = setup_test_db().await;

    for i in 0..5 {
        insert_transfer(&db, "alice", "bob", 100, 1, i * 1000).await;
    }

    let transfers = db.get_account_transfers("alice", 3).await.unwrap();
    assert_eq!(transfers.len(), 3);
}

#[tokio::test]
async fn test_get_recent_transfers() {
    let db = setup_test_db().await;

    insert_transfer(&db, "alice", "bob", 100, 1, 1000).await;
    insert_transfer(&db, "bob", "charlie", 50, 1, 2000).await;

    let transfers = db.get_recent_transfers(10).await.unwrap();
    assert_eq!(transfers.len(), 2);
    assert_eq!(transfers[0].created_at_ns, 2000);
}

#[tokio::test]
async fn test_get_account_balance_zero() {
    let db = setup_test_db().await;
    let balance = db.get_account_balance("alice").await.unwrap();
    assert_eq!(balance, 0);
}

#[tokio::test]
async fn test_get_account_balance() {
    let db = setup_test_db().await;

    insert_transfer(&db, "alice", "bob", 100, 10, 1000).await;
    insert_transfer(&db, "charlie", "alice", 200, 5, 2000).await;
    insert_transfer(&db, "alice", "dave", 50, 5, 3000).await;

    let balance = db.get_account_balance("alice").await.unwrap();
    assert_eq!(balance, 200 - 100 - 10 - 50 - 5);
}

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
