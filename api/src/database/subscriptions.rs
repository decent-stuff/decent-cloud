use super::types::Database;
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Subscription plan definition
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, Object)]
pub struct SubscriptionPlan {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub stripe_price_id: Option<String>,
    #[oai(rename = "monthlyPriceCents")]
    pub monthly_price_cents: i64,
    #[oai(rename = "trialDays")]
    pub trial_days: i64,
    pub features: Option<String>, // JSON array
}

impl SubscriptionPlan {
    /// Parse features JSON into a Vec<String>
    pub fn features_list(&self) -> Vec<String> {
        self.features
            .as_ref()
            .and_then(|f| serde_json::from_str(f).ok())
            .unwrap_or_default()
    }
}

/// Account subscription details
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct AccountSubscription {
    pub plan_id: String,
    pub plan_name: String,
    pub status: String,
    pub stripe_subscription_id: Option<String>,
    pub current_period_end: Option<i64>,
    pub cancel_at_period_end: bool,
    pub features: Vec<String>,
}

/// Subscription event for audit trail
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct SubscriptionEvent {
    pub id: i64,
    pub account_id: Vec<u8>,
    pub event_type: String,
    pub stripe_event_id: Option<String>,
    pub old_plan_id: Option<String>,
    pub new_plan_id: Option<String>,
    pub stripe_subscription_id: Option<String>,
    pub stripe_invoice_id: Option<String>,
    pub amount_cents: Option<i64>,
    pub metadata: Option<String>,
    pub created_at: i64,
}

/// Input for creating a subscription event
#[derive(Debug, Default)]
pub struct SubscriptionEventInput<'a> {
    pub event_type: &'a str,
    pub stripe_event_id: Option<&'a str>,
    pub old_plan_id: Option<&'a str>,
    pub new_plan_id: Option<&'a str>,
    pub stripe_subscription_id: Option<&'a str>,
    pub stripe_invoice_id: Option<&'a str>,
    pub amount_cents: Option<i64>,
    pub metadata: Option<&'a str>,
}

impl Database {
    /// List all subscription plans
    pub async fn list_subscription_plans(&self) -> Result<Vec<SubscriptionPlan>> {
        let plans = sqlx::query_as!(
            SubscriptionPlan,
            r#"SELECT id as "id!", name as "name!", description, stripe_price_id,
                      monthly_price_cents as "monthly_price_cents!",
                      trial_days as "trial_days!", features
               FROM subscription_plans
               ORDER BY monthly_price_cents ASC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(plans)
    }

    /// Get subscription plan by ID
    pub async fn get_subscription_plan(&self, plan_id: &str) -> Result<Option<SubscriptionPlan>> {
        let plan = sqlx::query_as!(
            SubscriptionPlan,
            r#"SELECT id as "id!", name as "name!", description, stripe_price_id,
                      monthly_price_cents as "monthly_price_cents!",
                      trial_days as "trial_days!", features
               FROM subscription_plans
               WHERE id = $1"#,
            plan_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(plan)
    }

    /// Get subscription plan by Stripe price ID
    pub async fn get_subscription_plan_by_stripe_price(
        &self,
        stripe_price_id: &str,
    ) -> Result<Option<SubscriptionPlan>> {
        let plan = sqlx::query_as!(
            SubscriptionPlan,
            r#"SELECT id as "id!", name as "name!", description, stripe_price_id,
                      monthly_price_cents as "monthly_price_cents!",
                      trial_days as "trial_days!", features
               FROM subscription_plans
               WHERE stripe_price_id = $1"#,
            stripe_price_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(plan)
    }

    /// Get account subscription details
    pub async fn get_account_subscription(&self, account_id: &[u8]) -> Result<AccountSubscription> {
        // First get account subscription fields
        let account_row = sqlx::query!(
            r#"SELECT subscription_plan_id, subscription_status,
                      subscription_stripe_id, subscription_current_period_end,
                      subscription_cancel_at_period_end
               FROM accounts WHERE id = $1"#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Account not found"))?;

        let plan_id = account_row
            .subscription_plan_id
            .unwrap_or_else(|| "free".to_string());

        // Get plan details
        let plan = self.get_subscription_plan(&plan_id).await?;
        let (plan_name, features) = match plan {
            Some(p) => {
                let features = p.features_list();
                (p.name, features)
            }
            None => ("Free".to_string(), vec!["marketplace_browse".to_string()]),
        };

        Ok(AccountSubscription {
            plan_id,
            plan_name,
            status: account_row
                .subscription_status
                .unwrap_or_else(|| "active".to_string()),
            stripe_subscription_id: account_row.subscription_stripe_id,
            current_period_end: account_row.subscription_current_period_end,
            cancel_at_period_end: account_row.subscription_cancel_at_period_end.unwrap_or(false),
            features,
        })
    }

    /// Update account subscription from Stripe webhook
    pub async fn update_account_subscription(
        &self,
        account_id: &[u8],
        plan_id: &str,
        status: &str,
        stripe_subscription_id: Option<&str>,
        current_period_end: Option<i64>,
        cancel_at_period_end: bool,
    ) -> Result<()> {
        sqlx::query!(
            r#"UPDATE accounts SET
               subscription_plan_id = $1,
               subscription_status = $2,
               subscription_stripe_id = $3,
               subscription_current_period_end = $4,
               subscription_cancel_at_period_end = $5,
               updated_at = (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
               WHERE id = $6"#,
            plan_id,
            status,
            stripe_subscription_id,
            current_period_end,
            cancel_at_period_end,
            account_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Link Stripe customer ID to account
    pub async fn set_stripe_customer_id(&self, account_id: &[u8], customer_id: &str) -> Result<()> {
        sqlx::query!(
            r#"UPDATE accounts SET stripe_customer_id = $1,
               updated_at = (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
               WHERE id = $2"#,
            customer_id,
            account_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get Stripe customer ID for account
    pub async fn get_stripe_customer_id(&self, account_id: &[u8]) -> Result<Option<String>> {
        let row = sqlx::query!(
            r#"SELECT stripe_customer_id FROM accounts WHERE id = $1"#,
            account_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| r.stripe_customer_id))
    }

    /// Get account by Stripe customer ID (for webhooks)
    pub async fn get_account_id_by_stripe_customer(
        &self,
        customer_id: &str,
    ) -> Result<Option<Vec<u8>>> {
        let row = sqlx::query!(
            r#"SELECT id as "id!" FROM accounts WHERE stripe_customer_id = $1"#,
            customer_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.id))
    }

    /// Record subscription event (audit trail)
    pub async fn insert_subscription_event(
        &self,
        account_id: &[u8],
        input: SubscriptionEventInput<'_>,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO subscription_events
               (account_id, event_type, stripe_event_id, old_plan_id, new_plan_id,
                stripe_subscription_id, stripe_invoice_id, amount_cents, metadata)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
            account_id,
            input.event_type,
            input.stripe_event_id,
            input.old_plan_id,
            input.new_plan_id,
            input.stripe_subscription_id,
            input.stripe_invoice_id,
            input.amount_cents,
            input.metadata
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Count active contracts (status = 'active' or 'provisioned') for an account
    /// This counts contracts where the requester is any pubkey associated with the account
    pub async fn count_active_contracts_for_account(&self, account_id: &[u8]) -> Result<i64> {
        let row = sqlx::query!(
            r#"SELECT COUNT(*) as "count!: i64" FROM contract_sign_requests
               WHERE requester_pubkey IN (
                   SELECT public_key FROM account_public_keys WHERE account_id = $1 AND is_active = TRUE
               )
               AND status IN ('active', 'provisioned', 'provisioning', 'accepted', 'pending')"#,
            account_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(row.count)
    }

    /// Check if account has a specific feature (based on subscription)
    pub async fn account_has_feature(&self, account_id: &[u8], feature: &str) -> Result<bool> {
        let subscription = self.get_account_subscription(account_id).await?;

        // Only active/trialing subscriptions get features
        if subscription.status != "active" && subscription.status != "trialing" {
            return Ok(false);
        }

        Ok(subscription.features.contains(&feature.to_string()))
    }

    /// Update Stripe price ID for a plan (admin operation)
    #[allow(dead_code)]
    pub async fn update_plan_stripe_price_id(
        &self,
        plan_id: &str,
        stripe_price_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            r#"UPDATE subscription_plans SET stripe_price_id = $1,
               updated_at = (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
               WHERE id = $2"#,
            stripe_price_id,
            plan_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    async fn insert_contract_for_account(
        db: &crate::database::Database,
        contract_id: &[u8],
        requester_pubkey: &[u8],
        provider_pubkey: &[u8],
        status: &str,
    ) {
        sqlx::query!(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, request_memo, created_at_ns, status, payment_method, payment_status, currency) VALUES ($1, $2, 'ssh-key', 'contact', $3, 'off-1', 1000, 'memo', 0, $4, 'icpay', 'succeeded', 'usd')",
            contract_id,
            requester_pubkey,
            provider_pubkey,
            status,
        )
        .execute(&db.pool)
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_list_subscription_plans() {
        let db = setup_test_db().await;

        let plans = db.list_subscription_plans().await.unwrap();

        // Should have the 3 default plans from migration
        assert_eq!(plans.len(), 3);
        assert_eq!(plans[0].id, "free");
        assert_eq!(plans[1].id, "pro");
        assert_eq!(plans[2].id, "enterprise");
    }

    #[tokio::test]
    async fn test_get_subscription_plan() {
        let db = setup_test_db().await;

        let plan = db.get_subscription_plan("pro").await.unwrap();
        assert!(plan.is_some());

        let plan = plan.unwrap();
        assert_eq!(plan.id, "pro");
        assert_eq!(plan.name, "Pro");
        assert_eq!(plan.monthly_price_cents, 2900);
        assert_eq!(plan.trial_days, 14);
    }

    #[tokio::test]
    async fn test_get_subscription_plan_not_found() {
        let db = setup_test_db().await;

        let plan = db.get_subscription_plan("nonexistent").await.unwrap();
        assert!(plan.is_none());
    }

    #[tokio::test]
    async fn test_plan_features_list() {
        let db = setup_test_db().await;

        let plan = db.get_subscription_plan("pro").await.unwrap().unwrap();
        let features = plan.features_list();

        assert!(features.contains(&"marketplace_browse".to_string()));
        assert!(features.contains(&"unlimited_rentals".to_string()));
        assert!(features.contains(&"priority_support".to_string()));
        assert!(features.contains(&"api_access".to_string()));
    }

    #[tokio::test]
    async fn test_free_plan_features() {
        let db = setup_test_db().await;

        let plan = db.get_subscription_plan("free").await.unwrap().unwrap();
        let features = plan.features_list();

        assert!(features.contains(&"marketplace_browse".to_string()));
        assert!(features.contains(&"one_rental".to_string()));
        assert!(!features.contains(&"unlimited_rentals".to_string()));
    }

    #[tokio::test]
    async fn test_count_active_contracts_no_contracts() {
        let db = setup_test_db().await;
        let pubkey = [1u8; 32];

        // Create account
        let account = db
            .create_account("testuser", &pubkey, "test@example.com")
            .await
            .unwrap();

        // No contracts - should return 0
        let count = db
            .count_active_contracts_for_account(&account.id)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_count_active_contracts_with_active_contracts() {
        let db = setup_test_db().await;
        let pubkey = [1u8; 32];
        let provider_pubkey = [2u8; 32];

        // Create account
        let account = db
            .create_account("testuser", &pubkey, "test@example.com")
            .await
            .unwrap();

        // Insert active contract
        insert_contract_for_account(&db, &[3u8; 32], &pubkey, &provider_pubkey, "active").await;

        // Insert provisioned contract
        insert_contract_for_account(&db, &[4u8; 32], &pubkey, &provider_pubkey, "provisioned")
            .await;

        // Insert cancelled contract (should not be counted)
        insert_contract_for_account(&db, &[5u8; 32], &pubkey, &provider_pubkey, "cancelled").await;

        let count = db
            .count_active_contracts_for_account(&account.id)
            .await
            .unwrap();
        assert_eq!(count, 2); // Only active and provisioned
    }

    #[tokio::test]
    async fn test_account_has_feature_free_tier() {
        let db = setup_test_db().await;
        let pubkey = [1u8; 32];

        // Create account (defaults to free plan)
        let account = db
            .create_account("testuser", &pubkey, "test@example.com")
            .await
            .unwrap();

        // Free tier has one_rental but not unlimited_rentals
        assert!(db
            .account_has_feature(&account.id, "one_rental")
            .await
            .unwrap());
        assert!(db
            .account_has_feature(&account.id, "marketplace_browse")
            .await
            .unwrap());
        assert!(!db
            .account_has_feature(&account.id, "unlimited_rentals")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_account_has_feature_pro_tier() {
        let db = setup_test_db().await;
        let pubkey = [1u8; 32];

        // Create account
        let account = db
            .create_account("testuser", &pubkey, "test@example.com")
            .await
            .unwrap();

        // Upgrade to pro
        db.update_account_subscription(&account.id, "pro", "active", None, None, false)
            .await
            .unwrap();

        // Pro tier has unlimited_rentals
        assert!(db
            .account_has_feature(&account.id, "unlimited_rentals")
            .await
            .unwrap());
        assert!(db
            .account_has_feature(&account.id, "marketplace_browse")
            .await
            .unwrap());
        assert!(!db
            .account_has_feature(&account.id, "one_rental")
            .await
            .unwrap()); // Pro doesn't have one_rental, it has unlimited
    }
}
