use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{cache_reputation::ReputationAge, cache_reputation::ReputationChange};
use super::types::{Database, LedgerEntryData};

impl Database {
    // Reputation changes
    pub(crate) async fn insert_reputation_changes(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let change = ReputationChange::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation change: {}", e))?;

            sqlx::query(
                "INSERT INTO reputation_changes (pubkey_hash, change_amount, reason, block_timestamp_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(change.changes()[0].1) // Get the delta amount from first change
            .bind("") // Reason is not stored in the structure, use empty string
            .bind(entry.block_timestamp_ns as i64) // Use actual block timestamp
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reputation aging
    pub(crate) async fn insert_reputation_aging(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let age = ReputationAge::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation age: {}", e))?;

            sqlx::query(
                "INSERT INTO reputation_aging (block_timestamp_ns, aging_factor_ppm) VALUES (?, ?)",
            )
            .bind(entry.block_timestamp_ns as i64)
            .bind(age.reductions_ppm() as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
