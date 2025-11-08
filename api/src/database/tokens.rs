use anyhow::Result;
use dcc_common::{FundsTransfer, FundsTransferApproval};
use super::types::{Database, LedgerEntryData};

impl Database {
    // Token transfers
    pub(crate) async fn insert_token_transfers(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let transfer = FundsTransfer::from_bytes(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse transfer: {}", e))?;

            sqlx::query(
                "INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns, block_hash, block_offset) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(transfer.from().to_string())
            .bind(transfer.to().to_string())
            .bind(transfer.amount() as i64)
            .bind(transfer.fee().unwrap_or(0) as i64)
            .bind(String::from_utf8_lossy(transfer.memo()).to_string())
            .bind(entry.block_timestamp_ns as i64)
            .bind(&entry.block_hash)
            .bind(entry.block_offset as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Token approvals
    pub(crate) async fn insert_token_approvals(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let approval = FundsTransferApproval::deserialize(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse approval: {}", e))?;

            sqlx::query(
                "INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(approval.approver().to_string())
            .bind(approval.spender().to_string())
            .bind(approval.allowance().allowance.0.to_string().parse::<i64>().unwrap_or(0))
            .bind(approval.allowance().expires_at.map(|v| v as i64))
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}
