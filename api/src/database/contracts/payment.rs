use super::*;
use crate::database::types::Database;
use anyhow::Result;

impl Database {
    /// Update contract with checkout session payment details (includes tax info)
    pub async fn update_checkout_session_payment(
        &self,
        contract_id: &[u8],
        checkout_session_id: &str,
        tax_amount_e9s: Option<i64>,
        customer_tax_id: Option<&str>,
        reverse_charge: bool,
        stripe_invoice_id: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE contract_sign_requests SET stripe_payment_intent_id = $1, payment_status = $2, tax_amount_e9s = $3, customer_tax_id = $4, reverse_charge = $5, stripe_invoice_id = $6 WHERE contract_id = $7"
        )
        .bind(checkout_session_id)
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

    /// Calculate prorated refund amount based on time used
    ///
    /// Formula: refund = (unused_time / total_time) * payment_amount
    /// Only returns a refund for contracts that haven't started or are in early stages
    ///
    /// # Arguments
    /// * `payment_amount_e9s` - Original payment amount in e9s
    /// * `service_start_ns` - When user actually got access (provisioning_completed_at_ns)
    /// * `end_timestamp_ns` - Contract end time in nanoseconds
    /// * `current_timestamp_ns` - Current time in nanoseconds
    ///
    /// # Returns
    /// Refund amount in e9s. Full refund if service never started.
    pub(super) fn calculate_prorated_refund(
        payment_amount_e9s: i64,
        service_start_ns: Option<i64>,
        end_timestamp_ns: Option<i64>,
        current_timestamp_ns: i64,
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

        // Time user actually used the service
        let time_used_ns = current_timestamp_ns.saturating_sub(service_start);

        // If current time is before service started, full refund
        if time_used_ns <= 0 {
            return payment_amount_e9s;
        }

        // Time remaining
        let time_remaining_ns = end.saturating_sub(current_timestamp_ns);

        // If contract already expired, no refund
        if time_remaining_ns <= 0 {
            return 0;
        }

        // Calculate prorated refund using integer arithmetic (avoid float precision loss)
        let refund_amount = ((payment_amount_e9s as i128) * (time_remaining_ns as i128)
            / (total_duration_ns as i128)) as i64;

        // Ensure non-negative
        refund_amount.max(0)
    }

    /// Update ICPay transaction ID for a contract
    pub async fn update_icpay_transaction_id(
        &self,
        contract_id: &[u8],
        transaction_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE contract_sign_requests SET icpay_transaction_id = $1 WHERE contract_id = $2",
            transaction_id,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update ICPay payment confirmation (webhook callback)
    /// Sets icpay_payment_id and payment_status = 'succeeded'
    pub async fn update_icpay_payment_confirmed(
        &self,
        contract_id: &[u8],
        payment_id: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE contract_sign_requests SET icpay_payment_id = $1, payment_status = $2 WHERE contract_id = $3",
            payment_id,
            "succeeded",
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update ICPay payment status
    pub async fn update_icpay_payment_status(
        &self,
        contract_id: &[u8],
        new_status: &str,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE contract_sign_requests SET payment_status = $1 WHERE contract_id = $2",
            new_status,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Process ICPay refund for a contract cancellation
    ///
    /// # Arguments
    /// * `contract` - The contract to refund
    /// * `icpay_client` - Optional ICPay client for API calls
    /// * `current_timestamp_ns` - Current timestamp for prorated calculation
    ///
    /// # Returns
    /// Tuple of (refund_amount_e9s, refund_id)
    pub(super) async fn process_icpay_refund(
        &self,
        contract: &Contract,
        icpay_client: Option<&crate::icpay_client::IcpayClient>,
        current_timestamp_ns: i64,
    ) -> Result<(Option<i64>, Option<String>)> {
        // Get payment ID - prefer icpay_payment_id (webhook-set), fall back to icpay_transaction_id (frontend-set)
        let payment_id = match (&contract.icpay_payment_id, &contract.icpay_transaction_id) {
            (Some(id), _) => id,
            (None, Some(id)) => id,
            (None, None) => return Ok((None, None)),
        };

        // Calculate prorated refund based on when service became active
        // If never provisioned, user gets full refund
        let gross_refund_e9s = Self::calculate_prorated_refund(
            contract.payment_amount_e9s,
            contract.provisioning_completed_at_ns,
            contract.end_timestamp_ns,
            current_timestamp_ns,
        );

        // Subtract any amounts already released to provider
        let already_released = contract.total_released_e9s.unwrap_or(0);
        let net_refund_e9s = gross_refund_e9s.saturating_sub(already_released);

        // Only process refund if amount is positive and icpay_client is provided
        if net_refund_e9s > 0 {
            if let Some(client) = icpay_client {
                // Create refund via ICPay API
                match client.create_refund(payment_id, Some(net_refund_e9s)).await {
                    Ok(refund_id) => {
                        tracing::info!(
                            "ICPay refund created: {} for contract {} (amount: {} e9s)",
                            refund_id,
                            &contract.contract_id,
                            net_refund_e9s
                        );
                        Ok((Some(net_refund_e9s), Some(refund_id)))
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to create ICPay refund for contract {}: {:#}",
                            &contract.contract_id,
                            e
                        );
                        Ok((Some(net_refund_e9s), None))
                    }
                }
            } else {
                // No icpay_client provided, just track the calculated amount
                Ok((Some(net_refund_e9s), None))
            }
        } else {
            Ok((None, None))
        }
    }

    /// Get active ICPay contracts ready for daily release
    pub async fn get_contracts_for_release(&self) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(encode(c.contract_id, 'hex')) as "contract_id!: String", lower(encode(c.requester_pubkey, 'hex')) as "requester_pubkey!: String", c.requester_ssh_pubkey as "requester_ssh_pubkey!", c.requester_contact as "requester_contact!", lower(encode(c.provider_pubkey, 'hex')) as "provider_pubkey!: String",
               c.offering_id as "offering_id!", NULL::TEXT as offering_name, c.region_name, c.instance_config, c.payment_amount_e9s, c.start_timestamp_ns, c.end_timestamp_ns,
               c.duration_hours, c.original_duration_hours, c.request_memo as "request_memo!", c.created_at_ns, c.status as "status!",
               c.provisioning_instance_details, c.provisioning_completed_at_ns, c.payment_method as "payment_method!", c.stripe_payment_intent_id, c.stripe_customer_id, c.icpay_transaction_id, c.payment_status as "payment_status!",
               c.currency as "currency!", c.refund_amount_e9s, c.stripe_refund_id, c.refund_created_at_ns, c.status_updated_at_ns, c.icpay_payment_id, c.icpay_refund_id, c.total_released_e9s, c.last_release_at_ns,
               c.tax_amount_e9s, c.tax_rate_percent, c.tax_type, c.tax_jurisdiction, c.customer_tax_id, c.reverse_charge, c.buyer_address, c.stripe_invoice_id, c.receipt_number, c.receipt_sent_at_ns,
               c.stripe_subscription_id, c.subscription_status, c.current_period_end_ns, COALESCE(c.cancel_at_period_end, FALSE) as "cancel_at_period_end!: bool",
               COALESCE(c.auto_renew, FALSE) as "auto_renew!: bool",
               c.gateway_slug, c.gateway_subdomain, c.gateway_ssh_port, c.gateway_port_range_start, c.gateway_port_range_end,
                pd.password_reset_requested_at_ns, pd.ssh_key_rotation_requested_at_ns, c.operating_system
                FROM contract_sign_requests c
                LEFT JOIN contract_provisioning_details pd ON pd.contract_id = c.contract_id
                WHERE c.payment_method = 'icpay'
               AND c.payment_status = 'succeeded'
               AND c.status IN ('active', 'provisioned')
               ORDER BY c.created_at_ns ASC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Calculate and create a payment release record for a contract
    pub async fn create_payment_release(
        &self,
        contract_id: &[u8],
        release_type: &str,
        period_start_ns: i64,
        period_end_ns: i64,
        amount_e9s: i64,
        provider_pubkey: &[u8],
    ) -> Result<PaymentRelease> {
        let created_at_ns = crate::now_ns()?;
        let status = "pending";

        let id: i64 = sqlx::query_scalar(
            r#"INSERT INTO payment_releases (contract_id, release_type, period_start_ns, period_end_ns, amount_e9s, provider_pubkey, status, created_at_ns)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id"#,
        )
        .bind(contract_id)
        .bind(release_type)
        .bind(period_start_ns)
        .bind(period_end_ns)
        .bind(amount_e9s)
        .bind(provider_pubkey)
        .bind(status)
        .bind(created_at_ns)
        .fetch_one(&self.pool)
        .await?;

        Ok(PaymentRelease {
            id,
            contract_id: contract_id.to_vec(),
            release_type: release_type.to_string(),
            period_start_ns,
            period_end_ns,
            amount_e9s,
            provider_pubkey: provider_pubkey.to_vec(),
            status: status.to_string(),
            created_at_ns,
            released_at_ns: None,
            payout_id: None,
        })
    }

    /// Update contract's release tracking fields
    pub async fn update_contract_release_tracking(
        &self,
        contract_id: &[u8],
        last_release_at_ns: i64,
        total_released_e9s: i64,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE contract_sign_requests SET last_release_at_ns = $1, total_released_e9s = $2 WHERE contract_id = $3",
            last_release_at_ns,
            total_released_e9s,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get pending releases for a provider (ready for payout)
    pub async fn get_provider_pending_releases(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<PaymentRelease>> {
        let releases = sqlx::query_as::<_, PaymentRelease>(
            r#"SELECT id, contract_id, release_type, period_start_ns,
               period_end_ns, amount_e9s, provider_pubkey,
               status, created_at_ns, released_at_ns, payout_id
               FROM payment_releases
               WHERE provider_pubkey = $1 AND status = 'pending'
               ORDER BY created_at_ns ASC"#,
        )
        .bind(provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(releases)
    }

    /// Mark releases as paid out with payout_id
    pub async fn mark_releases_paid_out(&self, release_ids: &[i64], payout_id: &str) -> Result<()> {
        if release_ids.is_empty() {
            return Ok(());
        }

        // Build placeholders for IN clause (starting at $3 since $1 and $2 are used)
        let placeholders: Vec<String> = (3..=release_ids.len() + 2)
            .map(|i| format!("${}", i))
            .collect();
        let query = format!(
            "UPDATE payment_releases SET status = $1, payout_id = $2 WHERE id IN ({})",
            placeholders.join(",")
        );

        let mut query_builder = sqlx::query(&query);
        query_builder = query_builder.bind("paid_out").bind(payout_id);
        for id in release_ids {
            query_builder = query_builder.bind(id);
        }

        query_builder.execute(&self.pool).await?;

        Ok(())
    }

    /// Get all providers with pending releases (for admin overview)
    pub async fn get_providers_with_pending_releases(
        &self,
    ) -> Result<Vec<ProviderPendingReleases>> {
        let results = sqlx::query_as::<_, ProviderPendingReleases>(
            r#"SELECT provider_pubkey, SUM(amount_e9s) as total_pending_e9s, COUNT(*) as release_count
               FROM payment_releases
               WHERE status = 'pending'
               GROUP BY provider_pubkey
               ORDER BY total_pending_e9s DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results)
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
