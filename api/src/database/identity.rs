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
                sqlx::query(
                    "INSERT INTO linked_ic_ids (pubkey_hash, ic_principal, operation, linked_at_ns) VALUES (?, ?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(principal.to_text())
                .bind("add")
                .bind(entry.block_timestamp_ns as i64)
                .execute(&mut **tx)
                .await?;
            }

            // Insert removed principals
            for principal in linked_ids.alt_principals_rm() {
                sqlx::query(
                    "INSERT INTO linked_ic_ids (pubkey_hash, ic_principal, operation, linked_at_ns) VALUES (?, ?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(principal.to_text())
                .bind("remove")
                .bind(entry.block_timestamp_ns as i64)
                .execute(&mut **tx)
                .await?;
            }
        }
        Ok(())
    }
}
