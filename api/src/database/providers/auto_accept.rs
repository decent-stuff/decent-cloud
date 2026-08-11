use super::*;
use crate::database::types::Database;
use anyhow::Result;

impl Database {
    /// Check if provider has auto-accept rentals enabled.
    ///
    /// Returns the provider's `auto_accept_rentals` setting, or the **schema
    /// default of `TRUE`** when no `provider_profiles` row exists for this
    /// pubkey. The TRUE default is load-bearing for cloud-resell (Model B)
    /// offerings: they are operator-provisioned with no human provider to
    /// accept/reject, so a rental against an un-onboarded provider pubkey must
    /// still auto-advance past `requested`. The column itself defaults to TRUE
    /// (`migrations_pg/001_schema.sql`), so this lookup must agree. A provider
    /// opts out by inserting a profile row with `auto_accept_rentals = FALSE`.
    pub async fn get_provider_auto_accept_rentals(&self, pubkey: &[u8]) -> Result<bool> {
        let row = sqlx::query_scalar!(
            "SELECT auto_accept_rentals FROM provider_profiles WHERE pubkey = $1",
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        // No profile row → schema default (TRUE); otherwise honor the stored value.
        Ok(row.unwrap_or(true))
    }

    /// Set provider auto-accept rentals setting.
    /// Updates the provider_profiles table. Returns error if provider doesn't exist.
    pub async fn set_provider_auto_accept_rentals(
        &self,
        pubkey: &[u8],
        enabled: bool,
    ) -> Result<()> {
        let result = sqlx::query!(
            "UPDATE provider_profiles SET auto_accept_rentals = $1 WHERE pubkey = $2",
            enabled,
            pubkey
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("Provider profile not found"));
        }

        Ok(())
    }

    /// Create a per-offering auto-accept rule for a provider.
    /// Returns error if a rule already exists for this provider+offering pair.
    /// Returns error if min_duration_hours > max_duration_hours (when both are set).
    pub async fn create_auto_accept_rule(
        &self,
        provider_pubkey: &[u8],
        offering_id: &str,
        min_duration_hours: Option<i64>,
        max_duration_hours: Option<i64>,
    ) -> Result<AutoAcceptRule> {
        if let (Some(min), Some(max)) = (min_duration_hours, max_duration_hours) {
            anyhow::ensure!(
                min <= max,
                "min_duration_hours ({min}) must not exceed max_duration_hours ({max})"
            );
        }
        let row = sqlx::query_as!(
            AutoAcceptRule,
            r#"INSERT INTO auto_accept_rules (provider_pubkey, offering_id, min_duration_hours, max_duration_hours)
               VALUES ($1, $2, $3, $4)
               RETURNING id as "id!", offering_id, min_duration_hours, max_duration_hours, enabled"#,
            provider_pubkey,
            offering_id,
            min_duration_hours,
            max_duration_hours,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// List all auto-accept rules for a provider.
    pub async fn list_auto_accept_rules(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<AutoAcceptRule>> {
        let rows = sqlx::query_as!(
            AutoAcceptRule,
            r#"SELECT id as "id!", offering_id, min_duration_hours, max_duration_hours, enabled
               FROM auto_accept_rules WHERE provider_pubkey = $1 ORDER BY id"#,
            provider_pubkey,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Update an existing auto-accept rule. The rule must belong to the given provider.
    /// Returns error if min_duration_hours > max_duration_hours (when both are set).
    pub async fn update_auto_accept_rule(
        &self,
        provider_pubkey: &[u8],
        rule_id: i64,
        min_duration_hours: Option<i64>,
        max_duration_hours: Option<i64>,
        enabled: bool,
    ) -> Result<AutoAcceptRule> {
        if let (Some(min), Some(max)) = (min_duration_hours, max_duration_hours) {
            anyhow::ensure!(
                min <= max,
                "min_duration_hours ({min}) must not exceed max_duration_hours ({max})"
            );
        }
        let row = sqlx::query_as!(
            AutoAcceptRule,
            r#"UPDATE auto_accept_rules
               SET min_duration_hours = $1, max_duration_hours = $2, enabled = $3, updated_at = NOW()
               WHERE id = $4 AND provider_pubkey = $5
               RETURNING id as "id!", offering_id, min_duration_hours, max_duration_hours, enabled"#,
            min_duration_hours,
            max_duration_hours,
            enabled,
            rule_id,
            provider_pubkey,
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Auto-accept rule {rule_id} not found for this provider"))?;
        Ok(row)
    }

    /// Delete an auto-accept rule. The rule must belong to the given provider.
    /// Returns error if the rule does not exist for this provider.
    pub async fn delete_auto_accept_rule(
        &self,
        provider_pubkey: &[u8],
        rule_id: i64,
    ) -> Result<()> {
        let result = sqlx::query!(
            "DELETE FROM auto_accept_rules WHERE id = $1 AND provider_pubkey = $2",
            rule_id,
            provider_pubkey,
        )
        .execute(&self.pool)
        .await?;
        anyhow::ensure!(
            result.rows_affected() > 0,
            "Auto-accept rule {rule_id} not found for this provider"
        );
        Ok(())
    }

    /// Check whether a contract request matches the provider's auto-accept rules for the given offering.
    ///
    /// Returns true (should auto-accept) when:
    ///   - No rule exists for this offering → accept all (backward-compatible default)
    ///   - A matching enabled rule exists and the duration falls within [min, max]
    ///
    /// Returns false (should NOT auto-accept) when:
    ///   - A rule exists but is disabled
    ///   - A rule exists and the duration is outside [min, max]
    pub async fn check_auto_accept_rule_matches(
        &self,
        provider_pubkey: &[u8],
        offering_id: &str,
        duration_hours: Option<i64>,
    ) -> Result<bool> {
        let row = sqlx::query!(
            r#"SELECT min_duration_hours, max_duration_hours, enabled
               FROM auto_accept_rules WHERE provider_pubkey = $1 AND offering_id = $2"#,
            provider_pubkey,
            offering_id,
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(rule) = row else {
            // No rule for this offering → accept all
            return Ok(true);
        };

        if !rule.enabled {
            return Ok(false);
        }

        let hours = duration_hours.unwrap_or(0);
        if let Some(min) = rule.min_duration_hours {
            if hours < min {
                return Ok(false);
            }
        }
        if let Some(max) = rule.max_duration_hours {
            if hours > max {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
