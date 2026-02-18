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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;
    use sqlx::Row;

    fn make_entry(label: &str, key: &[u8], value: &[u8], timestamp: u64) -> LedgerEntryData {
        LedgerEntryData {
            label: label.to_string(),
            key: key.to_vec(),
            value: value.to_vec(),
            block_timestamp_ns: timestamp,
            block_hash: vec![0; 3],
            block_offset: 0,
        }
    }

    #[tokio::test]
    async fn test_insert_reward_distribution_with_timestamp_in_value() {
        let db = setup_test_db().await;
        let encoded_ts: u64 = 9_999_000_000;
        let block_ts: u64 = 1_000_000_000;

        let entry = make_entry(
            "RewardDistr",
            b"dist_key",
            &encoded_ts.to_le_bytes(),
            block_ts,
        );
        db.insert_entries(vec![entry]).await.unwrap();

        let row = sqlx::query("SELECT block_timestamp_ns FROM reward_distributions")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let stored_ts: i64 = row.get("block_timestamp_ns");
        assert_eq!(
            stored_ts, encoded_ts as i64,
            "Should store the timestamp decoded from value, not block_timestamp_ns"
        );
    }

    #[tokio::test]
    async fn test_insert_reward_distribution_short_value_uses_block_timestamp() {
        let db = setup_test_db().await;
        let block_ts: u64 = 5_555_000_000;

        // Value shorter than 8 bytes forces fallback to block_timestamp_ns
        let entry = make_entry("RewardDistr", b"dist_key", &[1, 2, 3], block_ts);
        db.insert_entries(vec![entry]).await.unwrap();

        let row = sqlx::query("SELECT block_timestamp_ns FROM reward_distributions")
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let stored_ts: i64 = row.get("block_timestamp_ns");
        assert_eq!(
            stored_ts, block_ts as i64,
            "Short value should cause block_timestamp_ns to be stored"
        );
    }

    #[tokio::test]
    async fn test_insert_reward_distribution_placeholder_values_are_zero() {
        let db = setup_test_db().await;
        let ts: u64 = 7_777_000_000;

        let entry = make_entry("RewardDistr", b"dist_key", &ts.to_le_bytes(), ts);
        db.insert_entries(vec![entry]).await.unwrap();

        let row = sqlx::query(
            "SELECT total_amount_e9s, providers_count, amount_per_provider_e9s FROM reward_distributions",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();

        assert_eq!(row.get::<i64, _>("total_amount_e9s"), 0i64);
        assert_eq!(row.get::<i32, _>("providers_count"), 0i32);
        assert_eq!(row.get::<i64, _>("amount_per_provider_e9s"), 0i64);
    }

    #[tokio::test]
    async fn test_insert_reward_distributions_batch() {
        let db = setup_test_db().await;

        let entries: Vec<LedgerEntryData> = (0..5)
            .map(|i| {
                let ts: u64 = 1_000_000_000 + i * 1_000_000;
                make_entry("RewardDistr", b"dist_key", &ts.to_le_bytes(), 0)
            })
            .collect();

        db.insert_entries(entries).await.unwrap();

        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM reward_distributions")
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(count.0, 5, "All 5 reward distributions should be stored");
    }
}
