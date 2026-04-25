use super::*;
use crate::database::types::Database;
use anyhow::Result;
use dcc_common::ContractStatus;

impl Database {
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
        let extended_by_hex = hex::encode(extended_by_pubkey);
        if contract.requester_pubkey != extended_by_hex
            && contract.provider_pubkey != extended_by_hex
        {
            return Err(anyhow::anyhow!(
                "Unauthorized: only requester or provider can extend contract"
            ));
        }

        // Verify contract is in extendable status (active or provisioned)
        let status: ContractStatus = contract.status.parse().map_err(|e| {
            anyhow::anyhow!("Contract has invalid status '{}': {}", contract.status, e)
        })?;
        if !status.is_operational() {
            return Err(anyhow::anyhow!(
                "Contract cannot be extended in '{}' status (must be provisioned or active)",
                contract.status
            ));
        }

        if extension_hours < 1 {
            return Err(anyhow::anyhow!(
                "extension_hours must be at least 1 (got {})",
                extension_hours
            ));
        }

        // Get current end timestamp
        let previous_end_timestamp_ns = contract
            .end_timestamp_ns
            .ok_or_else(|| anyhow::anyhow!("Contract has no end timestamp"))?;

        // Get offering to calculate extension payment and check max duration
        let offering = self
            .get_offering_by_id(&contract.offering_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Offering not found"))?;

        let new_duration_hours = contract.duration_hours.unwrap_or(0) + extension_hours;
        if let Some(max_hours) = offering.max_contract_hours {
            if new_duration_hours > max_hours {
                return Err(anyhow::anyhow!(
                    "Extension would exceed maximum {} hours (total would be {})",
                    max_hours,
                    new_duration_hours
                ));
            }
        }

        // Calculate new end timestamp
        let new_end_timestamp_ns =
            previous_end_timestamp_ns + (extension_hours * 3600 * 1_000_000_000);

        // Use integer arithmetic to avoid floating-point precision issues
        let monthly_price_e9s = (offering.monthly_price * 1_000_000_000.0) as i128;
        let extension_payment_e9s = (monthly_price_e9s * extension_hours as i128 / 720) as i64;

        let created_at_ns = crate::now_ns()?;

        // Update contract end timestamp and duration
        sqlx::query!(
            "UPDATE contract_sign_requests SET end_timestamp_ns = $1, duration_hours = $2 WHERE contract_id = $3",
            new_end_timestamp_ns,
            new_duration_hours,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        // Record extension in history
        sqlx::query!(
            "INSERT INTO contract_extensions (contract_id, extended_by_pubkey, extension_hours, extension_payment_e9s, previous_end_timestamp_ns, new_end_timestamp_ns, extension_memo, created_at_ns) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            contract_id,
            extended_by_pubkey,
            extension_hours,
            extension_payment_e9s,
            previous_end_timestamp_ns,
            new_end_timestamp_ns,
            extension_memo,
            created_at_ns
        )
        .execute(&self.pool)
        .await?;

        let actor = if hex::encode(extended_by_pubkey) == contract.requester_pubkey {
            "tenant"
        } else {
            "provider"
        };
        let details = format!("Extended by {} hours", extension_hours);
        self.insert_contract_event(contract_id, "extension", None, None, actor, Some(&details))
            .await?;

        Ok(extension_payment_e9s)
    }

    /// Get extension history for a contract
    pub async fn get_contract_extensions(
        &self,
        contract_id: &[u8],
    ) -> Result<Vec<ContractExtension>> {
        let extensions = sqlx::query_as!(
            ContractExtension,
            r#"SELECT id as "id!", contract_id, extended_by_pubkey, extension_hours as "extension_hours!",
               extension_payment_e9s as "extension_payment_e9s!", previous_end_timestamp_ns as "previous_end_timestamp_ns!",
               new_end_timestamp_ns as "new_end_timestamp_ns!", extension_memo, created_at_ns as "created_at_ns!"
               FROM contract_extensions WHERE contract_id = $1 ORDER BY created_at_ns DESC"#,
            contract_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(extensions)
    }

    /// Return active contracts where auto_renew is true and expiry is within 48 hours.
    ///
    /// Only contracts with a confirmed payment (succeeded or self_rental) are returned.
    pub async fn get_contracts_for_renewal(&self) -> Result<Vec<Contract>> {
        // 48 hours in nanoseconds
        let window_ns: i64 = 48 * 3600 * 1_000_000_000;
        let now_ns = crate::now_ns()?;
        let deadline_ns = now_ns + window_ns;

        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(encode(c.contract_id, 'hex')) as "contract_id!: String", lower(encode(c.requester_pubkey, 'hex')) as "requester_pubkey!: String", c.requester_ssh_pubkey as "requester_ssh_pubkey!", c.requester_contact as "requester_contact!", lower(encode(c.provider_pubkey, 'hex')) as "provider_pubkey!: String",
               c.offering_id as "offering_id!", NULL::TEXT as offering_name, c.region_name, c.instance_config, c.payment_amount_e9s, c.start_timestamp_ns, c.end_timestamp_ns,
               c.duration_hours, c.original_duration_hours, c.request_memo as "request_memo!", c.created_at_ns, c.status as "status!",
               c.provisioning_instance_details, c.provisioning_completed_at_ns, c.payment_method as "payment_method!", c.stripe_checkout_session_id, c.stripe_payment_intent_id, c.stripe_customer_id, c.icpay_transaction_id, c.payment_status as "payment_status!",
               c.currency as "currency!", c.refund_amount_e9s, c.stripe_refund_id, c.refund_created_at_ns, c.status_updated_at_ns, c.icpay_payment_id, c.icpay_refund_id, c.total_released_e9s, c.last_release_at_ns,
               c.tax_amount_e9s, c.tax_rate_percent, c.tax_type, c.tax_jurisdiction, c.customer_tax_id, c.reverse_charge, c.buyer_address, c.stripe_invoice_id, c.receipt_number, c.receipt_sent_at_ns,
               c.stripe_subscription_id, c.subscription_status, c.current_period_end_ns, COALESCE(c.cancel_at_period_end, FALSE) as "cancel_at_period_end!: bool",
               COALESCE(c.auto_renew, FALSE) as "auto_renew!: bool",
               c.gateway_slug, c.gateway_subdomain, c.gateway_ssh_port, c.gateway_port_range_start, c.gateway_port_range_end,
                pd.password_reset_requested_at_ns, pd.ssh_key_rotation_requested_at_ns, c.operating_system
                FROM contract_sign_requests c
                LEFT JOIN contract_provisioning_details pd ON pd.contract_id = c.contract_id
                WHERE c.auto_renew = TRUE
                 AND c.status = 'active'
                 AND c.end_timestamp_ns IS NOT NULL
                 AND c.end_timestamp_ns <= $1
                 AND (c.payment_status = 'succeeded' OR c.payment_method = 'self_rental')"#,
            deadline_ns
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Set auto_renew flag on a contract.
    ///
    /// Only the original requester may change this setting.
    pub async fn set_contract_auto_renew(
        &self,
        contract_id: &[u8],
        requester_pubkey: &[u8],
        auto_renew: bool,
    ) -> Result<()> {
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow::anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;

        if contract.requester_pubkey != hex::encode(requester_pubkey) {
            return Err(anyhow::anyhow!(
                "Unauthorized: only the contract requester can change auto-renew"
            ));
        }

        let result = sqlx::query!(
            "UPDATE contract_sign_requests SET auto_renew = $1 WHERE contract_id = $2",
            auto_renew,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Contract not found (ID: {})",
                hex::encode(contract_id)
            ));
        }

        Ok(())
    }

    /// Insert a contract event into the timeline.
    pub async fn insert_contract_event(
        &self,
        contract_id: &[u8],
        event_type: &str,
        old_status: Option<&str>,
        new_status: Option<&str>,
        actor: &str,
        details: Option<&str>,
    ) -> Result<i64> {
        let created_at = crate::now_ns()?;
        let id: i64 = sqlx::query_scalar(
            r#"INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id"#,
        )
        .bind(contract_id)
        .bind(event_type)
        .bind(old_status)
        .bind(new_status)
        .bind(actor)
        .bind(details)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }

    /// Get all events for a contract ordered chronologically.
    pub async fn get_contract_events(&self, contract_id: &[u8]) -> Result<Vec<ContractEvent>> {
        let events = sqlx::query_as!(
            ContractEvent,
            r#"SELECT id as "id!", lower(encode(contract_id, 'hex')) as "contract_id!: String",
               event_type as "event_type!", old_status, new_status,
               actor as "actor!", details, created_at as "created_at!"
               FROM contract_events
               WHERE contract_id = $1
               ORDER BY created_at ASC"#,
            contract_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }

    /// Get recent SSH key rotation events across all contracts for a user,
    /// created after the given timestamp (nanoseconds since epoch).
    /// Returns events ordered by created_at ASC.
    pub async fn get_ssh_key_rotation_events_for_user(
        &self,
        requester_pubkey: &[u8],
        after_ns: i64,
    ) -> Result<Vec<ContractEvent>> {
        let events = sqlx::query_as!(
            ContractEvent,
            r#"SELECT ce.id as "id!", lower(encode(ce.contract_id, 'hex')) as "contract_id!: String",
                ce.event_type as "event_type!", ce.old_status, ce.new_status,
                ce.actor as "actor!", ce.details, ce.created_at as "created_at!"
                FROM contract_events ce
                JOIN contract_sign_requests csr ON csr.contract_id = ce.contract_id
                WHERE csr.requester_pubkey = $1
                  AND ce.event_type IN ('ssh_key_rotation', 'ssh_key_rotation_complete')
                  AND ce.created_at > $2
                ORDER BY ce.created_at ASC"#,
            requester_pubkey,
            after_ns
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(events)
    }
}
