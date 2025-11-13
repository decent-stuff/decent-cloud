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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ReputationInfo {
    pub pubkey_hash: Vec<u8>,
    pub total_reputation: i64,
    pub change_count: i64,
}

impl Database {
    /// Get the latest block timestamp from provider check-ins
    pub async fn get_latest_block_timestamp_ns(&self) -> Result<Option<i64>> {
        let result: (Option<i64>,) =
            sqlx::query_as("SELECT MAX(block_timestamp_ns) FROM provider_check_ins")
                .fetch_one(&self.pool)
                .await?;
        Ok(result.0)
    }

    /// Get platform-wide statistics
    pub async fn get_platform_stats(&self) -> Result<PlatformStats> {
        // Total providers = all who have ever checked in or created a profile
        // Exclude the example provider used for template generation
        let example_provider_hash =
            hex::decode("6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572")
                .unwrap();
        let total_providers: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT pubkey_hash) FROM (
                SELECT pubkey_hash FROM provider_profiles WHERE pubkey_hash != ?
                UNION
                SELECT pubkey_hash FROM provider_check_ins WHERE pubkey_hash != ?
            )",
        )
        .bind(&example_provider_hash)
        .bind(&example_provider_hash)
        .fetch_one(&self.pool)
        .await?;

        // Active in the last year
        let cutoff_ns =
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0) - 365 * 24 * 3600 * 1_000_000_000;
        let active_providers: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT pubkey_hash) FROM provider_check_ins WHERE block_timestamp_ns > ? AND pubkey_hash != ?"
        )
        .bind(cutoff_ns)
        .bind(&example_provider_hash)
        .fetch_one(&self.pool)
        .await?;

        let total_offerings: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM provider_offerings WHERE LOWER(visibility) = 'public'",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_contracts: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM contract_sign_requests")
            .fetch_one(&self.pool)
            .await?;

        let total_transfers: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM token_transfers")
            .fetch_one(&self.pool)
            .await?;

        let total_volume: (Option<i64>,) =
            sqlx::query_as("SELECT SUM(amount_e9s) FROM token_transfers")
                .fetch_one(&self.pool)
                .await?;

        Ok(PlatformStats {
            total_providers: total_providers.0,
            active_providers: active_providers.0,
            total_offerings: total_offerings.0,
            total_contracts: total_contracts.0,
            total_transfers: total_transfers.0,
            total_volume_e9s: total_volume.0.unwrap_or(0),
        })
    }

    /// Get reputation for an identity
    pub async fn get_reputation(&self, pubkey_hash: &[u8]) -> Result<Option<ReputationInfo>> {
        let info = sqlx::query_as::<_, ReputationInfo>(
            "SELECT pubkey_hash, SUM(change_amount) as total_reputation, COUNT(*) as change_count
             FROM reputation_changes
             WHERE pubkey_hash = ?
             GROUP BY pubkey_hash",
        )
        .bind(pubkey_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(info)
    }

    /// Get top providers by reputation
    #[allow(dead_code)]
    pub async fn get_top_providers_by_reputation(&self, limit: i64) -> Result<Vec<ReputationInfo>> {
        let top = sqlx::query_as::<_, ReputationInfo>(
            "SELECT pubkey_hash, SUM(change_amount) as total_reputation, COUNT(*) as change_count
             FROM reputation_changes
             GROUP BY pubkey_hash
             ORDER BY total_reputation DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(top)
    }

    /// Get contract stats for a provider
    pub async fn get_provider_stats(&self, pubkey_hash: &[u8]) -> Result<ProviderStats> {
        let total_contracts: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM contract_sign_requests WHERE provider_pubkey_hash = ?",
        )
        .bind(pubkey_hash)
        .fetch_one(&self.pool)
        .await?;

        let pending_contracts: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM contract_sign_requests WHERE provider_pubkey_hash = ? AND status = 'pending'"
        )
        .bind(pubkey_hash)
        .fetch_one(&self.pool)
        .await?;

        let total_revenue: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(payment_amount_e9s) FROM contract_sign_requests WHERE provider_pubkey_hash = ?"
        )
        .bind(pubkey_hash)
        .fetch_one(&self.pool)
        .await?;

        let offerings_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM provider_offerings WHERE pubkey_hash = ?")
                .bind(pubkey_hash)
                .fetch_one(&self.pool)
                .await?;

        Ok(ProviderStats {
            total_contracts: total_contracts.0,
            pending_contracts: pending_contracts.0,
            total_revenue_e9s: total_revenue.0.unwrap_or(0),
            offerings_count: offerings_count.0,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderStats {
    pub total_contracts: i64,
    pub pending_contracts: i64,
    pub total_revenue_e9s: i64,
    pub offerings_count: i64,
}

#[cfg(test)]
mod tests;
