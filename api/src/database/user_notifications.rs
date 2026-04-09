use super::types::Database;
use anyhow::Result;

/// A user notification stored in the database.
#[derive(Debug, Clone)]
pub struct UserNotification {
    pub id: i64,
    pub notification_type: String,
    pub title: String,
    pub body: String,
    pub contract_id: Option<String>,
    pub offering_id: Option<i64>,
    pub price_direction: Option<String>,
    pub read_at: Option<i64>,
    pub created_at: i64,
}

impl Database {
    /// Insert a new notification for the given user. Returns the new notification ID.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_user_notification(
        &self,
        user_pubkey: &[u8],
        notification_type: &str,
        title: &str,
        body: &str,
        contract_id: Option<&str>,
        offering_id: Option<i64>,
        price_direction: Option<&str>,
    ) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let id = sqlx::query_scalar!(
            r#"INSERT INTO user_notifications (user_pubkey, type, title, body, contract_id, offering_id, price_direction, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id"#,
            user_pubkey,
            notification_type,
            title,
            body,
            contract_id,
            offering_id,
            price_direction,
            now,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Return the last `limit` notifications for a user, newest first.
    pub async fn get_user_notifications(
        &self,
        user_pubkey: &[u8],
        limit: i64,
    ) -> Result<Vec<UserNotification>> {
        let rows = sqlx::query!(
            r#"SELECT id, type, title, body, contract_id, offering_id, price_direction, read_at, created_at
               FROM user_notifications
               WHERE user_pubkey = $1
               ORDER BY created_at DESC, id DESC
               LIMIT $2"#,
            user_pubkey,
            limit,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| UserNotification {
                id: r.id,
                notification_type: r.r#type,
                title: r.title,
                body: r.body,
                contract_id: r.contract_id,
                offering_id: r.offering_id,
                price_direction: r.price_direction,
                read_at: r.read_at,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Count unread notifications for a user.
    pub async fn get_unread_count(&self, user_pubkey: &[u8]) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!: i64"
               FROM user_notifications
               WHERE user_pubkey = $1 AND read_at IS NULL"#,
            user_pubkey,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count)
    }

    /// Mark specific notification IDs as read (only if they belong to user_pubkey).
    pub async fn mark_notifications_read(&self, ids: &[i64], user_pubkey: &[u8]) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query!(
            r#"UPDATE user_notifications
               SET read_at = $1
               WHERE id = ANY($2) AND user_pubkey = $3 AND read_at IS NULL"#,
            now,
            ids,
            user_pubkey,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Mark all notifications for a user as read.
    pub async fn mark_all_notifications_read(&self, user_pubkey: &[u8]) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query!(
            r#"UPDATE user_notifications
               SET read_at = $1
               WHERE user_pubkey = $2 AND read_at IS NULL"#,
            now,
            user_pubkey,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_insert_and_get_notifications() {
        let db = setup_test_db().await;
        let pubkey = vec![0x01u8; 32];

        let id = db
            .insert_user_notification(
                &pubkey,
                "contract_status",
                "Contract Accepted",
                "Your rental request was accepted.",
                Some("abc123"),
                None,
                None,
            )
            .await
            .unwrap();

        assert!(id > 0);

        let notifications = db.get_user_notifications(&pubkey, 50).await.unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].id, id);
        assert_eq!(notifications[0].notification_type, "contract_status");
        assert_eq!(notifications[0].title, "Contract Accepted");
        assert_eq!(notifications[0].contract_id.as_deref(), Some("abc123"));
        assert!(notifications[0].price_direction.is_none());
        assert!(notifications[0].read_at.is_none());
    }

    #[tokio::test]
    async fn test_get_notifications_empty_for_unknown_pubkey() {
        let db = setup_test_db().await;
        let pubkey = vec![0x99u8; 32];

        let notifications = db.get_user_notifications(&pubkey, 50).await.unwrap();
        assert!(notifications.is_empty());
    }

    #[tokio::test]
    async fn test_unread_count() {
        let db = setup_test_db().await;
        let pubkey = vec![0x02u8; 32];

        // No notifications yet
        assert_eq!(db.get_unread_count(&pubkey).await.unwrap(), 0);

        db.insert_user_notification(&pubkey, "contract_provisioned", "VM Ready", "Your VM is provisioned.", None, None, None)
        .await
        .unwrap();
        db.insert_user_notification(&pubkey, "auto_renewed", "Auto-renewed", "Contract was renewed.", None, None, None)
        .await
        .unwrap();

        assert_eq!(db.get_unread_count(&pubkey).await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_mark_specific_notifications_read() {
        let db = setup_test_db().await;
        let pubkey = vec![0x03u8; 32];

        let id1 = db
            .insert_user_notification(
                &pubkey,
                "contract_status",
                "Cancelled",
                "Contract was cancelled.",
                None,
                None,
                None,
            )
            .await
            .unwrap();
        let id2 = db
            .insert_user_notification(
                &pubkey,
                "contract_status",
                "Rejected",
                "Contract was rejected.",
                None,
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(db.get_unread_count(&pubkey).await.unwrap(), 2);

        db.mark_notifications_read(&[id1], &pubkey).await.unwrap();

        assert_eq!(db.get_unread_count(&pubkey).await.unwrap(), 1);

        let notifications = db.get_user_notifications(&pubkey, 50).await.unwrap();
        let n1 = notifications.iter().find(|n| n.id == id1).unwrap();
        let n2 = notifications.iter().find(|n| n.id == id2).unwrap();
        assert!(n1.read_at.is_some());
        assert!(n2.read_at.is_none());
    }

    #[tokio::test]
    async fn test_mark_all_notifications_read() {
        let db = setup_test_db().await;
        let pubkey = vec![0x04u8; 32];

        db.insert_user_notification(
            &pubkey,
            "rental_request",
            "New Request",
            "A tenant rented your VM.",
            None,
            None,
            None,
        )
        .await
        .unwrap();
        db.insert_user_notification(
            &pubkey,
            "password_reset_complete",
            "Password Reset",
            "Password was reset.",
            None,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(db.get_unread_count(&pubkey).await.unwrap(), 2);

        db.mark_all_notifications_read(&pubkey).await.unwrap();

        assert_eq!(db.get_unread_count(&pubkey).await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_mark_read_does_not_affect_other_users() {
        let db = setup_test_db().await;
        let user_a = vec![0x05u8; 32];
        let user_b = vec![0x06u8; 32];

        let id = db
            .insert_user_notification(&user_a, "contract_status", "Title", "Body", None, None, None)
            .await
            .unwrap();

        // Try to mark user_a's notification as read with user_b's pubkey - should do nothing
        db.mark_notifications_read(&[id], &user_b).await.unwrap();

        // user_a still has 1 unread
        assert_eq!(db.get_unread_count(&user_a).await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_get_notifications_limit() {
        let db = setup_test_db().await;
        let pubkey = vec![0x07u8; 32];

        for i in 0..5 {
            db.insert_user_notification(
                &pubkey,
                "contract_status",
                &format!("N{}", i),
                "body",
                None,
                None,
                None,
            )
            .await
            .unwrap();
        }

        let limited = db.get_user_notifications(&pubkey, 3).await.unwrap();
        assert_eq!(limited.len(), 3);
    }

    #[tokio::test]
    async fn test_notifications_ordered_newest_first() {
        let db = setup_test_db().await;
        let pubkey = vec![0x08u8; 32];

        let id1 = db
            .insert_user_notification(&pubkey, "contract_status", "First", "body", None, None, None)
            .await
            .unwrap();
        let id2 = db
            .insert_user_notification(&pubkey, "contract_status", "Second", "body", None, None, None)
            .await
            .unwrap();

        let notifications = db.get_user_notifications(&pubkey, 50).await.unwrap();
        // Newest first: id2 should appear before id1
        assert_eq!(notifications[0].id, id2);
        assert_eq!(notifications[1].id, id1);
    }

    #[tokio::test]
    async fn test_notification_with_offering_id() {
        let db = setup_test_db().await;
        let pubkey = vec![0x09u8; 32];

        let id = db
            .insert_user_notification(
                &pubkey,
                "saved_offering_price_change",
                "Saved offering price changed",
                "Test Offer: monthly_price from USD 10.00 to USD 12.50.",
                None,
                Some(42),
                Some("up"),
            )
            .await
            .unwrap();

        let notifications = db.get_user_notifications(&pubkey, 50).await.unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].offering_id, Some(42));
        assert_eq!(notifications[0].price_direction.as_deref(), Some("up"));

        let id2 = db
            .insert_user_notification(
                &pubkey,
                "contract_status",
                "Accepted",
                "Contract accepted.",
                Some("c1"),
                None,
                None,
            )
            .await
            .unwrap();

        let notifications = db.get_user_notifications(&pubkey, 50).await.unwrap();
        assert_eq!(notifications.len(), 2);
        let with_offering = notifications.iter().find(|n| n.id == id).unwrap();
        assert_eq!(with_offering.offering_id, Some(42));
        let without_offering = notifications.iter().find(|n| n.id == id2).unwrap();
        assert_eq!(without_offering.offering_id, None);
    }

    #[tokio::test]
    async fn test_notification_rejects_invalid_price_direction() {
        let db = setup_test_db().await;
        let pubkey = vec![0x0Au8; 32];

        let err = db
            .insert_user_notification(
                &pubkey,
                "saved_offering_price_change",
                "Saved offering price changed",
                "Test Offer: monthly_price from USD 10.00 to USD 12.50.",
                None,
                Some(42),
                Some("sideways"),
            )
            .await
            .expect_err("invalid price direction should fail");

        assert!(err.to_string().contains("user_notifications_price_direction_check"));
    }
}
