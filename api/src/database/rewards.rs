use super::types::{Database, LedgerEntryData};
use anyhow::Result;

impl Database {
    // Reward distributions
    pub(crate) async fn insert_reward_distributions(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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
            let timestamp_i64 = distribution_timestamp as i64;

            // Calculate reward distribution statistics from token transfers
            //
            // The blockchain stores reward distributions as timestamp entries. The actual
            // reward amounts are distributed via individual token transfers from MINTING_ACCOUNT
            // to eligible providers. These token transfers are created in the same transaction
            // immediately after the reward distribution entry.
            //
            // Since we're processing entries sequentially and token transfers haven't been
            // inserted yet, we can't query the token_transfers table. Instead, we store
            // placeholder values (0) for the summary statistics.
            //
            // To get accurate reward distribution statistics, query token_transfers table:
            //   SELECT COUNT(*) as providers_count,
            //          SUM(amount_e9s) as total_amount_e9s,
            //          AVG(amount_e9s) as amount_per_provider_e9s
            //   FROM token_transfers
            //   WHERE from_account = 'MINTING_ACCOUNT'
            //     AND created_at_ns >= <distribution_timestamp>
            //     AND created_at_ns < <next_distribution_timestamp>

            sqlx::query!(
                "INSERT INTO reward_distributions (block_timestamp_ns, total_amount_e9s, providers_count, amount_per_provider_e9s) VALUES ($1, $2, $3, $4)",
                timestamp_i64,
                0i64,  // total_amount_e9s: calculated from token_transfers
                0i32,  // providers_count: calculated from token_transfers
                0i64   // amount_per_provider_e9s: calculated from token_transfers
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
