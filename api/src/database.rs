use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{
    cache_reputation::ReputationAge, cache_reputation::ReputationChange,
    linked_identity::LinkedIcIdsRecord, offerings, CheckInPayload, ContractSignReply,
    ContractSignRequest, FundsTransfer, FundsTransferApproval, UpdateProfilePayload,
    DC_TOKEN_DECIMALS_DIV,
};
use serde_json;
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LedgerEntryData {
    pub label: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
    pub block_timestamp_ns: u64,
    pub block_hash: Vec<u8>,
    pub block_offset: u64,
}

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
        sqlx::query("UPDATE sync_state SET last_position = ? WHERE id = 1")
            .bind(position as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
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
                "ProvRegister" => {
                    self.insert_provider_registrations(&mut tx, &entries)
                        .await?
                }
                "ProvCheckIn" => self.insert_provider_check_ins(&mut tx, &entries).await?,
                "ProvProfile" => self.insert_provider_profiles(&mut tx, &entries).await?,
                "ProvOffering" => self.insert_provider_offerings(&mut tx, &entries).await?,
                "DCTokenTransfer" => self.insert_token_transfers(&mut tx, &entries).await?,
                "DCTokenApproval" => self.insert_token_approvals(&mut tx, &entries).await?,
                "UserRegister" => self.insert_user_registrations(&mut tx, &entries).await?,
                "ContractSignReq" => {
                    self.insert_contract_sign_requests(&mut tx, &entries)
                        .await?
                }
                "ContractSignReply" => self.insert_contract_sign_replies(&mut tx, &entries).await?,
                "RepChange" => self.insert_reputation_changes(&mut tx, &entries).await?,
                "RepAge" => self.insert_reputation_aging(&mut tx, &entries).await?,
                "RewardDistr" => self.insert_reward_distributions(&mut tx, &entries).await?,
                "LinkedIcIds" => self.insert_linked_ic_ids(&mut tx, &entries).await?,
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
    async fn insert_provider_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // For now, store raw data since registration is just signature
            sqlx::query(
                "INSERT OR REPLACE INTO provider_registrations (pubkey_hash, pubkey_bytes, signature, created_at_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&entry.key)
            .bind(&entry.value) // Store signature directly
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider check-ins
    async fn insert_provider_check_ins(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let check_in = CheckInPayload::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse check-in: {}", e))?;

            sqlx::query(
                "INSERT INTO provider_check_ins (pubkey_hash, memo, nonce_signature, block_timestamp_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(check_in.memo())
            .bind(check_in.nonce_signature())
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider profiles
    async fn insert_provider_profiles(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let profile_payload = UpdateProfilePayload::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse profile payload: {}", e))?;
            let profile = profile_payload
                .deserialize_update_profile()
                .map_err(|e| anyhow::anyhow!("Failed to deserialize profile: {}", e))?;

            // Extract structured fields from profile
            // Extract structured fields from profile based on ProfileV0_1_0 structure
            let name = "".to_string(); // TODO: Extract from profile structure
            let description = "".to_string(); // TODO: Extract from profile structure
            let website_url = "".to_string(); // TODO: Extract from profile structure
            let contact_email = "".to_string(); // TODO: Extract from profile structure
            let location = "".to_string(); // TODO: Extract from profile structure

            // Store capabilities as JSON since it's complex nested data
            let capabilities_json = serde_json::to_string(&profile)
                .map_err(|e| anyhow::anyhow!("Failed to serialize profile: {}", e))?;

            sqlx::query(
                "INSERT OR REPLACE INTO provider_profiles (pubkey_hash, name, description, website_url, contact_email, location, capabilities_json, updated_at_ns) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(name)
            .bind(description)
            .bind(website_url)
            .bind(contact_email)
            .bind(location)
            .bind(capabilities_json)
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Provider offerings
    async fn insert_provider_offerings(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let offering_payload = offerings::UpdateOfferingsPayload::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse offering payload: {}", e))?;
            let provider_key = &entry.key;
            let offering = offering_payload
                .deserialize_offerings(provider_key)
                .map_err(|e| anyhow::anyhow!("Failed to deserialize offering: {}", e))?;

            // Store each offering as a structured record
            for offering in &offering.server_offerings {
                let price_per_hour_e9s =
                    (offering.monthly_price / 30.0 / 24.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
                let price_per_day_e9s =
                    (offering.monthly_price / 30.0 * DC_TOKEN_DECIMALS_DIV as f64) as i64;
                let tags = offering.features.join(",");
                let availability_json = serde_json::to_string(&offering.payment_methods)
                    .map_err(|e| anyhow::anyhow!("Failed to serialize payment methods: {}", e))?;

                sqlx::query(
                        "INSERT INTO provider_offerings (pubkey_hash, offering_id, instance_type, region, pricing_model, price_per_hour_e9s, price_per_day_e9s, min_contract_hours, max_contract_hours, availability_json, tags, description, created_at_ns) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                    )
                    .bind(&entry.key)
                    .bind(&offering.unique_internal_identifier)
                    .bind(offering.product_type.to_string())
                    .bind(&offering.datacenter_country)
                    .bind("monthly") // pricing model
                    .bind(price_per_hour_e9s)
                    .bind(price_per_day_e9s)
                    .bind(Some(1)) // min contract hours
                    .bind(None::<i64>) // max contract hours
                    .bind(availability_json)
                    .bind(tags)
                    .bind(&offering.description)
                    .bind(entry.block_timestamp_ns as i64)
                    .execute(&mut **tx)
                    .await?;
            }
        }
        Ok(())
    }

    // Token transfers
    async fn insert_token_transfers(
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
    async fn insert_token_approvals(
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

    // User registrations
    async fn insert_user_registrations(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // For now, store raw data since registration is just signature
            sqlx::query(
                "INSERT OR REPLACE INTO user_registrations (pubkey_hash, pubkey_bytes, signature, created_at_ns) VALUES (?, ?, ?, ?)"
            )
            .bind(&entry.key)
            .bind(&entry.key)
            .bind(&entry.value) // Store signature directly
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Contract sign requests
    async fn insert_contract_sign_requests(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let request = ContractSignRequest::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse contract sign request: {}", e))?;

            // Extract structured fields from the request
            let contract_id = entry.key.clone(); // Use the ledger entry key as contract ID
            let requester_pubkey_hash = request.requester_pubkey_bytes().to_vec();
            let requester_ssh_pubkey = request.requester_ssh_pubkey().clone();
            let requester_contact = request.requester_contact().clone();
            let provider_pubkey_hash = request.provider_pubkey_bytes().to_vec();
            let offering_id = request.offering_id().clone();
            let region_name = request.contract_id().cloned(); // Note: field name might be confusing in original struct
            let instance_config = request.instance_config().cloned();
            let payment_amount_e9s = request.payment_amount_e9s() as i64;
            let start_timestamp = request.contract_start_timestamp();
            let request_memo = request.request_memo().clone();

            // Insert the main contract request
            sqlx::query(
                "INSERT INTO contract_sign_requests (contract_id, requester_pubkey_hash, requester_ssh_pubkey, requester_contact, provider_pubkey_hash, offering_id, region_name, instance_config, payment_amount_e9s, start_timestamp, request_memo, created_at_ns) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&contract_id)
            .bind(&requester_pubkey_hash)
            .bind(&requester_ssh_pubkey)
            .bind(&requester_contact)
            .bind(&provider_pubkey_hash)
            .bind(&offering_id)
            .bind(region_name.as_deref())
            .bind(instance_config.as_deref())
            .bind(payment_amount_e9s)
            .bind(start_timestamp.map(|t| t as i64))
            .bind(&request_memo)
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;

            // Insert payment entries if available (need to extract from the payment_entries field)
            // Note: The ContractSignRequest structure has payment_entries, but we need to parse them
            // This would require access to the payment_entries field in the struct
        }
        Ok(())
    }

    // Contract sign replies
    async fn insert_contract_sign_replies(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let reply = ContractSignReply::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse contract sign reply: {}", e))?;

            // Use the entry key as contract ID since it's how contracts are identified
            let contract_id = entry.key.clone();
            let provider_pubkey_hash = entry.key.clone(); // Provider who signed the reply

            // Extract reply status and memo from the reply structure
            // Note: We need to access the actual fields of ContractSignReply
            // For now, using placeholder values - this needs to be adjusted based on the actual struct
            let reply_status = "accepted"; // Default, should be extracted from reply
            let reply_memo = ""; // Default, should be extracted from reply
            let instance_details = serde_json::to_string(&reply)
                .map_err(|e| anyhow::anyhow!("Failed to serialize reply details: {}", e))?;

            sqlx::query(
                "INSERT INTO contract_sign_replies (contract_id, provider_pubkey_hash, reply_status, reply_memo, instance_details, created_at_ns) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&contract_id)
            .bind(&provider_pubkey_hash)
            .bind(reply_status)
            .bind(reply_memo)
            .bind(&instance_details)
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reputation changes
    async fn insert_reputation_changes(
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
            .bind(change.changes()[0].1 as i64) // Get the delta amount from first change
            .bind("") // Reason is not stored in the structure, use empty string
            .bind(entry.block_timestamp_ns as i64) // Use actual block timestamp
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reputation aging
    async fn insert_reputation_aging(
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

    // Reward distributions
    async fn insert_reward_distributions(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            // Reward distributions are stored as a simple timestamp entry
            sqlx::query(
                "INSERT INTO reward_distributions (block_timestamp_ns, total_amount_e9s, providers_count, amount_per_provider_e9s) VALUES (?, ?, ?, ?)"
            )
            .bind(entry.block_timestamp_ns as i64)
            .bind(0) // These would need to be parsed from actual reward data
            .bind(0)
            .bind(0)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Linked IC identities
    async fn insert_linked_ic_ids(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let linked_ids = LinkedIcIdsRecord::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse linked IC IDs: {}", e))?;

            // For each added principal in the linked identities record
            for principal in linked_ids.alt_principals_add() {
                sqlx::query(
                    "INSERT OR REPLACE INTO linked_ic_ids (pubkey_hash, ic_principal, linked_at_ns) VALUES (?, ?, ?)"
                )
                .bind(&entry.key)
                .bind(principal.to_text())
                .bind(entry.block_timestamp_ns as i64)
                .execute(&mut **tx)
                .await?;
            }
        }
        Ok(())
    }

    /// Test helper method to access the underlying pool
    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
