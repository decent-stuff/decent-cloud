use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use dcc_common::{FundsTransfer, FundsTransferApproval};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
#[oai(skip_serializing_if_is_none)]
pub struct TokenTransfer {
    pub from_account: String,
    pub to_account: String,
    pub amount_e9s: i64,
    pub fee_e9s: i64,
    #[oai(skip_serializing_if_is_none)]
    pub memo: Option<String>,
    pub created_at_ns: i64,
}

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
    /// Get token transfers for an account
    pub async fn get_account_transfers(
        &self,
        account: &str,
        limit: i64,
    ) -> Result<Vec<TokenTransfer>> {
        let transfers = sqlx::query_as::<_, TokenTransfer>(
            "SELECT from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns
             FROM token_transfers
             WHERE from_account = ? OR to_account = ?
             ORDER BY created_at_ns DESC LIMIT ?",
        )
        .bind(account)
        .bind(account)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(transfers)
    }

    /// Get recent token transfers
    pub async fn get_recent_transfers(&self, limit: i64) -> Result<Vec<TokenTransfer>> {
        let transfers = sqlx::query_as::<_, TokenTransfer>(
            "SELECT from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns
             FROM token_transfers
             ORDER BY created_at_ns DESC LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(transfers)
    }

    /// Get account balance (sum of all transfers)
    pub async fn get_account_balance(&self, account: &str) -> Result<i64> {
        let received: (Option<i64>,) =
            sqlx::query_as("SELECT SUM(amount_e9s) FROM token_transfers WHERE to_account = ?")
                .bind(account)
                .fetch_one(&self.pool)
                .await?;

        let sent: (Option<i64>,) = sqlx::query_as(
            "SELECT SUM(amount_e9s + fee_e9s) FROM token_transfers WHERE from_account = ?",
        )
        .bind(account)
        .fetch_one(&self.pool)
        .await?;

        Ok(received.0.unwrap_or(0) - sent.0.unwrap_or(0))
    }

    /// Get token approvals for an account
    #[allow(dead_code)]
    pub async fn get_account_approvals(&self, account: &str) -> Result<Vec<TokenApproval>> {
        let approvals = sqlx::query_as::<_, TokenApproval>(
            "SELECT owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns
             FROM token_approvals
             WHERE owner_account = ? OR spender_account = ?
             ORDER BY created_at_ns DESC",
        )
        .bind(account)
        .bind(account)
        .fetch_all(&self.pool)
        .await?;

        Ok(approvals)
    }
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

#[cfg(test)]
mod tests;
