use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{ContractSignReplyPayload, ContractSignRequestPayload};

impl Database {
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
