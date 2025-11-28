use super::types::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct PlatformStats {
    #[ts(type = "number")]
    pub total_providers: i64,
    #[ts(type = "number")]
    pub active_providers: i64,
    #[ts(type = "number")]
    pub total_offerings: i64,
    #[ts(type = "number")]
    pub total_contracts: i64,
    #[ts(type = "number")]
    pub total_transfers: i64,
    #[ts(type = "number")]
    pub total_volume_e9s: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, poem_openapi::Object)]
pub struct ReputationInfo {
    #[oai(skip)]
    pub pubkey: Vec<u8>,
    pub total_reputation: i64,
    pub change_count: i64,
}

impl Database {
    /// Get the latest block timestamp from provider check-ins
    pub async fn get_latest_block_timestamp_ns(&self) -> Result<Option<i64>> {
        let result = sqlx::query_scalar!("SELECT MAX(block_timestamp_ns) FROM provider_check_ins")
            .fetch_one(&self.pool)
            .await?;
        Ok(result)
    }

    /// Get platform-wide statistics
    pub async fn get_platform_stats(&self) -> Result<PlatformStats> {
        // Total providers = all who have ever checked in or created a profile
        // Exclude the example provider used for template generation
        let example_provider_hash =
            hex::decode("6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572")
                .unwrap();
        let example_provider_hash_for_profiles = example_provider_hash.clone();
        let example_provider_hash_for_checkins = example_provider_hash.clone();
        let total_providers = sqlx::query_scalar!(
            r#"SELECT COUNT(DISTINCT pubkey) FROM (
                SELECT pubkey FROM provider_profiles WHERE pubkey != ?
                UNION
                SELECT pubkey FROM provider_check_ins WHERE pubkey != ?
            )"#,
            example_provider_hash_for_profiles,
            example_provider_hash_for_checkins
        )
        .fetch_one(&self.pool)
        .await?;

        // Active in the last year
        let cutoff_ns =
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) - 365 * 24 * 3600 * 1_000_000_000;
        let example_provider_hash_active = example_provider_hash.clone();
        let active_providers = sqlx::query_scalar!(
            "SELECT COUNT(DISTINCT pubkey) FROM provider_check_ins WHERE block_timestamp_ns > ? AND (pubkey) != ?",
            cutoff_ns,
            example_provider_hash_active
        )
        .fetch_one(&self.pool)
        .await?;

        let example_provider_hash_offerings = example_provider_hash.clone();
        let total_offerings = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM provider_offerings WHERE LOWER(visibility) = 'public' AND pubkey != ?",
            example_provider_hash_offerings
        )
        .fetch_one(&self.pool)
        .await?;

        let total_contracts = sqlx::query_scalar!("SELECT COUNT(*) FROM contract_sign_requests")
            .fetch_one(&self.pool)
            .await?;

        let total_transfers = sqlx::query_scalar!("SELECT COUNT(*) FROM token_transfers")
            .fetch_one(&self.pool)
            .await?;

        let total_volume: Option<i64> =
            sqlx::query_scalar!("SELECT SUM(amount_e9s) FROM token_transfers")
                .fetch_one(&self.pool)
                .await?;

        Ok(PlatformStats {
            total_providers,
            active_providers,
            total_offerings,
            total_contracts,
            total_transfers,
            total_volume_e9s: total_volume.unwrap_or(0),
        })
    }

    /// Get reputation for an identity
    pub async fn get_reputation(&self, pubkey: &[u8]) -> Result<Option<ReputationInfo>> {
        let info = sqlx::query_as!(
            ReputationInfo,
            r#"SELECT pubkey, COALESCE(SUM(change_amount), 0) as "total_reputation!: i64", COUNT(*) as "change_count!: i64"
             FROM reputation_changes
             WHERE pubkey = ?
             GROUP BY pubkey"#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(info)
    }

    /// Get top providers by reputation
    #[allow(dead_code)]
    pub async fn get_top_providers_by_reputation(&self, limit: i64) -> Result<Vec<ReputationInfo>> {
        let top = sqlx::query_as!(
            ReputationInfo,
            r#"SELECT pubkey, COALESCE(SUM(change_amount), 0) as "total_reputation!: i64", COUNT(*) as "change_count!: i64"
             FROM reputation_changes
             GROUP BY pubkey
             ORDER BY COALESCE(SUM(change_amount), 0) DESC
             LIMIT ?"#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(top)
    }

    /// Get contract stats for a provider
    pub async fn get_provider_stats(&self, pubkey: &[u8]) -> Result<ProviderStats> {
        let total_contracts: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM contract_sign_requests WHERE provider_pubkey = ?",
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        let pending_contracts: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM contract_sign_requests WHERE provider_pubkey = ? AND status = 'pending'",
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        let total_revenue: i64 = sqlx::query_scalar!(
            "SELECT COALESCE(SUM(payment_amount_e9s), 0) FROM contract_sign_requests WHERE provider_pubkey = ?",
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        let offerings_count: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM provider_offerings WHERE pubkey = ?",
            pubkey
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ProviderStats {
            total_contracts,
            pending_contracts,
            total_revenue_e9s: total_revenue,
            offerings_count,
        })
    }

    /// Search accounts by username, display name, or public key
    pub async fn search_accounts(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<AccountSearchResult>> {
        // Prepare search pattern for LIKE queries
        let search_pattern = format!("%{}%", query.to_lowercase());
        let hex_search_pattern = format!("{}%", query.to_uppercase());

        #[derive(sqlx::FromRow)]
        struct SearchRow {
            username: String,
            display_name: Option<String>,
            pubkey: String,
            reputation_score: i64,
            contract_count: i64,
            offering_count: i64,
        }

        let results = sqlx::query_as::<_, SearchRow>(
            r#"SELECT DISTINCT
                a.username,
                a.display_name,
                hex(apk.public_key) as pubkey,
                COALESCE(rep.total_reputation, 0) as reputation_score,
                COALESCE(contracts.contract_count, 0) as contract_count,
                COALESCE(offerings.offering_count, 0) as offering_count
            FROM accounts a
            INNER JOIN account_public_keys apk ON a.id = apk.account_id
            LEFT JOIN (
                SELECT pubkey, SUM(change_amount) as total_reputation
                FROM reputation_changes
                GROUP BY pubkey
            ) rep ON apk.public_key = rep.pubkey
            LEFT JOIN (
                SELECT provider_pubkey as pubkey, COUNT(*) as contract_count
                FROM contract_sign_requests
                GROUP BY provider_pubkey
                UNION ALL
                SELECT requester_pubkey as pubkey, COUNT(*) as contract_count
                FROM contract_sign_requests
                GROUP BY requester_pubkey
            ) contracts ON apk.public_key = contracts.pubkey
            LEFT JOIN (
                SELECT pubkey, COUNT(*) as offering_count
                FROM provider_offerings
                GROUP BY pubkey
            ) offerings ON apk.public_key = offerings.pubkey
            WHERE apk.is_active = 1
              AND (
                lower(a.username) LIKE ?
                OR lower(a.display_name) LIKE ?
                OR hex(apk.public_key) LIKE ?
              )
            GROUP BY a.username, a.display_name, apk.public_key
            ORDER BY reputation_score DESC, contract_count DESC, offering_count DESC
            LIMIT ?"#,
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&hex_search_pattern)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(results
            .into_iter()
            .map(|row| AccountSearchResult {
                username: row.username,
                display_name: row.display_name,
                pubkey: row.pubkey,
                reputation_score: row.reputation_score,
                contract_count: row.contract_count,
                offering_count: row.offering_count,
            })
            .collect())
    }
}

#[derive(Debug, Serialize, Deserialize, poem_openapi::Object)]
pub struct ProviderStats {
    pub total_contracts: i64,
    pub pending_contracts: i64,
    pub total_revenue_e9s: i64,
    pub offerings_count: i64,
}

/// Account search result with reputation and activity stats
#[derive(Debug, Serialize, Deserialize, poem_openapi::Object, TS)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
pub struct AccountSearchResult {
    pub username: String,
    #[oai(skip_serializing_if_is_none)]
    pub display_name: Option<String>,
    pub pubkey: String,
    #[ts(type = "number")]
    pub reputation_score: i64,
    #[ts(type = "number")]
    pub contract_count: i64,
    #[ts(type = "number")]
    pub offering_count: i64,
}

#[cfg(test)]
mod tests;
