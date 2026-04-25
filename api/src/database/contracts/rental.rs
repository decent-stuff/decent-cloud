use super::*;
use crate::database::types::Database;
use anyhow::Result;
use dcc_common::ContractStatus;

impl Database {
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

        // Validate offering has valid currency (fail-fast principle)
        if offering.currency.is_empty() || offering.currency == "???" {
            return Err(anyhow::anyhow!(
                "Offering {} has invalid currency '{}'. Cannot create contract.",
                offering.offering_id,
                offering.currency
            ));
        }

        // Get user's SSH key and contact if not provided
        let ssh_pubkey = if let Some(key) = params.ssh_pubkey {
            key
        } else {
            // Try to get first SSH key from user's account profile
            match self.get_account_id_by_public_key(requester_pubkey).await? {
                Some(account_id) => {
                    let keys = self.get_account_external_keys(&account_id).await?;
                    keys.iter()
                        .find(|k| k.key_type.to_lowercase().contains("ssh"))
                        .map(|k| k.key_data.clone())
                        .unwrap_or_else(|| "".to_string())
                }
                None => "".to_string(),
            }
        };

        let contact = if let Some(c) = params.contact_method {
            c
        } else {
            // Try to get account email (primary contact method)
            match self.get_account_id_by_public_key(requester_pubkey).await? {
                Some(account_id) => {
                    match self.get_account(&account_id).await? {
                        Some(account) if account.email.is_some() => {
                            format!("email:{}", account.email.unwrap())
                        }
                        _ => {
                            // Fall back to first non-email contact (phone, telegram, etc.)
                            let contacts = self.get_account_contacts(&account_id).await?;
                            contacts
                                .first()
                                .map(|c| format!("{}:{}", c.contact_type, c.contact_value))
                                .unwrap_or_default()
                        }
                    }
                }
                None => "".to_string(),
            }
        };

        let memo = params
            .request_memo
            .unwrap_or_else(|| format!("Rental request for {}", offering.offer_name));

        // Validate payment method (fail-fast if not provided)
        let payment_method_str = params
            .payment_method
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("payment_method is required"))?;
        use std::str::FromStr;
        dcc_common::PaymentMethod::from_str(payment_method_str)
            .map_err(|e| anyhow::anyhow!("Invalid payment method: {}", e))?;

        let created_at_ns = crate::now_ns()?;

        // Calculate duration and timestamps
        let duration_hours = params.duration_hours.unwrap_or(720); // Default: 30 days
        if duration_hours < 1 {
            return Err(anyhow::anyhow!(
                "duration_hours must be at least 1 (got {})",
                duration_hours
            ));
        }
        if let Some(min_hours) = offering.min_contract_hours {
            if duration_hours < min_hours {
                return Err(anyhow::anyhow!(
                    "Offering requires minimum {} hours (requested {})",
                    min_hours,
                    duration_hours
                ));
            }
        }
        if let Some(max_hours) = offering.max_contract_hours {
            if duration_hours > max_hours {
                return Err(anyhow::anyhow!(
                    "Offering allows maximum {} hours (requested {})",
                    max_hours,
                    duration_hours
                ));
            }
        }
        let start_timestamp_ns = created_at_ns;
        let end_timestamp_ns = start_timestamp_ns + (duration_hours * 3600 * 1_000_000_000);

        // Decode provider pubkey for comparison
        let offering_pubkey_bytes = hex::decode(&offering.pubkey)
            .map_err(|_| anyhow::anyhow!("Invalid pubkey hex in offering"))?;

        // Detect self-rental: requester_pubkey == provider_pubkey
        let is_self_rental = requester_pubkey == offering_pubkey_bytes.as_slice();

        // Calculate payment based on duration (monthly_price is per ~720 hours)
        // Self-rental is FREE - no payment required
        // Use integer arithmetic to avoid floating-point precision issues:
        // 1. Convert monthly_price to e9s (1 float multiply, unavoidable since price is f64)
        // 2. Use i128 for the ratio calculation to avoid overflow
        let payment_amount_e9s = if is_self_rental {
            0
        } else {
            let monthly_price_e9s = (offering.monthly_price * 1_000_000_000.0) as i128;
            (monthly_price_e9s * duration_hours as i128 / 720) as i64
        };

        // Generate deterministic contract ID from SHA256 hash of request data
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(requester_pubkey);
        hasher.update(&offering_pubkey_bytes);
        hasher.update(offering.offering_id.as_bytes());
        hasher.update(ssh_pubkey.as_bytes());
        hasher.update(contact.as_bytes());
        hasher.update(payment_amount_e9s.to_le_bytes());
        hasher.update(memo.as_bytes());
        hasher.update(created_at_ns.to_le_bytes());
        let contract_id = hasher.finalize().to_vec();

        // Insert contract request
        let original_duration_hours = duration_hours;
        let requested_status = ContractStatus::Requested.to_string();
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        // Set payment_status based on payment method and self-rental
        // Self-rental is FREE - payment succeeds immediately
        // ICPay payments are pre-paid, so they succeed immediately
        // Stripe payments require webhook confirmation, so they start as pending
        let payment_status = if is_self_rental || payment_method_str == "icpay" {
            "succeeded"
        } else {
            "pending"
        };

        // Ensure accounts exist for both requester and provider
        let requester_account_id = self.ensure_account_for_pubkey(requester_pubkey).await?;
        let provider_account_id = self
            .ensure_account_for_pubkey(&offering_pubkey_bytes)
            .await?;

        let is_self_provisioned = offering.offering_source.as_deref() == Some("self_provisioned");
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"INSERT INTO contract_sign_requests (
                contract_id, requester_pubkey, requester_ssh_pubkey,
                requester_contact, provider_pubkey, offering_id,
                payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
                duration_hours, original_duration_hours, request_memo,
                created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency, buyer_address,
                requester_account_id, provider_account_id, operating_system
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23)"#,
            contract_id,
            requester_pubkey,
            ssh_pubkey,
            contact,
            offering_pubkey_bytes,
            offering.offering_id,
            payment_amount_e9s,
            start_timestamp_ns,
            end_timestamp_ns,
            duration_hours,
            original_duration_hours,
            memo,
            created_at_ns,
            requested_status,
            payment_method_str,
            stripe_payment_intent_id,
            stripe_customer_id,
            payment_status,
            offering.currency,
            params.buyer_address,
            requester_account_id,
            provider_account_id,
            params.operating_system
        )
        .execute(&mut *tx)
        .await?;

        if is_self_provisioned {
            let reserved: Option<(i64,)> = sqlx::query_as(
                r#"
                UPDATE cloud_resources
                SET contract_id = $1,
                    updated_at = NOW()
                WHERE id = (
                    SELECT id
                    FROM cloud_resources
                    WHERE offering_id = $2
                      AND listing_mode = 'marketplace'
                      AND status = 'running'
                      AND terminated_at IS NULL
                      AND contract_id IS NULL
                    ORDER BY created_at ASC
                    LIMIT 1
                )
                RETURNING offering_id
                "#,
            )
            .bind(&contract_id)
            .bind(params.offering_db_id)
            .fetch_optional(&mut *tx)
            .await?;

            if reserved.is_none() {
                tx.rollback().await?;
                return Err(anyhow::anyhow!(
                    "Self-provisioned offering {} is out of stock",
                    offering.offer_name
                ));
            }

            sqlx::query(
                r#"
                UPDATE provider_offerings
                SET stock_status = 'out_of_stock'
                WHERE id = $1
                "#,
            )
            .bind(params.offering_db_id)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            r#"INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at)
               VALUES ($1, 'status_change', NULL, $2, 'tenant', NULL, $3)"#,
        )
        .bind(&contract_id)
        .bind(&requested_status)
        .bind(created_at_ns)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(contract_id)
    }

    /// Update contract status with authorization check and state transition validation
    ///
    /// # Errors
    /// - Contract not found
    /// - Unauthorized (only provider can update)
    /// - Invalid status string
    /// - Invalid state transition (e.g. active -> requested)
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
        if contract.provider_pubkey != hex::encode(updated_by_pubkey) {
            return Err(anyhow::anyhow!(
                "Unauthorized: only provider can update contract status"
            ));
        }

        // Parse and validate the new status
        let target_status: ContractStatus =
            new_status.parse().map_err(|e| anyhow::anyhow!("{}", e))?;

        // Parse current status and validate transition
        let current_status: ContractStatus = contract.status.parse().map_err(|e| {
            anyhow::anyhow!(
                "Contract {} has invalid status '{}' in database: {}",
                hex::encode(contract_id),
                contract.status,
                e
            )
        })?;

        // Validate state transition
        if !current_status.can_transition_to(target_status) {
            return Err(anyhow::anyhow!(
                "Invalid status transition: {} -> {}. Valid transitions from '{}': {:?}",
                current_status,
                target_status,
                current_status,
                current_status.valid_transitions()
            ));
        }

        // Convert target status to string for database storage
        let new_status_str = target_status.to_string();

        // Update status and history atomically
        let updated_at_ns = crate::now_ns()?;
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3 WHERE contract_id = $4",
            new_status_str,
            updated_at_ns,
            updated_by_pubkey,
            contract_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!("INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id,
            contract.status,
            new_status_str,
            updated_by_pubkey,
            updated_at_ns,
            change_memo
        )
            .execute(&mut *tx)
            .await?;

        sqlx::query!(
            r#"INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at)
               VALUES ($1, 'status_change', $2, $3, 'provider', $4, $5)"#,
            contract_id,
            contract.status,
            new_status_str,
            change_memo,
            updated_at_ns
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    /// Reject a rental request with full refund (provider-initiated)
    ///
    /// Unlike cancellation (user-initiated, prorated), rejection gives full refund
    /// since the user never received the service.
    pub async fn reject_contract(
        &self,
        contract_id: &[u8],
        rejected_by_pubkey: &[u8],
        reject_memo: Option<&str>,
        stripe_client: Option<&crate::stripe_client::StripeClient>,
        icpay_client: Option<&crate::icpay_client::IcpayClient>,
    ) -> Result<()> {
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow::anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;

        // Only provider can reject
        if contract.provider_pubkey != hex::encode(rejected_by_pubkey) {
            return Err(anyhow::anyhow!(
                "Unauthorized: only provider can reject rental request"
            ));
        }

        // Can only reject contracts in valid states (those that can transition to rejected)
        let current_status: ContractStatus = contract.status.parse().map_err(|e| {
            anyhow::anyhow!("Contract has invalid status '{}': {}", contract.status, e)
        })?;
        if !current_status.can_transition_to(ContractStatus::Rejected) {
            return Err(anyhow::anyhow!(
                "Contract cannot be rejected in '{}' status. Valid for: requested, pending, accepted",
                contract.status
            ));
        }

        // Full refund if payment succeeded (user never got the service)
        let (refund_amount_e9s, stripe_refund_id, icpay_refund_id) = if contract.payment_status
            == "succeeded"
        {
            let full_refund = contract.payment_amount_e9s;
            match contract.payment_method.as_str() {
                "stripe" => {
                    // Prefer real PaymentIntent ID (pi_*); fall back to checkout session
                    // ID (cs_*) for legacy rows that predate the column split.
                    let stripe_id = contract
                        .stripe_payment_intent_id
                        .as_deref()
                        .or(contract.stripe_checkout_session_id.as_deref());
                    if let Some(payment_intent_id) = stripe_id {
                        if let Some(client) = stripe_client {
                            let refund_cents = full_refund / 10_000_000;
                            match client
                                .create_refund(payment_intent_id, Some(refund_cents), None)
                                .await
                            {
                                Ok(refund_id) => {
                                    tracing::info!(
                                            "Stripe full refund created: {} for rejected contract {} (amount: {} cents)",
                                            refund_id,
                                            hex::encode(contract_id),
                                            refund_cents
                                        );
                                    (Some(full_refund), Some(refund_id), None)
                                }
                                Err(e) => {
                                    tracing::error!(
                                            "Failed to create Stripe refund for rejected contract {}: {}",
                                            hex::encode(contract_id),
                                            e
                                        );
                                    (Some(full_refund), None, None)
                                }
                            }
                        } else {
                            (Some(full_refund), None, None)
                        }
                    } else {
                        (Some(full_refund), None, None)
                    }
                }
                "icpay" => {
                    if let Some(client) = icpay_client {
                        if let Some(payment_id) = &contract.icpay_payment_id {
                            match client.create_refund(payment_id, Some(full_refund)).await {
                                Ok(refund_id) => {
                                    tracing::info!(
                                        "ICPay full refund created: {} for rejected contract {}",
                                        refund_id,
                                        hex::encode(contract_id)
                                    );
                                    (Some(full_refund), None, Some(refund_id))
                                }
                                Err(e) => {
                                    tracing::error!(
                                            "Failed to create ICPay refund for rejected contract {}: {}",
                                            hex::encode(contract_id),
                                            e
                                        );
                                    (Some(full_refund), None, None)
                                }
                            }
                        } else {
                            (Some(full_refund), None, None)
                        }
                    } else {
                        (Some(full_refund), None, None)
                    }
                }
                _ => (None, None, None),
            }
        } else {
            (None, None, None)
        };

        // Update status and refund info atomically
        let updated_at_ns = crate::now_ns()?;
        let rejected_status = ContractStatus::Rejected.to_string();
        let mut tx = self.pool.begin().await?;

        if refund_amount_e9s.is_some() || stripe_refund_id.is_some() || icpay_refund_id.is_some() {
            sqlx::query!(
                "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3, payment_status = $4, refund_amount_e9s = $5, stripe_refund_id = $6, icpay_refund_id = $7, refund_created_at_ns = $8 WHERE contract_id = $9",
                rejected_status,
                updated_at_ns,
                rejected_by_pubkey,
                "refunded",
                refund_amount_e9s,
                stripe_refund_id,
                icpay_refund_id,
                updated_at_ns,
                contract_id
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3 WHERE contract_id = $4",
                rejected_status,
                updated_at_ns,
                rejected_by_pubkey,
                contract_id
            )
            .execute(&mut *tx)
            .await?;
        }

        // Record in history
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id,
            contract.status,
            rejected_status,
            rejected_by_pubkey,
            updated_at_ns,
            reject_memo
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at)
               VALUES ($1, 'status_change', $2, $3, 'provider', $4, $5)"#,
            contract_id,
            contract.status,
            rejected_status,
            reject_memo,
            updated_at_ns
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        if let Err(e) = self.release_self_provisioned_resource(contract_id).await {
            tracing::warn!(
                "Failed to release self-provisioned resource for rejected contract {}: {:#}",
                hex::encode(contract_id),
                e
            );
        }

        Ok(())
    }

    /// Check if a contract status is cancellable
    fn is_cancellable_status(status: &str) -> bool {
        status
            .parse::<ContractStatus>()
            .map(|s| s.is_cancellable())
            .unwrap_or(false)
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
    ///
    /// For Stripe and ICPay payments: automatically processes prorated refund
    pub async fn cancel_contract(
        &self,
        contract_id: &[u8],
        cancelled_by_pubkey: &[u8],
        cancel_memo: Option<&str>,
        stripe_client: Option<&crate::stripe_client::StripeClient>,
        icpay_client: Option<&crate::icpay_client::IcpayClient>,
    ) -> Result<()> {
        // Get contract to verify it exists and check authorization
        let contract = self.get_contract(contract_id).await?.ok_or_else(|| {
            anyhow::anyhow!("Contract not found (ID: {})", hex::encode(contract_id))
        })?;

        // Verify authorization: only requester can cancel their own request
        if contract.requester_pubkey != hex::encode(cancelled_by_pubkey) {
            return Err(anyhow::anyhow!(
                "Unauthorized: only the requester can cancel their rental request"
            ));
        }

        // Verify contract is in a cancellable status
        if !Self::is_cancellable_status(&contract.status) {
            return Err(anyhow::anyhow!(
                "Contract cannot be cancelled in '{}' status. Only requested, pending, accepted, provisioning, provisioned, or active contracts can be cancelled.",
                contract.status
            ));
        }

        // Calculate prorated refund based on payment method
        let current_timestamp_ns = crate::now_ns()?;
        let (refund_amount_e9s, stripe_refund_id, icpay_refund_id) = if contract.payment_status
            == "succeeded"
        {
            match contract.payment_method.as_str() {
                "stripe" => {
                    // Prefer real PaymentIntent ID (pi_*); fall back to checkout session
                    // ID (cs_*) for legacy rows that predate the column split.
                    let stripe_id = contract
                        .stripe_payment_intent_id
                        .as_deref()
                        .or(contract.stripe_checkout_session_id.as_deref());
                    if let Some(payment_intent_id) = stripe_id {
                        // Calculate prorated refund based on when service became active.
                        // Pause credit comes from `total_paused_ns`; failure to read it is
                        // fatal here because under-crediting a refund silently is worse than
                        // a loud cancel failure (operator sees, retries).
                        let total_paused_ns = self.get_total_paused_ns(contract_id).await?;
                        let refund_e9s = Self::calculate_prorated_refund(
                            contract.payment_amount_e9s,
                            contract.provisioning_completed_at_ns,
                            contract.end_timestamp_ns,
                            current_timestamp_ns,
                            total_paused_ns,
                        );

                        // Only process refund if amount is positive and stripe_client is provided
                        if refund_e9s > 0 {
                            if let Some(client) = stripe_client {
                                // Convert e9s to cents for Stripe (e9s / 10_000_000 = cents)
                                let refund_cents = refund_e9s / 10_000_000;

                                // Create refund via Stripe API
                                match client
                                    .create_refund(payment_intent_id, Some(refund_cents), None)
                                    .await
                                {
                                    Ok(refund_id) => {
                                        tracing::info!(
                                            "Stripe refund created: {} for contract {} (amount: {} cents)",
                                            refund_id,
                                            hex::encode(contract_id),
                                            refund_cents
                                        );
                                        (Some(refund_e9s), Some(refund_id), None)
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to create Stripe refund for contract {}: {:#}",
                                            hex::encode(contract_id),
                                            e
                                        );
                                        (Some(refund_e9s), None, None)
                                    }
                                }
                            } else {
                                // No stripe_client provided, just track the calculated amount
                                (Some(refund_e9s), None, None)
                            }
                        } else {
                            (None, None, None)
                        }
                    } else {
                        (None, None, None)
                    }
                }
                "icpay" => {
                    let (amount, refund_id) = self
                        .process_icpay_refund(&contract, icpay_client, current_timestamp_ns)
                        .await?;
                    (amount, None, refund_id)
                }
                _ => (None, None, None),
            }
        } else {
            // Payment not succeeded yet
            (None, None, None)
        };

        // Update status, refund info, and history atomically
        let updated_at_ns = crate::now_ns()?;
        let cancelled_status = ContractStatus::Cancelled.to_string();
        let mut tx = self.pool.begin().await?;

        // Update contract status to cancelled with refund info
        if refund_amount_e9s.is_some() || stripe_refund_id.is_some() || icpay_refund_id.is_some() {
            sqlx::query!(
                "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3, payment_status = $4, refund_amount_e9s = $5, stripe_refund_id = $6, icpay_refund_id = $7, refund_created_at_ns = $8 WHERE contract_id = $9",
                cancelled_status,
                updated_at_ns,
                cancelled_by_pubkey,
                "refunded",
                refund_amount_e9s,
                stripe_refund_id,
                icpay_refund_id,
                updated_at_ns,
                contract_id
            )
            .execute(&mut *tx)
            .await?;
        } else {
            sqlx::query!(
                "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3 WHERE contract_id = $4",
                cancelled_status,
                updated_at_ns,
                cancelled_by_pubkey,
                contract_id
            )
            .execute(&mut *tx)
            .await?;
        }

        // Record status change in history
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id,
            contract.status,
            cancelled_status,
            cancelled_by_pubkey,
            updated_at_ns,
            cancel_memo
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at)
               VALUES ($1, 'status_change', $2, $3, 'tenant', $4, $5)"#,
            contract_id,
            contract.status,
            cancelled_status,
            cancel_memo,
            updated_at_ns
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        if self.release_self_provisioned_resource(contract_id).await? {
            return Ok(());
        }

        // Trigger cloud_resource deletion if this contract has a linked resource
        if let Err(e) = self.mark_contract_resource_for_deletion(contract_id).await {
            tracing::warn!(
                "Failed to mark cloud resource for deletion for contract {}: {}",
                hex::encode(contract_id),
                e
            );
        }

        Ok(())
    }
}
