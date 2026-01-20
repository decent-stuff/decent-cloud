use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{cache_reputation::ReputationAge, cache_reputation::ReputationChange};

impl Database {
    // Reputation changes
    pub(crate) async fn insert_reputation_changes(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let change = ReputationChange::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation change: {}", e))?;

            let timestamp_i64 = entry.block_timestamp_ns as i64;
            let delta_amount = change
                .changes()
                .first()
                .map(|(_, delta)| *delta)
                .ok_or_else(|| anyhow::anyhow!("Reputation change has no entries"))?;

            sqlx::query!(
                "INSERT INTO reputation_changes (pubkey, change_amount, reason, block_timestamp_ns) VALUES ($1, $2, $3, $4)",
                entry.key,
                delta_amount,
                "", // Reason is not stored in structure, use empty string
                timestamp_i64 // Use actual block timestamp
            )

            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reputation aging
    pub(crate) async fn insert_reputation_aging(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let age = ReputationAge::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation age: {}", e))?;

            let timestamp_i64 = entry.block_timestamp_ns as i64;
            let aging_factor = age.reductions_ppm() as i64;

            sqlx::query!(
                "INSERT INTO reputation_aging (block_timestamp_ns, aging_factor_ppm) VALUES ($1, $2)",
                timestamp_i64,
                aging_factor
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
