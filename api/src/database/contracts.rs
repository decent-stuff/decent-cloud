use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{ContractSignReplyPayload, ContractSignRequestPayload};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Contract {
    pub contract_id: Vec<u8>,
    pub requester_pubkey_hash: Vec<u8>,
    pub requester_ssh_pubkey: String,
    pub requester_contact: String,
    pub provider_pubkey_hash: Vec<u8>,
    pub offering_id: String,
    pub region_name: Option<String>,
    pub instance_config: Option<String>,
    pub payment_amount_e9s: i64,
    pub start_timestamp: Option<i64>,
    pub request_memo: String,
    pub created_at_ns: i64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ContractReply {
    pub contract_id: Vec<u8>,
    pub provider_pubkey_hash: Vec<u8>,
    pub reply_status: String,
    pub reply_memo: Option<String>,
    pub instance_details: Option<String>,
    pub created_at_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentEntry {
    pub pricing_model: String,
    pub time_period_unit: String,
    pub quantity: i64,
    pub amount_e9s: i64,
}

#[derive(Debug, Deserialize)]
pub struct RentalRequestParams {
    pub offering_db_id: i64,
    pub ssh_pubkey: Option<String>,
    pub contact_method: Option<String>,
    pub request_memo: Option<String>,
}

impl Database {
    /// Get contracts for a user (as requester)
    pub async fn get_user_contracts(&self, pubkey_hash: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests WHERE requester_pubkey_hash = ? ORDER BY created_at_ns DESC"
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get contracts for a provider
    pub async fn get_provider_contracts(&self, pubkey_hash: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests WHERE provider_pubkey_hash = ? ORDER BY created_at_ns DESC"
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get pending contracts for a provider
    pub async fn get_pending_provider_contracts(
        &self,
        pubkey_hash: &[u8],
    ) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests WHERE provider_pubkey_hash = ? AND status = 'pending' ORDER BY created_at_ns DESC"
        )
        .bind(pubkey_hash)
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get contract by ID
    pub async fn get_contract(&self, contract_id: &[u8]) -> Result<Option<Contract>> {
        let contract = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests WHERE contract_id = ?",
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(contract)
    }

    /// Get contract reply
    pub async fn get_contract_reply(&self, contract_id: &[u8]) -> Result<Option<ContractReply>> {
        let reply = sqlx::query_as::<_, ContractReply>(
            "SELECT * FROM contract_sign_replies WHERE contract_id = ?",
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(reply)
    }

    /// Get contract payment entries
    pub async fn get_contract_payments(&self, contract_id: &[u8]) -> Result<Vec<PaymentEntry>> {
        let payments = sqlx::query_as::<_, PaymentEntry>(
            "SELECT pricing_model, time_period_unit, quantity, amount_e9s FROM contract_payment_entries WHERE contract_id = ?"
        )
        .bind(contract_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(payments)
    }

    /// Get all contracts with pagination
    pub async fn list_contracts(&self, limit: i64, offset: i64) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests ORDER BY created_at_ns DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Create a rental request for an offering
    pub async fn create_rental_request(
        &self,
        requester_pubkey: &[u8],
        params: RentalRequestParams,
    ) -> Result<Vec<u8>> {
        // Get offering details
        let offering = self
            .get_offering(params.offering_db_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Offering not found"))?;

        // Get user's SSH key and contact if not provided
        let ssh_pubkey = if let Some(key) = params.ssh_pubkey {
            key
        } else {
            // Try to get first SSH key from user's profile
            let keys = self.get_user_public_keys(requester_pubkey).await?;
            keys.iter()
                .find(|k| k.key_type.to_lowercase().contains("ssh"))
                .map(|k| k.key_data.clone())
                .unwrap_or_else(|| "".to_string())
        };

        let contact = if let Some(c) = params.contact_method {
            c
        } else {
            // Try to get first contact from user's profile
            let contacts = self.get_user_contacts(requester_pubkey).await?;
            contacts
                .first()
                .map(|c| format!("{}:{}", c.contact_type, c.contact_value))
                .unwrap_or_else(|| "".to_string())
        };

        let memo = params
            .request_memo
            .unwrap_or_else(|| format!("Rental request for {}", offering.offer_name));

        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let payment_amount_e9s = (offering.monthly_price * 1_000_000_000.0) as i64;

        // Generate deterministic contract ID from SHA256 hash of request data
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(requester_pubkey);
        hasher.update(&offering.pubkey_hash);
        hasher.update(offering.offering_id.as_bytes());
        hasher.update(ssh_pubkey.as_bytes());
        hasher.update(contact.as_bytes());
        hasher.update(payment_amount_e9s.to_le_bytes());
        hasher.update(memo.as_bytes());
        hasher.update(created_at_ns.to_le_bytes());
        let contract_id = hasher.finalize().to_vec();

        // Insert contract request
        sqlx::query(
            "INSERT INTO contract_sign_requests (
                contract_id, requester_pubkey_hash, requester_ssh_pubkey,
                requester_contact, provider_pubkey_hash, offering_id,
                payment_amount_e9s, request_memo, created_at_ns, status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&contract_id)
        .bind(requester_pubkey)
        .bind(&ssh_pubkey)
        .bind(&contact)
        .bind(&offering.pubkey_hash)
        .bind(&offering.offering_id)
        .bind(payment_amount_e9s)
        .bind(&memo)
        .bind(created_at_ns)
        .bind("requested")
        .execute(&self.pool)
        .await?;

        Ok(contract_id)
    }
    // Contract sign requests
    pub async fn insert_contract_sign_requests(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let csr = ContractSignRequestPayload::try_from_slice(&entry.value).map_err(|e| {
                anyhow::anyhow!("Failed to parse contract sign request payload: {}", e)
            })?;
            let request = csr.deserialize_contract_sign_request().map_err(|e| {
                anyhow::anyhow!("Failed to deserialize contract sign request: {}", e)
            })?;

            // Use the calculated contract ID from the payload
            let contract_id = csr.calc_contract_id().to_vec();
            let requester_pubkey_hash = request.requester_pubkey_bytes().to_vec();
            let requester_ssh_pubkey = request.requester_ssh_pubkey().clone();
            let requester_contact = request.requester_contact().clone();
            let provider_pubkey_hash = request.provider_pubkey_bytes().to_vec();
            let offering_id = request.offering_id().clone();
            let region_name = request.region_name().cloned();
            let instance_config = request.instance_config().cloned();
            let payment_amount_e9s = request.payment_amount_e9s() as i64;
            let start_timestamp = request.contract_start_timestamp();
            let request_memo = request.request_memo().clone();

            // Insert the main contract request
            sqlx::query(
                "INSERT OR REPLACE INTO contract_sign_requests (contract_id, requester_pubkey_hash, requester_ssh_pubkey, requester_contact, provider_pubkey_hash, offering_id, region_name, instance_config, payment_amount_e9s, start_timestamp, request_memo, created_at_ns, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
            .bind("pending") // Default status
            .execute(&mut **tx)
            .await?;

            // Insert payment entries from the request
            for payment_entry in request.payment_entries() {
                sqlx::query(
                            "INSERT INTO contract_payment_entries (contract_id, pricing_model, time_period_unit, quantity, amount_e9s) VALUES (?, ?, ?, ?, ?)"
                        )
                        .bind(&contract_id)
                        .bind(&payment_entry.e.pricing_model)
                        .bind(&payment_entry.e.time_period_unit)
                        .bind(payment_entry.e.quantity as i64)
                        .bind(payment_entry.amount_e9s as i64)
                        .execute(&mut **tx)
                        .await?;
            }
        }
        Ok(())
    }

    // Contract sign replies
    pub(crate) async fn insert_contract_sign_replies(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let payload = ContractSignReplyPayload::try_from_slice(&entry.value).map_err(|e| {
                anyhow::anyhow!("Failed to parse contract sign reply payload: {}", e)
            })?;
            let reply = payload
                .deserialize_contract_sign_reply()
                .map_err(|e| anyhow::anyhow!("Failed to deserialize contract sign reply: {}", e))?;

            // Use the contract ID from the reply structure
            let contract_id = reply.contract_id().to_vec();
            let provider_pubkey_hash = entry.key.clone(); // Provider who signed the reply (from entry key)

            // Extract reply status and memo from the reply structure
            let reply_status = if reply.sign_accepted() {
                "accepted"
            } else {
                "rejected"
            };
            let reply_memo = reply.response_text();
            let instance_details = reply.response_details();

            sqlx::query(
                "INSERT INTO contract_sign_replies (contract_id, provider_pubkey_hash, reply_status, reply_memo, instance_details, created_at_ns) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&contract_id)
            .bind(&provider_pubkey_hash)
            .bind(reply_status)
            .bind(reply_memo)
            .bind(instance_details)
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
