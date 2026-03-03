use super::types::Database;
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Spending alert configuration for a user.
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct SpendingAlert {
    /// Monthly spending limit in USD.
    pub monthly_limit_usd: f64,
    /// Percentage of limit at which to send the first alert (1–100).
    pub alert_at_pct: i32,
    /// When the last notification was sent (Unix seconds), if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub last_notified_at: Option<i64>,
}

impl Database {
    /// Get spending alert config for a user (hex pubkey).
    pub async fn get_spending_alert(&self, pubkey_hex: &str) -> Result<Option<SpendingAlert>> {
        let row = sqlx::query!(
            r#"SELECT monthly_limit_usd,
                      alert_at_pct,
                      EXTRACT(EPOCH FROM last_notified_at)::BIGINT as "last_notified_at: i64"
               FROM spending_alerts WHERE pubkey = $1"#,
            pubkey_hex
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| SpendingAlert {
            monthly_limit_usd: r.monthly_limit_usd,
            alert_at_pct: r.alert_at_pct,
            last_notified_at: r.last_notified_at,
        }))
    }

    /// Insert or update spending alert config for a user.
    #[cfg(test)]
    pub async fn upsert_spending_alert(
        &self,
        pubkey_hex: &str,
        limit_usd: f64,
        alert_at_pct: i32,
    ) -> Result<SpendingAlert> {
        sqlx::query!(
            r#"INSERT INTO spending_alerts (pubkey, monthly_limit_usd, alert_at_pct)
               VALUES ($1, $2, $3)
               ON CONFLICT (pubkey) DO UPDATE
               SET monthly_limit_usd = EXCLUDED.monthly_limit_usd,
                   alert_at_pct = EXCLUDED.alert_at_pct,
                   updated_at = NOW()"#,
            pubkey_hex,
            limit_usd,
            alert_at_pct,
        )
        .execute(&self.pool)
        .await?;

        self.get_spending_alert(pubkey_hex)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Spending alert not found after upsert"))
    }

    /// Delete spending alert config for a user.
    #[cfg(test)]
    pub async fn delete_spending_alert(&self, pubkey_hex: &str) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM spending_alerts WHERE pubkey = $1", pubkey_hex)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0u64)
    }

    /// Sum of payment_amount_e9s for active/provisioning/provisioned contracts
    /// created by this requester in the current calendar month, converted to USD.
    ///
    /// `payment_amount_e9s` is in nanocents (1 USD = 1_000_000_000 e9s).
    pub async fn get_current_month_spending_usd(&self, requester_pubkey: &[u8]) -> Result<f64> {
        let sum: Option<i64> = sqlx::query_scalar!(
            r#"SELECT SUM(payment_amount_e9s)::BIGINT as "sum: i64"
               FROM contract_sign_requests
               WHERE requester_pubkey = $1
                 AND status IN ('active', 'provisioning', 'provisioned')
                 AND DATE_TRUNC('month', created_at) = DATE_TRUNC('month', NOW())"#,
            requester_pubkey,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(sum.unwrap_or(0) as f64 / 1_000_000_000.0)
    }

    /// Update the last_notified_at timestamp for a user's spending alert to now.
    pub async fn touch_spending_alert_notified_at(&self, pubkey_hex: &str) -> Result<()> {
        sqlx::query!(
            "UPDATE spending_alerts SET last_notified_at = NOW() WHERE pubkey = $1",
            pubkey_hex
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[test]
    fn test_spending_alert_struct_defaults() {
        let alert = SpendingAlert {
            monthly_limit_usd: 100.0,
            alert_at_pct: 80,
            last_notified_at: None,
        };
        assert_eq!(alert.monthly_limit_usd, 100.0);
        assert_eq!(alert.alert_at_pct, 80);
        assert!(alert.last_notified_at.is_none());
    }

    #[test]
    fn test_spending_alert_serialization() {
        let alert = SpendingAlert {
            monthly_limit_usd: 50.5,
            alert_at_pct: 75,
            last_notified_at: Some(1700000000),
        };
        let json = serde_json::to_value(&alert).unwrap();
        assert_eq!(json["monthlyLimitUsd"], 50.5);
        assert_eq!(json["alertAtPct"], 75);
        assert_eq!(json["lastNotifiedAt"], 1700000000i64);
    }

    #[test]
    fn test_spending_alert_serialization_no_notified_at() {
        let alert = SpendingAlert {
            monthly_limit_usd: 200.0,
            alert_at_pct: 90,
            last_notified_at: None,
        };
        let json = serde_json::to_value(&alert).unwrap();
        assert!(json.get("lastNotifiedAt").is_none());
    }

    #[tokio::test]
    async fn test_upsert_and_get_spending_alert() {
        let db = setup_test_db().await;
        let pubkey_hex = "a".repeat(64);

        let alert = db
            .upsert_spending_alert(&pubkey_hex, 150.0, 80)
            .await
            .unwrap();

        assert!((alert.monthly_limit_usd - 150.0).abs() < 0.01);
        assert_eq!(alert.alert_at_pct, 80);
        assert!(alert.last_notified_at.is_none());

        let fetched = db.get_spending_alert(&pubkey_hex).await.unwrap().unwrap();
        assert!((fetched.monthly_limit_usd - 150.0).abs() < 0.01);
        assert_eq!(fetched.alert_at_pct, 80);
    }

    #[tokio::test]
    async fn test_get_spending_alert_nonexistent() {
        let db = setup_test_db().await;
        let result = db.get_spending_alert(&"b".repeat(64)).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_upsert_spending_alert_updates_existing() {
        let db = setup_test_db().await;
        let pubkey_hex = "c".repeat(64);

        db.upsert_spending_alert(&pubkey_hex, 100.0, 70)
            .await
            .unwrap();
        let updated = db
            .upsert_spending_alert(&pubkey_hex, 200.0, 90)
            .await
            .unwrap();

        assert!((updated.monthly_limit_usd - 200.0).abs() < 0.01);
        assert_eq!(updated.alert_at_pct, 90);
    }

    #[tokio::test]
    async fn test_delete_spending_alert() {
        let db = setup_test_db().await;
        let pubkey_hex = "d".repeat(64);

        db.upsert_spending_alert(&pubkey_hex, 100.0, 80)
            .await
            .unwrap();
        let deleted = db.delete_spending_alert(&pubkey_hex).await.unwrap();
        assert!(deleted);

        let fetched = db.get_spending_alert(&pubkey_hex).await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_delete_spending_alert_nonexistent() {
        let db = setup_test_db().await;
        let deleted = db.delete_spending_alert(&"e".repeat(64)).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_get_current_month_spending_usd_empty() {
        let db = setup_test_db().await;
        let pubkey = vec![0x42u8; 32];
        let spending = db.get_current_month_spending_usd(&pubkey).await.unwrap();
        assert_eq!(spending, 0.0);
    }
}
