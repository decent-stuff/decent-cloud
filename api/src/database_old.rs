use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use dcc_common::{
    CheckInPayload, ContractSignReply, ContractSignRequest, FundsTransfer, 
    FundsTransferApproval, LinkedIdentity, Offerings, Profiles, ReputationAge, 
    ReputationChange, Registration, get_timestamp_ns, IcrcCompatibleAccount
};
use serde_json;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn get_last_sync_position(&self) -> Result<u64> {
        let row = sqlx::query("SELECT last_position FROM sync_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("last_position") as u64)
    }

    pub async fn update_sync_position(&self, position: u64) -> Result<()> {
        sqlx::query("UPDATE sync_state SET last_position = ?, last_sync_at = CURRENT_TIMESTAMP WHERE id = 1")
            .bind(position as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Test helper method to access the underlying pool for test assertions
    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn insert_entries(&self, entries: Vec<LedgerEntryData>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Group entries by label for batch processing
        let mut grouped_entries: HashMap<String, Vec<LedgerEntryData>> = HashMap::new();
        for entry in entries {
            grouped_entries
                .entry(entry.label.clone())
                .or_insert_with(Vec::new)
                .push(entry);
        }

        // Process each group
        for (label, entries) in grouped_entries {
            match label.as_str() {
                "ProvRegister" => self.insert_provider_registrations(&mut tx, entries).await?,
                "ProvCheckIn" => self.insert_provider_check_ins(&mut tx, entries).await?,
                "ProvProfile" => self.insert_provider_profiles(&mut tx, entries).await?,
                "ProvOffering" => self.insert_provider_offerings(&mut tx, entries).await?,
                "DCTokenTransfer" => self.insert_token_transfers(&mut tx, entries).await?,
                "DCTokenApproval" => self.insert_token_approvals(&mut tx, entries).await?,
                "UserRegister" => self.insert_user_registrations(&mut tx, entries).await?,
                "ContractSignReq" => self.insert_contract_sign_requests(&mut tx, entries).await?,
                "ContractSignReply" => self.insert_contract_sign_replies(&mut tx, entries).await?,
                "RepChange" => self.insert_reputation_changes(&mut tx, entries).await?,
                "RepAge" => self.insert_reputation_aging(&mut tx, entries).await?,
                "RewardDistr" => self.insert_reward_distributions(&mut tx, entries).await?,
                "LinkedIcIds" => self.insert_linked_ic_ids(&mut tx, entries).await?,
                _ => {
                    // Unknown label - skip or handle as needed
                    tracing::warn!("Unknown ledger entry label: {}", label);
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    // Provider registrations
    async fn insert_provider_registrations(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            // For now, store raw data since registration is just signature
            sqlx::query(
                "INSERT OR REPLACE INTO provider_registrations (pubkey_hash, pubkey_bytes, signature, created_at_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&entry.key)
            .bind(&entry.value) // Store signature directly
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider check-ins
    async fn insert_provider_check_ins(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let check_in = CheckInPayload::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse check-in: {}", e))?;
            
            sqlx::query(
                "INSERT INTO provider_check_ins (pubkey_hash, memo, nonce_signature, block_timestamp_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(check_in.memo())
            .bind(check_in.nonce_signature())
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider profiles
    async fn insert_provider_profiles(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let profile = Profiles::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse profile: {}", e))?;
            
            // Convert profile to JSON string for storage
            let profile_json = serde_json::to_string(&profile)
                .map_err(|e| anyhow::anyhow!("Failed to serialize profile: {}", e))?;
            
            sqlx::query(
                "INSERT OR REPLACE INTO provider_profiles (pubkey_hash, profile_json, updated_at_ns) VALUES (?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&profile_json)
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider offerings
    async fn insert_provider_offerings(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let offering = Offerings::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse offering: {}", e))?;
            
            // Convert offering to JSON string for storage
            let offering_json = serde_json::to_string(&offering)
                .map_err(|e| anyhow::anyhow!("Failed to serialize offering: {}", e))?;
            
            sqlx::query(
                "INSERT INTO provider_offerings (pubkey_hash, offering_json, created_at_ns) VALUES (?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&offering_json)
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Token transfers
    async fn insert_token_transfers(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let transfer = FundsTransfer::from_bytes(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse transfer: {}", e))?;
            
            sqlx::query(
                "INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at_ns, block_hash, block_offset) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(transfer.from_account().to_string())
            .bind(transfer.to_account().to_string())
            .bind(transfer.amount_e9s() as i64)
            .bind(transfer.fee_e9s() as i64)
            .bind(transfer.memo().map(|m| String::from_utf8_lossy(m)))
            .bind(get_timestamp_ns())
            .bind(transfer.block_hash())
            .bind(transfer.block_offset() as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Token approvals
    async fn insert_token_approvals(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let approval = FundsTransferApproval::deserialize(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse approval: {}", e))?;
            
            sqlx::query(
                "INSERT INTO token_approvals (owner_account, spender_account, amount_e9s, expires_at_ns, created_at_ns) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(approval.owner_account().to_string())
            .bind(approval.spender_account().to_string())
            .bind(approval.amount_e9s() as i64)
            .bind(approval.expires_at())
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // User registrations
    async fn insert_user_registrations(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            // For now, store raw data since registration is just signature
            sqlx::query(
                "INSERT OR REPLACE INTO user_registrations (pubkey_hash, pubkey_bytes, signature, created_at_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&entry.key)
            .bind(&entry.value) // Store signature directly
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Contract sign requests
    async fn insert_contract_sign_requests(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let request = ContractSignRequest::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse contract sign request: {}", e))?;
            
            // Convert request to JSON string for storage
            let request_json = serde_json::to_string(&request)
                .map_err(|e| anyhow::anyhow!("Failed to serialize contract sign request: {}", e))?;
            
            sqlx::query(
                "INSERT INTO contract_sign_requests (pubkey_hash, contract_json, created_at_ns) VALUES (?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&request_json)
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Contract sign replies
    async fn insert_contract_sign_replies(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let reply = ContractSignReply::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse contract sign reply: {}", e))?;
            
            // Convert reply to JSON string for storage
            let reply_json = serde_json::to_string(&reply)
                .map_err(|e| anyhow::anyhow!("Failed to serialize contract sign reply: {}", e))?;
            
            sqlx::query(
                "INSERT INTO contract_sign_replies (request_id, pubkey_hash, reply_json, created_at_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(reply.request_id() as i64)
            .bind(&entry.key)
            .bind(&reply_json)
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reputation changes
    async fn insert_reputation_changes(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let change = ReputationChange::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation change: {}", e))?;
            
            sqlx::query(
                "INSERT INTO reputation_changes (pubkey_hash, change_amount, reason, block_timestamp_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(change.change_amount())
            .bind(change.reason())
            .bind(change.block_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reputation aging
    async fn insert_reputation_aging(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let age = ReputationAge::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation age: {}", e))?;
            
            sqlx::query(
                "INSERT INTO reputation_aging (block_timestamp_ns, aging_factor_ppm) VALUES (?, ?)"
            )
            .bind(age.block_timestamp_ns())
            .bind(age.aging_factor())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reward distributions
    async fn insert_reward_distributions(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            // Reward distributions are stored as a simple timestamp entry
            sqlx::query(
                "INSERT INTO reward_distributions (block_timestamp_ns, total_amount_e9s, providers_count, amount_per_provider_e9s) VALUES (?, ?, ?, ?)"
            )
            .bind(get_timestamp_ns())
            .bind(0) // These would need to be parsed from actual reward data
            .bind(0)
            .bind(0)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Linked IC identities
    async fn insert_linked_ic_ids(&self, tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>, entries: &[LedgerEntryData]) -> Result<()> {
        for entry in entries {
            let linked_ids = LinkedIdentity::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse linked IC IDs: {}", e))?;
            
            sqlx::query(
                "INSERT OR REPLACE INTO linked_ic_ids (pubkey_hash, ic_principal, linked_at_ns) VALUES (?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(linked_ids.ic_principal().to_string())
            .bind(get_timestamp_ns())
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
        match entry.label.as_str() {
            "ProvRegister" => {
                // Registration stores the crypto signature as value, pubkey as key
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT OR REPLACE INTO provider_registrations (pubkey_hash, pubkey_bytes, created_at) VALUES (?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(&entry.key)
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "ProvCheckIn" => {
                // CheckIn payload - store the raw data for now
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO provider_check_ins (pubkey_hash, memo, nonce_signature, block_timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(&entry.key)
                .bind("") // Placeholder memo
                .bind(&entry.value) // Store the raw payload
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "ProvProfile" => {
                // Profile contains the actual profile data
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT OR REPLACE INTO provider_profiles (pubkey_hash, profile_data, updated_at) VALUES (?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(&entry.value)
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "ProvOffering" => {
                // Offering contains the offering data
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO provider_offerings (pubkey_hash, offering_data, created_at) VALUES (?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(&entry.value)
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "DCTokenTransfer" => {
                // Token transfer - store raw data for now
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO token_transfers (from_account, to_account, amount_e9s, fee_e9s, memo, created_at) VALUES (?, ?, ?, ?, ?, ?)"
                )
                .bind("") // Placeholder from account
                .bind("") // Placeholder to account
                .bind(0) // Placeholder amount
                .bind(0) // Placeholder fee
                .bind("") // Placeholder memo
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "DCTokenApproval" => {
                // Token approval - store raw data for now
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO token_approvals (spender_account, amount_e9s, expires_at, created_at) VALUES (?, ?, ?, ?)"
                )
                .bind("") // Placeholder spender account
                .bind(0) // Placeholder amount
                .bind(0) // Placeholder expires at
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "UserRegister" => {
                // User registration stores crypto signature as value, pubkey as key
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT OR REPLACE INTO user_registrations (pubkey_hash, pubkey_bytes, created_at) VALUES (?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(&entry.key)
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "ContractSignReq" => {
                // Contract sign request data
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO contract_sign_requests (pubkey_hash, contract_data, created_at) VALUES (?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(&entry.value)
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "ContractSignReply" => {
                // Contract sign reply data
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO contract_sign_replies (request_id, pubkey_hash, reply_data, created_at) VALUES (?, ?, ?, ?)"
                )
                .bind(0) // Placeholder request ID
                .bind(&entry.key)
                .bind(&entry.value)
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "RepChange" => {
                // Reputation change - store raw data for now
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO reputation_changes (pubkey_hash, change_amount, reason, block_timestamp) VALUES (?, ?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(0) // Placeholder change amount
                .bind("") // Placeholder reason
                .bind(current_time)
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            "RepAge" => {
                // Reputation aging - store raw data for now
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                sqlx::query(
                    "INSERT INTO reputation_aging (block_timestamp, aging_factor) VALUES (?, ?)"
                )
                .bind(current_time)
                .bind(0) // Placeholder aging factor
                .execute(&mut **tx)
                .await?;
                Ok(true)
            }
            
            _ => Ok(false) // Not a structured entry, fallback to generic storage
        }
    }
}

#[derive(Clone)]
pub struct LedgerEntryData {
    pub label: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}
