use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::linked_identity::LinkedIcIdsRecord;

impl Database {
    // Linked IC identities
    pub(crate) async fn insert_linked_ic_ids(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let linked_ids = LinkedIcIdsRecord::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse linked IC IDs: {}", e))?;

            // Insert added principals
            for principal in linked_ids.alt_principals_add() {
                let timestamp_i64 = entry.block_timestamp_ns as i64;
                let principal_text = principal.to_text();
                sqlx::query!(
                    "INSERT INTO linked_ic_ids (pubkey, ic_principal, operation, linked_at_ns) VALUES (?, ?, ?, ?)",
                    entry.key,
                    principal_text,
                    "add",
                    timestamp_i64
                )
                .execute(&mut **tx)
                .await?;
            }

            // Insert removed principals
            for principal in linked_ids.alt_principals_rm() {
                let timestamp_i64 = entry.block_timestamp_ns as i64;
                let principal_text = principal.to_text();
                sqlx::query!(
                    "INSERT INTO linked_ic_ids (pubkey, ic_principal, operation, linked_at_ns) VALUES (?, ?, ?, ?)",
                    entry.key,
                    principal_text,
                    "remove",
                    timestamp_i64
                )
                .execute(&mut **tx)
                .await?;
            }
        }
        Ok(())
    }
}
