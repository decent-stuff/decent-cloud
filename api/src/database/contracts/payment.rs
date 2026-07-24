use super::*;
use crate::database::types::Database;
use anyhow::Result;

impl Database {
    /// Update contract with checkout session payment details (includes tax info)
    ///
    /// `payment_intent_id` is the real PaymentIntent ID (`pi_*`) read from
    /// `session.payment_intent` at checkout completion. It can be `None` when
    /// the session has not yet had a PaymentIntent attached (e.g. async flows).
    pub async fn update_checkout_session_payment(
        &self,
        contract_id: &[u8],
        checkout_session_id: &str,
        payment_intent_id: Option<&str>,
        tax_amount_e9s: Option<i64>,
        customer_tax_id: Option<&str>,
        reverse_charge: bool,
        stripe_invoice_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE contract_sign_requests SET stripe_checkout_session_id = $1, stripe_payment_intent_id = $2, payment_status = $3, tax_amount_e9s = $4, customer_tax_id = $5, reverse_charge = $6, stripe_invoice_id = $7 WHERE contract_id = $8"
        )
        .bind(checkout_session_id)
        .bind(payment_intent_id)
        .bind("succeeded")
        .bind(tax_amount_e9s)
        .bind(customer_tax_id)
        .bind(reverse_charge)
        .bind(stripe_invoice_id)
        .bind(contract_id)
        .execute(&self.pool)
        .await?;

        self.insert_contract_event(
            contract_id,
            "payment_confirmed",
            None,
            None,
            "system",
            Some(&format!("Stripe session: {}", checkout_session_id)),
        )
        .await?;

        Ok(())
    }

    /// Update stripe_invoice_id for a contract (called from invoice.paid webhook)
    pub async fn update_stripe_invoice_id(
        &self,
        contract_id: &[u8],
        stripe_invoice_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE contract_sign_requests SET stripe_invoice_id = $1 WHERE contract_id = $2",
            stripe_invoice_id,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get offering by offering_id string
    pub async fn get_offering_by_id(
        &self,
        offering_id: &str,
    ) -> Result<Option<crate::database::offerings::Offering>> {
        let example_provider_pubkey = hex::encode(Self::example_provider_pubkey());
        let offering = sqlx::query_as::<_, crate::database::offerings::Offering>(
            r#"SELECT id, lower(encode(pubkey, 'hex')) as pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval,
               billing_unit, pricing_model, price_per_unit, included_units, overage_price_per_unit, stripe_metered_price_id,
               is_subscription, subscription_interval_days,
               stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, gpu_count, gpu_memory_gb, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems,
               NULL as trust_score, NULL as has_critical_flags, CASE WHEN lower(encode(pubkey, 'hex')) = $1 THEN TRUE ELSE FALSE END as is_example,
               offering_source, external_checkout_url, NULL as reseller_name, NULL as reseller_commission_percent, NULL as owner_username,
               provisioner_type, provisioner_config, template_name, agent_pool_id, post_provision_script, NULL as provider_online, NULL as resolved_pool_id, NULL as resolved_pool_name
               FROM provider_offerings WHERE offering_id = $2"#
        )
        .bind(example_provider_pubkey)
        .bind(offering_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(offering)
    }

    /// Calculate prorated refund amount based on time used.
    ///
    /// Refund formula:
    ///   billable_time = max(0, current - service_start - total_paused_ns)
    ///   refund        = payment * (total_duration - billable_time) / total_duration
    ///
    /// `total_paused_ns` (from `contract_sign_requests.total_paused_ns`) is the
    /// cumulative time the contract spent in `paused` state. Paused intervals
    /// are not billable -- customers must not be charged for windows where
    /// their VM was stopped pending dispute resolution. Pass `0` when the
    /// contract was never paused.
    ///
    /// # Arguments
    /// * `payment_amount_e9s` - Original payment amount in e9s
    /// * `service_start_ns` - When user actually got access (provisioning_completed_at_ns)
    /// * `end_timestamp_ns` - Contract end time in nanoseconds
    /// * `current_timestamp_ns` - Current time in nanoseconds
    /// * `total_paused_ns` - Cumulative pause duration to credit back (>= 0)
    ///
    /// # Returns
    /// Refund amount in e9s. Full refund if service never started or was
    /// paused for the entire usage window.
    pub(super) fn calculate_prorated_refund(
        payment_amount_e9s: i64,
        service_start_ns: Option<i64>,
        end_timestamp_ns: Option<i64>,
        current_timestamp_ns: i64,
        total_paused_ns: i64,
    ) -> i64 {
        // If service never started (not provisioned), full refund
        let service_start = match service_start_ns {
            Some(s) => s,
            None => return payment_amount_e9s,
        };

        let end = match end_timestamp_ns {
            Some(e) => e,
            None => return 0, // No end time = invalid contract
        };

        // Total service duration (from provisioning to end)
        let total_duration_ns = end - service_start;
        if total_duration_ns <= 0 {
            return 0;
        }

        // Time user actually used the service. Negative paused values are
        // treated as zero (defensive: bad data should never inflate refunds).
        let elapsed_ns = current_timestamp_ns.saturating_sub(service_start);
        let billable_used_ns = elapsed_ns.saturating_sub(total_paused_ns.max(0));

        // If billable window is empty (current time is before service
        // started, OR pauses cover the entire elapsed window), full refund.
        if billable_used_ns <= 0 {
            return payment_amount_e9s;
        }

        // Time remaining = total - billable_used. We do NOT just use
        // (end - now): that would ignore pause credit. Pause credits extend
        // the remaining bucket, capped by total_duration.
        let time_remaining_ns = (total_duration_ns - billable_used_ns).max(0);
        if time_remaining_ns <= 0 {
            return 0;
        }

        // Calculate prorated refund using integer arithmetic (avoid float precision loss)
        let refund_amount = ((payment_amount_e9s as i128) * (time_remaining_ns as i128)
            / (total_duration_ns as i128)) as i64;

        // Ensure non-negative; cap at payment_amount as a defensive bound.
        refund_amount.clamp(0, payment_amount_e9s)
    }

    /// Net refund owed to the customer on cancellation/rejection: the prorated
    /// refund for unused time.
    ///
    /// Under Stripe-only no funds are ever pre-released to the provider, so the
    /// net refund is simply the gross prorated refund. Shared by the cancel,
    /// reject, and dispute-lost paths so all honour the same policy. Returns a
    /// value in `[0, payment_amount_e9s]`.
    pub(super) async fn calculate_net_refund_e9s(
        &self,
        contract: &Contract,
        current_timestamp_ns: i64,
    ) -> Result<i64> {
        let contract_id_bytes = hex::decode(&contract.contract_id)?;
        let total_paused_ns = self.get_total_paused_ns(&contract_id_bytes).await?;
        let gross_refund_e9s = Self::calculate_prorated_refund(
            contract.payment_amount_e9s,
            contract.provisioning_completed_at_ns,
            contract.end_timestamp_ns,
            current_timestamp_ns,
            total_paused_ns,
        );
        Ok(gross_refund_e9s.max(0))
    }

    // ========== Pending Stripe Receipts ==========

    /// Schedule a pending Stripe receipt for delayed processing
    /// First attempt will be after 1 minute
    pub async fn schedule_pending_stripe_receipt(&self, contract_id: &[u8]) -> Result<()> {
        let now_ns = crate::now_ns()?;
        let first_attempt_ns = now_ns + 60_000_000_000; // 1 minute

        sqlx::query!(
            "INSERT INTO pending_stripe_receipts (contract_id, created_at_ns, next_attempt_at_ns, attempts) VALUES ($1, $2, $3, 0) ON CONFLICT (contract_id) DO NOTHING",
            contract_id,
            now_ns,
            first_attempt_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_pending_stripe_receipts(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingStripeReceipt>> {
        let now_ns = crate::now_ns()?;

        let rows = sqlx::query!(
            r#"SELECT contract_id, created_at_ns, next_attempt_at_ns, attempts
               FROM pending_stripe_receipts
               WHERE next_attempt_at_ns <= $1
               ORDER BY next_attempt_at_ns ASC
               LIMIT $2"#,
            now_ns,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| PendingStripeReceipt {
                contract_id: r.contract_id,
                attempts: r.attempts,
            })
            .collect())
    }

    /// Update pending receipt for next retry (1 minute intervals, max 5 attempts)
    pub async fn update_pending_stripe_receipt_retry(&self, contract_id: &[u8]) -> Result<bool> {
        let now_ns = crate::now_ns()?;
        let next_attempt_ns = now_ns + 60_000_000_000; // 1 minute

        // Increment attempts and update next_attempt_at_ns
        // Only if attempts < 5
        let result = sqlx::query!(
            "UPDATE pending_stripe_receipts SET attempts = attempts + 1, next_attempt_at_ns = $1 WHERE contract_id = $2 AND attempts < 5",
            next_attempt_ns,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Remove pending receipt (either sent successfully or max attempts reached)
    pub async fn remove_pending_stripe_receipt(&self, contract_id: &[u8]) -> Result<()> {
        sqlx::query!(
            "DELETE FROM pending_stripe_receipts WHERE contract_id = $1",
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Cancel pending receipt if receipt already sent (e.g., via invoice.paid webhook)
    pub async fn cancel_pending_stripe_receipt_if_sent(&self, contract_id: &[u8]) -> Result<bool> {
        // Check if receipt already sent for this contract
        let contract = self.get_contract(contract_id).await?;
        if let Some(c) = contract {
            if c.receipt_sent_at_ns.is_some() {
                self.remove_pending_stripe_receipt(contract_id).await?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}
