use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{ContractSignReplyPayload, ContractSignRequestPayload};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct Contract {
    #[serde(skip_deserializing)]
    #[oai(skip)]
    pub contract_id: Vec<u8>,
    #[serde(skip_deserializing)]
    #[oai(skip)]
    pub requester_pubkey: Vec<u8>,
    pub requester_ssh_pubkey: String,
    pub requester_contact: String,
    #[serde(skip_deserializing)]
    #[oai(skip)]
    pub provider_pubkey: Vec<u8>,
    pub offering_id: String,
    #[oai(skip_serializing_if_is_none)]
    pub region_name: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub instance_config: Option<String>,
    #[ts(type = "number")]
    pub payment_amount_e9s: i64,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub start_timestamp_ns: Option<i64>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub end_timestamp_ns: Option<i64>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub duration_hours: Option<i64>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub original_duration_hours: Option<i64>,
    pub request_memo: String,
    #[ts(type = "number")]
    pub created_at_ns: i64,
    pub status: String,
    #[oai(skip_serializing_if_is_none)]
    pub provisioning_instance_details: Option<String>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub provisioning_completed_at_ns: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct ContractReply {
    pub contract_id: Vec<u8>,
    pub provider_pubkey: Vec<u8>,
    pub reply_status: String,
    pub reply_memo: Option<String>,
    pub instance_details: Option<String>,
    pub created_at_ns: i64,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
#[allow(dead_code)]
pub struct PaymentEntry {
    pub pricing_model: String,
    pub time_period_unit: String,
    pub quantity: i64,
    pub amount_e9s: i64,
}

#[derive(Debug, Deserialize, Object)]
#[oai(skip_serializing_if_is_none)]
pub struct RentalRequestParams {
    pub offering_db_id: i64,
    #[oai(skip_serializing_if_is_none)]
    pub ssh_pubkey: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub contact_method: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub request_memo: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub duration_hours: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
#[oai(skip_serializing_if_is_none)]
pub struct ContractExtension {
    pub id: i64,
    #[oai(skip)]
    pub contract_id: Vec<u8>,
    #[oai(skip)]
    pub extended_by_pubkey: Vec<u8>,
    pub extension_hours: i64,
    pub extension_payment_e9s: i64,
    pub previous_end_timestamp_ns: i64,
    pub new_end_timestamp_ns: i64,
    #[oai(skip_serializing_if_is_none)]
    pub extension_memo: Option<String>,
    pub created_at_ns: i64,
}

impl Database {
    /// Get contracts for a user (as requester)
    pub async fn get_user_contracts(&self, pubkey: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests WHERE requester_pubkey = ? ORDER BY created_at_ns DESC"
        )
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get contracts for a provider
    pub async fn get_provider_contracts(&self, pubkey: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests WHERE provider_pubkey = ? ORDER BY created_at_ns DESC"
        )
        .bind(pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get pending contracts for a provider
    pub async fn get_pending_provider_contracts(&self, pubkey: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as::<_, Contract>(
            "SELECT * FROM contract_sign_requests WHERE provider_pubkey = ? AND status IN ('requested', 'pending') ORDER BY created_at_ns DESC"
        )
        .bind(pubkey)
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

        // Calculate duration and timestamps
        let duration_hours = params.duration_hours.unwrap_or(720); // Default: 30 days
        let start_timestamp_ns = created_at_ns;
        let end_timestamp_ns = start_timestamp_ns + (duration_hours * 3600 * 1_000_000_000);

        // Calculate payment based on duration (monthly_price is per ~720 hours)
        let payment_amount_e9s =
            ((offering.monthly_price * duration_hours as f64 / 720.0) * 1_000_000_000.0) as i64;

        // Generate deterministic contract ID from SHA256 hash of request data
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(requester_pubkey);
        hasher.update(&offering.pubkey);
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
                contract_id, requester_pubkey, requester_ssh_pubkey,
                requester_contact, provider_pubkey, offering_id,
                payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
                duration_hours, original_duration_hours, request_memo,
                created_at_ns, status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&contract_id)
        .bind(requester_pubkey)
        .bind(&ssh_pubkey)
        .bind(&contact)
        .bind(&offering.pubkey)
        .bind(&offering.offering_id)
        .bind(payment_amount_e9s)
        .bind(start_timestamp_ns)
        .bind(end_timestamp_ns)
        .bind(duration_hours)
        .bind(duration_hours) // original_duration_hours
        .bind(&memo)
        .bind(created_at_ns)
        .bind("requested")
        .execute(&self.pool)
        .await?;

        Ok(contract_id)
    }

    /// Update contract status with authorization check
    pub async fn update_contract_status(
        &self,
        contract_id: &[u8],
        new_status: &str,
        updated_by_pubkey: &[u8],
        change_memo: Option<&str>,
    ) -> Result<()> {
        // Get contract to verify authorization
        let contract = self
            .get_contract(contract_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        // Only provider can update status
        if contract.provider_pubkey != updated_by_pubkey {
            return Err(anyhow::anyhow!(
                "Unauthorized: only provider can update contract status"
            ));
        }

        // Update status and history atomically
        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ? WHERE contract_id = ?",
        )
        .bind(new_status)
        .bind(updated_at_ns)
        .bind(updated_by_pubkey)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(contract_id)
            .bind(&contract.status)
            .bind(new_status)
            .bind(updated_by_pubkey)
            .bind(updated_at_ns)
            .bind(change_memo)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    /// Add provisioning details to a contract
    pub async fn add_provisioning_details(
        &self,
        contract_id: &[u8],
        instance_details: &str,
    ) -> Result<()> {
        let provisioned_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE contract_sign_requests SET provisioning_instance_details = ?, provisioning_completed_at_ns = ? WHERE contract_id = ?",
        )
        .bind(instance_details)
        .bind(provisioned_at_ns)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query("INSERT INTO contract_provisioning_details (contract_id, instance_ip, instance_credentials, connection_instructions, provisioned_at_ns) VALUES (?, ?, ?, ?, ?) ON CONFLICT(contract_id) DO UPDATE SET instance_ip = excluded.instance_ip, instance_credentials = excluded.instance_credentials, connection_instructions = excluded.connection_instructions, provisioned_at_ns = excluded.provisioned_at_ns")
            .bind(contract_id)
            .bind(Option::<&str>::None)
            .bind(Option::<&str>::None)
            .bind(instance_details)
            .bind(provisioned_at_ns)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }

    /// Extend contract duration
    pub async fn extend_contract(
        &self,
        contract_id: &[u8],
        extended_by_pubkey: &[u8],
        extension_hours: i64,
        extension_memo: Option<String>,
    ) -> Result<i64> {
        // Get contract to verify it exists and is extendable
        let contract = self
            .get_contract(contract_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        // Verify authorization: only requester or provider can extend
        if contract.requester_pubkey != extended_by_pubkey
            && contract.provider_pubkey != extended_by_pubkey
        {
            return Err(anyhow::anyhow!(
                "Unauthorized: only requester or provider can extend contract"
            ));
        }

        // Verify contract is in extendable status (active or provisioned)
        if contract.status != "active" && contract.status != "provisioned" {
            return Err(anyhow::anyhow!(
                "Contract cannot be extended in '{}' status",
                contract.status
            ));
        }

        // Get current end timestamp
        let previous_end_timestamp_ns = contract
            .end_timestamp_ns
            .ok_or_else(|| anyhow::anyhow!("Contract has no end timestamp"))?;

        // Calculate new end timestamp
        let new_end_timestamp_ns =
            previous_end_timestamp_ns + (extension_hours * 3600 * 1_000_000_000);

        // Get offering to calculate extension payment
        let offering = self
            .get_offering_by_id(&contract.offering_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Offering not found"))?;

        let extension_payment_e9s =
            ((offering.monthly_price * extension_hours as f64 / 720.0) * 1_000_000_000.0) as i64;

        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // Update contract end timestamp and duration
        let new_duration_hours = contract.duration_hours.unwrap_or(0) + extension_hours;
        sqlx::query(
            "UPDATE contract_sign_requests SET end_timestamp_ns = ?, duration_hours = ? WHERE contract_id = ?",
        )
        .bind(new_end_timestamp_ns)
        .bind(new_duration_hours)
        .bind(contract_id)
        .execute(&self.pool)
        .await?;

        // Record extension in history
        sqlx::query(
            "INSERT INTO contract_extensions (contract_id, extended_by_pubkey, extension_hours, extension_payment_e9s, previous_end_timestamp_ns, new_end_timestamp_ns, extension_memo, created_at_ns) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(contract_id)
        .bind(extended_by_pubkey)
        .bind(extension_hours)
        .bind(extension_payment_e9s)
        .bind(previous_end_timestamp_ns)
        .bind(new_end_timestamp_ns)
        .bind(extension_memo)
        .bind(created_at_ns)
        .execute(&self.pool)
        .await?;

        Ok(extension_payment_e9s)
    }

    /// Get extension history for a contract
    pub async fn get_contract_extensions(
        &self,
        contract_id: &[u8],
    ) -> Result<Vec<ContractExtension>> {
        let extensions = sqlx::query_as::<_, ContractExtension>(
            "SELECT * FROM contract_extensions WHERE contract_id = ? ORDER BY created_at_ns DESC",
        )
        .bind(contract_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(extensions)
    }

    /// Get offering by offering_id string
    async fn get_offering_by_id(
        &self,
        offering_id: &str,
    ) -> Result<Option<crate::database::offerings::Offering>> {
        let offering = sqlx::query_as::<_, crate::database::offerings::Offering>(
            "SELECT * FROM provider_offerings WHERE offering_id = ?",
        )
        .bind(offering_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(offering)
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
            let requester_pubkey = request.requester_pubkey_bytes().to_vec();
            let requester_ssh_pubkey = request.requester_ssh_pubkey().clone();
            let requester_contact = request.requester_contact().clone();
            let provider_pubkey = request.provider_pubkey_bytes().to_vec();
            let offering_id = request.offering_id().clone();
            let region_name = request.region_name().cloned();
            let instance_config = request.instance_config().cloned();
            let payment_amount_e9s = request.payment_amount_e9s() as i64;
            let start_timestamp_ns = request.contract_start_timestamp().map(|t| t as i64);
            let request_memo = request.request_memo().clone();

            // Insert the main contract request
            sqlx::query(
                "INSERT OR REPLACE INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, region_name, instance_config, payment_amount_e9s, start_timestamp_ns, request_memo, created_at_ns, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&contract_id)
            .bind(&requester_pubkey)
            .bind(&requester_ssh_pubkey)
            .bind(&requester_contact)
            .bind(&provider_pubkey)
            .bind(&offering_id)
            .bind(region_name.as_deref())
            .bind(instance_config.as_deref())
            .bind(payment_amount_e9s)
            .bind(start_timestamp_ns)
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
            let provider_pubkey = entry.key.clone(); // Provider who signed the reply (from entry key)

            // Extract reply status and memo from the reply structure
            let reply_status = if reply.sign_accepted() {
                "accepted"
            } else {
                "rejected"
            };
            let reply_memo = reply.response_text();
            let instance_details = reply.response_details();

            sqlx::query(
                "INSERT INTO contract_sign_replies (contract_id, provider_pubkey, reply_status, reply_memo, instance_details, created_at_ns) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&contract_id)
            .bind(&provider_pubkey)
            .bind(reply_status)
            .bind(reply_memo)
            .bind(instance_details)
            .bind(entry.block_timestamp_ns as i64)
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    /// Check if a contract status is cancellable
    fn is_cancellable_status(status: &str) -> bool {
        matches!(
            status,
            "requested" | "pending" | "accepted" | "provisioning"
        )
    }

    /// Cancel a rental request (only by the original requester)
    ///
    /// Cancellable statuses:
    /// - requested: Initial request, not yet seen by provider
    /// - pending: Provider has seen but not responded
    /// - accepted: Provider accepted but hasn't started provisioning
    /// - provisioning: Provider is setting up the instance
    ///
    /// Non-cancellable statuses:
    /// - provisioned/active: Already deployed, requires termination instead
    /// - rejected/cancelled: Already in terminal state
    pub async fn cancel_contract(
        &self,
        contract_id: &[u8],
        cancelled_by_pubkey: &[u8],
        cancel_memo: Option<&str>,
    ) -> Result<()> {
        // Get contract to verify it exists and check authorization
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow::anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;

        // Verify authorization: only requester can cancel their own request
        if contract.requester_pubkey != cancelled_by_pubkey {
            return Err(anyhow::anyhow!(
                "Unauthorized: only the requester can cancel their rental request"
            ));
        }

        // Verify contract is in a cancellable status
        if !Self::is_cancellable_status(&contract.status) {
            return Err(anyhow::anyhow!(
                "Contract cannot be cancelled in '{}' status. Only requested, pending, accepted, or provisioning contracts can be cancelled.",
                contract.status
            ));
        }

        // Update status and history atomically
        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut tx = self.pool.begin().await?;

        // Update contract status to cancelled
        sqlx::query(
            "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ? WHERE contract_id = ?",
        )
        .bind("cancelled")
        .bind(updated_at_ns)
        .bind(cancelled_by_pubkey)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        // Record status change in history
        sqlx::query(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(contract_id)
        .bind(&contract.status)
        .bind("cancelled")
        .bind(cancelled_by_pubkey)
        .bind(updated_at_ns)
        .bind(cancel_memo)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
