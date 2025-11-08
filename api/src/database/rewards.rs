use super::types::{Database, LedgerEntryData};
use anyhow::Result;

impl Database {
    // Reward distributions
    pub(crate) async fn insert_reward_distributions(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // Reward distributions are stored as timestamp (8 bytes) in the value
            // The value contains the timestamp of the distribution
            let distribution_timestamp = if entry.value.len() >= 8 {
                let bytes: [u8; 8] = entry.value[..8].try_into().unwrap_or([0; 8]);
                u64::from_le_bytes(bytes)
            } else {
                entry.block_timestamp_ns
            };

            // Note: For detailed distribution amounts, we would need to query the reward distribution
            // logs or calculate based on the reward logic. For now, we store the timestamp.
            // The actual amounts distributed to providers would be recorded in token_transfers table.
            sqlx::query(
                "INSERT INTO reward_distributions (block_timestamp_ns, total_amount_e9s, providers_count, amount_per_provider_e9s) VALUES (?, ?, ?, ?)"
            )
            .bind(distribution_timestamp as i64)
            .bind(0) // TODO: Calculate from actual reward distribution data
            .bind(0) // TODO: Count actual providers who received rewards
            .bind(0) // TODO: Calculate per-provider amount
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
