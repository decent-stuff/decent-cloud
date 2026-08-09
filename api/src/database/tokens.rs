use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use dcc_common::{FundsTransfer, FundsTransferApproval};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct TokenApproval {
    pub owner_account: String,
    pub spender_account: String,
    pub amount_e9s: i64,
    pub expires_at_ns: Option<i64>,
    pub created_at_ns: i64,
}

impl Database {
    /// Get token approvals for an account
    #[allow(dead_code)]
    pub async fn get_account_approvals(&self, account: &str) -> Result<Vec<TokenApproval>> {
        let approvals = sqlx::query_as!(
            TokenApproval,
            r#"SELECT owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns
             FROM token_approvals
             WHERE owner_account = $1 OR spender_account = $2
             ORDER BY created_at_ns DESC"#,
            account,
            account
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(approvals)
    }
    // Token transfers
    pub(crate) async fn insert_token_transfers(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let transfer = FundsTransfer::from_bytes(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse transfer: {}", e))?;

            let from_account = transfer.from().to_string();
            let to_account = transfer.to().to_string();
            let amount_i64 = transfer.amount() as i64;
            let fee_i64 = transfer.fee().unwrap_or(0) as i64;
            // Filter out NULL bytes (0x00) which PostgreSQL TEXT type doesn't accept
            let memo = String::from_utf8_lossy(transfer.memo())
                .replace('\0', "")
                .to_string();
            let timestamp_i64 = entry.block_timestamp_ns as i64;
            let block_offset_i64 = entry.block_offset as i64;

            sqlx::query!(
                "INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns, block_hash, block_offset) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
                from_account,
                to_account,
                amount_i64,
                fee_i64,
                memo,
                timestamp_i64,
                entry.block_hash,
                block_offset_i64
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Token approvals
    pub(crate) async fn insert_token_approvals(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let approval = FundsTransferApproval::deserialize(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse approval: {}", e))?;

            let approver = approval.approver().to_string();
            let spender = approval.spender().to_string();
            let amount_e9s = approval
                .allowance()
                .allowance
                .0
                .to_string()
                .parse::<i64>()
                .unwrap_or(0);
            let expires_at = approval.allowance().expires_at.map(|v| v as i64);
            let timestamp_i64 = entry.block_timestamp_ns as i64;

            sqlx::query!(
                "INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES ($1, $2, $3, $4, $5)",
                approver,
                spender,
                amount_e9s,
                expires_at,
                timestamp_i64
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
