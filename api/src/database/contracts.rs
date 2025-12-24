use super::types::Database;
use anyhow::Result;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct Contract {
    #[ts(type = "string")]
    pub contract_id: String,
    #[ts(type = "string")]
    pub requester_pubkey: String,
    pub requester_ssh_pubkey: String,
    pub requester_contact: String,
    #[ts(type = "string")]
    pub provider_pubkey: String,
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
    pub payment_method: String,
    #[oai(skip_serializing_if_is_none)]
    pub stripe_payment_intent_id: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub stripe_customer_id: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub icpay_transaction_id: Option<String>,
    pub payment_status: String,
    pub currency: String,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub refund_amount_e9s: Option<i64>,
    #[oai(skip_serializing_if_is_none)]
    pub stripe_refund_id: Option<String>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub refund_created_at_ns: Option<i64>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub status_updated_at_ns: Option<i64>,
    #[oai(skip_serializing_if_is_none)]
    pub icpay_payment_id: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub icpay_refund_id: Option<String>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub total_released_e9s: Option<i64>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub last_release_at_ns: Option<i64>,
    // Tax tracking (from Stripe Tax or manual entry)
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub tax_amount_e9s: Option<i64>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub tax_rate_percent: Option<f64>,
    #[oai(skip_serializing_if_is_none)]
    pub tax_type: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub tax_jurisdiction: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub customer_tax_id: Option<String>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub reverse_charge: Option<i64>,
    /// Buyer address for B2B invoices
    #[oai(skip_serializing_if_is_none)]
    pub buyer_address: Option<String>,
    /// Stripe invoice ID for invoice PDF retrieval
    #[oai(skip_serializing_if_is_none)]
    pub stripe_invoice_id: Option<String>,
    /// Receipt tracking
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub receipt_number: Option<i64>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub receipt_sent_at_ns: Option<i64>,
    // Subscription tracking (for recurring billing)
    #[oai(skip_serializing_if_is_none)]
    pub stripe_subscription_id: Option<String>,
    #[oai(skip_serializing_if_is_none)]
    pub subscription_status: Option<String>,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub current_period_end_ns: Option<i64>,
    #[ts(type = "boolean")]
    #[sqlx(default)]
    pub cancel_at_period_end: bool,
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

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PaymentRelease {
    pub id: i64,
    pub contract_id: Vec<u8>,
    pub release_type: String,
    pub period_start_ns: i64,
    pub period_end_ns: i64,
    pub amount_e9s: i64,
    pub provider_pubkey: Vec<u8>,
    pub status: String,
    pub created_at_ns: i64,
    pub released_at_ns: Option<i64>,
    pub payout_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
pub struct ProviderPendingReleases {
    #[oai(skip)]
    pub provider_pubkey: Vec<u8>,
    pub total_pending_e9s: i64,
    pub release_count: i64,
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
    #[oai(skip_serializing_if_is_none)]
    pub payment_method: Option<String>,
    /// Buyer address for B2B invoices (street, city, postal code, country)
    #[oai(skip_serializing_if_is_none)]
    pub buyer_address: Option<String>,
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

/// Contract with offering specs for dc-agent provisioning
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
#[serde(rename_all = "camelCase")]
#[oai(skip_serializing_if_is_none)]
pub struct ContractWithSpecs {
    pub contract_id: String,
    pub offering_id: String,
    pub requester_ssh_pubkey: String,
    #[oai(skip_serializing_if_is_none)]
    pub instance_config: Option<String>,
    /// CPU cores from offering (processor_cores)
    #[oai(skip_serializing_if_is_none)]
    pub cpu_cores: Option<i64>,
    /// Memory amount from offering (e.g. "16 GB")
    #[oai(skip_serializing_if_is_none)]
    pub memory_amount: Option<String>,
    /// Storage capacity from offering (e.g. "100 GB")
    #[oai(skip_serializing_if_is_none)]
    pub storage_capacity: Option<String>,
    /// Provisioner type from offering (e.g. "proxmox", "script", "manual")
    /// NULL = use agent's default provisioner
    #[oai(skip_serializing_if_is_none)]
    pub provisioner_type: Option<String>,
    /// Provisioner config JSON from offering
    #[oai(skip_serializing_if_is_none)]
    pub provisioner_config: Option<String>,
}

/// Contract pending termination for dc-agent
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
#[serde(rename_all = "camelCase")]
pub struct ContractPendingTermination {
    pub contract_id: String,
    /// Instance details JSON (contains external_id needed for termination)
    pub instance_details: String,
}

impl Database {
    /// Get contracts for a user (as requester)
    pub async fn get_user_contracts(&self, pubkey: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(hex(contract_id)) as "contract_id!: String", lower(hex(requester_pubkey)) as "requester_pubkey!: String", requester_ssh_pubkey as "requester_ssh_pubkey!", requester_contact as "requester_contact!", lower(hex(provider_pubkey)) as "provider_pubkey!: String",
               offering_id as "offering_id!", region_name, instance_config, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
               duration_hours, original_duration_hours, request_memo as "request_memo!", created_at_ns, status as "status!",
               provisioning_instance_details, provisioning_completed_at_ns, payment_method as "payment_method!", stripe_payment_intent_id, stripe_customer_id, icpay_transaction_id, payment_status as "payment_status!",
               currency as "currency!", refund_amount_e9s, stripe_refund_id, refund_created_at_ns, status_updated_at_ns, icpay_payment_id, icpay_refund_id, total_released_e9s, last_release_at_ns,
               tax_amount_e9s, tax_rate_percent, tax_type, tax_jurisdiction, customer_tax_id, reverse_charge, buyer_address, stripe_invoice_id, receipt_number, receipt_sent_at_ns,
               stripe_subscription_id, subscription_status, current_period_end_ns, COALESCE(cancel_at_period_end, 0) as "cancel_at_period_end!: bool"
               FROM contract_sign_requests WHERE requester_pubkey = ? ORDER BY created_at_ns DESC"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get contracts for a provider
    pub async fn get_provider_contracts(&self, pubkey: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(hex(contract_id)) as "contract_id!: String", lower(hex(requester_pubkey)) as "requester_pubkey!: String", requester_ssh_pubkey as "requester_ssh_pubkey!", requester_contact as "requester_contact!", lower(hex(provider_pubkey)) as "provider_pubkey!: String",
               offering_id as "offering_id!", region_name, instance_config, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
               duration_hours, original_duration_hours, request_memo as "request_memo!", created_at_ns, status as "status!",
               provisioning_instance_details, provisioning_completed_at_ns, payment_method as "payment_method!", stripe_payment_intent_id, stripe_customer_id, icpay_transaction_id, payment_status as "payment_status!",
               currency as "currency!", refund_amount_e9s, stripe_refund_id, refund_created_at_ns, status_updated_at_ns, icpay_payment_id, icpay_refund_id, total_released_e9s, last_release_at_ns,
               tax_amount_e9s, tax_rate_percent, tax_type, tax_jurisdiction, customer_tax_id, reverse_charge, buyer_address, stripe_invoice_id, receipt_number, receipt_sent_at_ns,
               stripe_subscription_id, subscription_status, current_period_end_ns, COALESCE(cancel_at_period_end, 0) as "cancel_at_period_end!: bool"
               FROM contract_sign_requests WHERE provider_pubkey = ? ORDER BY created_at_ns DESC"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get pending contracts for a provider
    pub async fn get_pending_provider_contracts(&self, pubkey: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(hex(contract_id)) as "contract_id!: String", lower(hex(requester_pubkey)) as "requester_pubkey!: String", requester_ssh_pubkey as "requester_ssh_pubkey!", requester_contact as "requester_contact!", lower(hex(provider_pubkey)) as "provider_pubkey!: String",
               offering_id as "offering_id!", region_name, instance_config, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
               duration_hours, original_duration_hours, request_memo as "request_memo!", created_at_ns, status as "status!",
               provisioning_instance_details, provisioning_completed_at_ns, payment_method as "payment_method!", stripe_payment_intent_id, stripe_customer_id, icpay_transaction_id, payment_status as "payment_status!",
               currency as "currency!", refund_amount_e9s, stripe_refund_id, refund_created_at_ns, status_updated_at_ns, icpay_payment_id, icpay_refund_id, total_released_e9s, last_release_at_ns,
               tax_amount_e9s, tax_rate_percent, tax_type, tax_jurisdiction, customer_tax_id, reverse_charge, buyer_address, stripe_invoice_id, receipt_number, receipt_sent_at_ns,
               stripe_subscription_id, subscription_status, current_period_end_ns, COALESCE(cancel_at_period_end, 0) as "cancel_at_period_end!: bool"
               FROM contract_sign_requests WHERE provider_pubkey = ? AND status IN ('requested', 'pending') ORDER BY created_at_ns DESC"#,
            pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Get cancelled contracts pending termination
    ///
    /// Returns contracts that are cancelled, have instance details (were provisioned),
    /// and have not yet been terminated by dc-agent.
    pub async fn get_pending_termination_contracts(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<ContractPendingTermination>> {
        let contracts = sqlx::query_as!(
            ContractPendingTermination,
            r#"SELECT
               lower(hex(contract_id)) as "contract_id!: String",
               provisioning_instance_details as "instance_details!: String"
               FROM contract_sign_requests
               WHERE provider_pubkey = ?
               AND status = 'cancelled'
               AND provisioning_instance_details IS NOT NULL
               AND terminated_at_ns IS NULL
               ORDER BY status_updated_at_ns ASC"#,
            provider_pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }

    /// Mark a contract as terminated by dc-agent
    pub async fn mark_contract_terminated(&self, contract_id: &[u8]) -> Result<()> {
        let terminated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let result = sqlx::query!(
            "UPDATE contract_sign_requests SET terminated_at_ns = ? WHERE contract_id = ? AND status = 'cancelled'",
            terminated_at_ns,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!(
                "Contract not found or not in cancelled status (ID: {})",
                hex::encode(contract_id)
            ));
        }

        Ok(())
    }

    /// Get contract by ID
    pub async fn get_contract(&self, contract_id: &[u8]) -> Result<Option<Contract>> {
        let contract = sqlx::query_as!(
            Contract,
            r#"SELECT lower(hex(contract_id)) as "contract_id!: String", lower(hex(requester_pubkey)) as "requester_pubkey!: String", requester_ssh_pubkey as "requester_ssh_pubkey!", requester_contact as "requester_contact!", lower(hex(provider_pubkey)) as "provider_pubkey!: String",
               offering_id as "offering_id!", region_name, instance_config, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
               duration_hours, original_duration_hours, request_memo as "request_memo!", created_at_ns, status as "status!",
               provisioning_instance_details, provisioning_completed_at_ns, payment_method as "payment_method!", stripe_payment_intent_id, stripe_customer_id, icpay_transaction_id, payment_status as "payment_status!",
               currency as "currency!", refund_amount_e9s, stripe_refund_id, refund_created_at_ns, status_updated_at_ns, icpay_payment_id, icpay_refund_id, total_released_e9s, last_release_at_ns,
               tax_amount_e9s, tax_rate_percent, tax_type, tax_jurisdiction, customer_tax_id, reverse_charge, buyer_address, stripe_invoice_id, receipt_number, receipt_sent_at_ns,
               stripe_subscription_id, subscription_status, current_period_end_ns, COALESCE(cancel_at_period_end, 0) as "cancel_at_period_end!: bool"
               FROM contract_sign_requests WHERE contract_id = ?"#,
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(contract)
    }

    /// Get contract reply
    #[allow(dead_code)]
    pub async fn get_contract_reply(&self, contract_id: &[u8]) -> Result<Option<ContractReply>> {
        let reply = sqlx::query_as!(
            ContractReply,
            "SELECT contract_id, provider_pubkey, reply_status, reply_memo, instance_details, created_at_ns FROM contract_sign_replies WHERE contract_id = ?",
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(reply)
    }

    /// Get contract payment entries
    #[allow(dead_code)]
    pub async fn get_contract_payments(&self, contract_id: &[u8]) -> Result<Vec<PaymentEntry>> {
        let payments = sqlx::query_as!(
            PaymentEntry,
            "SELECT pricing_model, time_period_unit, quantity, amount_e9s FROM contract_payment_entries WHERE contract_id = ?",
            contract_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(payments)
    }

    /// Get all contracts with pagination
    pub async fn list_contracts(&self, limit: i64, offset: i64) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(hex(contract_id)) as "contract_id!: String", lower(hex(requester_pubkey)) as "requester_pubkey!: String", requester_ssh_pubkey as "requester_ssh_pubkey!", requester_contact as "requester_contact!", lower(hex(provider_pubkey)) as "provider_pubkey!: String",
               offering_id as "offering_id!", region_name, instance_config, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
               duration_hours, original_duration_hours, request_memo as "request_memo!", created_at_ns, status as "status!",
               provisioning_instance_details, provisioning_completed_at_ns, payment_method as "payment_method!", stripe_payment_intent_id, stripe_customer_id, icpay_transaction_id, payment_status as "payment_status!",
               currency as "currency!", refund_amount_e9s, stripe_refund_id, refund_created_at_ns, status_updated_at_ns, icpay_payment_id, icpay_refund_id, total_released_e9s, last_release_at_ns,
               tax_amount_e9s, tax_rate_percent, tax_type, tax_jurisdiction, customer_tax_id, reverse_charge, buyer_address, stripe_invoice_id, receipt_number, receipt_sent_at_ns,
               stripe_subscription_id, subscription_status, current_period_end_ns, COALESCE(cancel_at_period_end, 0) as "cancel_at_period_end!: bool"
               FROM contract_sign_requests ORDER BY created_at_ns DESC LIMIT ? OFFSET ?"#,
            limit,
            offset
        )
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
        let offering_pubkey_bytes = hex::decode(&offering.pubkey)
            .map_err(|_| anyhow::anyhow!("Invalid pubkey hex in offering"))?;
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
        let requested_status = "requested";
        let stripe_payment_intent_id: Option<&str> = None;
        let stripe_customer_id: Option<&str> = None;

        // Set payment_status based on payment method
        // ICPay payments are pre-paid, so they succeed immediately
        // Stripe payments require webhook confirmation, so they start as pending
        let payment_status = if payment_method_str == "icpay" {
            "succeeded"
        } else {
            "pending"
        };

        // Ensure accounts exist for both requester and provider
        let requester_account_id = self.ensure_account_for_pubkey(requester_pubkey).await?;
        let provider_account_id = self
            .ensure_account_for_pubkey(&offering_pubkey_bytes)
            .await?;

        sqlx::query!(
            r#"INSERT INTO contract_sign_requests (
                contract_id, requester_pubkey, requester_ssh_pubkey,
                requester_contact, provider_pubkey, offering_id,
                payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
                duration_hours, original_duration_hours, request_memo,
                created_at_ns, status, payment_method, stripe_payment_intent_id, stripe_customer_id, payment_status, currency, buyer_address,
                requester_account_id, provider_account_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
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
            provider_account_id
        )
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
        if contract.provider_pubkey != hex::encode(updated_by_pubkey) {
            return Err(anyhow::anyhow!(
                "Unauthorized: only provider can update contract status"
            ));
        }

        // Update status and history atomically
        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ? WHERE contract_id = ?",
            new_status,
            updated_at_ns,
            updated_by_pubkey,
            contract_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!("INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES (?, ?, ?, ?, ?, ?)",
            contract_id,
            contract.status,
            new_status,
            updated_by_pubkey,
            updated_at_ns,
            change_memo
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

        // Can only reject contracts in requested/pending status
        let status_lower = contract.status.to_lowercase();
        if status_lower != "requested" && status_lower != "pending" {
            return Err(anyhow::anyhow!(
                "Contract cannot be rejected in '{}' status. Only requested/pending contracts can be rejected.",
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
                    if let Some(payment_intent_id) = &contract.stripe_payment_intent_id {
                        if let Some(client) = stripe_client {
                            let refund_cents = full_refund / 10_000_000;
                            match client
                                .create_refund(payment_intent_id, Some(refund_cents))
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
        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut tx = self.pool.begin().await?;

        if refund_amount_e9s.is_some() || stripe_refund_id.is_some() || icpay_refund_id.is_some() {
            sqlx::query!(
                "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ?, payment_status = ?, refund_amount_e9s = ?, stripe_refund_id = ?, icpay_refund_id = ?, refund_created_at_ns = ? WHERE contract_id = ?",
                "rejected",
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
                "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ? WHERE contract_id = ?",
                "rejected",
                updated_at_ns,
                rejected_by_pubkey,
                contract_id
            )
            .execute(&mut *tx)
            .await?;
        }

        // Record in history
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES (?, ?, ?, ?, ?, ?)",
            contract_id,
            contract.status,
            "rejected",
            rejected_by_pubkey,
            updated_at_ns,
            reject_memo
        )
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

        sqlx::query!(
            "UPDATE contract_sign_requests SET provisioning_instance_details = ?, provisioning_completed_at_ns = ? WHERE contract_id = ?",
            instance_details,
            provisioned_at_ns,
            contract_id
        )
        .execute(&mut *tx)
        .await?;

        let empty_instance_ip: Option<&str> = None;
        let empty_credentials: Option<&str> = None;
        sqlx::query!(
            r#"INSERT INTO contract_provisioning_details (contract_id, instance_ip, instance_credentials, connection_instructions, provisioned_at_ns)
               VALUES (?, ?, ?, ?, ?)
               ON CONFLICT(contract_id) DO UPDATE SET instance_ip = excluded.instance_ip, instance_credentials = excluded.instance_credentials, connection_instructions = excluded.connection_instructions, provisioned_at_ns = excluded.provisioned_at_ns"#,
            contract_id,
            empty_instance_ip,
            empty_credentials,
            instance_details,
            provisioned_at_ns
        )
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
        let extended_by_hex = hex::encode(extended_by_pubkey);
        if contract.requester_pubkey != extended_by_hex
            && contract.provider_pubkey != extended_by_hex
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
        sqlx::query!(
            "UPDATE contract_sign_requests SET end_timestamp_ns = ?, duration_hours = ? WHERE contract_id = ?",
            new_end_timestamp_ns,
            new_duration_hours,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        // Record extension in history
        sqlx::query!(
            "INSERT INTO contract_extensions (contract_id, extended_by_pubkey, extension_hours, extension_payment_e9s, previous_end_timestamp_ns, new_end_timestamp_ns, extension_memo, created_at_ns) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
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
               FROM contract_extensions WHERE contract_id = ? ORDER BY created_at_ns DESC"#,
            contract_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(extensions)
    }

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
            "UPDATE contract_sign_requests SET stripe_payment_intent_id = ?, payment_status = ?, tax_amount_e9s = ?, customer_tax_id = ?, reverse_charge = ?, stripe_invoice_id = ? WHERE contract_id = ?"
        )
        .bind(checkout_session_id)
        .bind("succeeded")
        .bind(tax_amount_e9s)
        .bind(customer_tax_id)
        .bind(if reverse_charge { 1 } else { 0 })
        .bind(stripe_invoice_id)
        .bind(contract_id)
        .execute(&self.pool)
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
            "UPDATE contract_sign_requests SET stripe_invoice_id = ? WHERE contract_id = ?",
            stripe_invoice_id,
            contract_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get offering by offering_id string
    async fn get_offering_by_id(
        &self,
        offering_id: &str,
    ) -> Result<Option<crate::database::offerings::Offering>> {
        let offering = sqlx::query_as::<_, crate::database::offerings::Offering>(
            r#"SELECT id, lower(hex(pubkey)) as pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price,
               setup_fee, visibility, product_type, virtualization_type, billing_interval, stock_status,
               processor_brand, processor_amount, processor_cores, processor_speed, processor_name,
               memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity,
               ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic,
               datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude,
               control_panel, gpu_name, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems
               FROM provider_offerings WHERE offering_id = ?"#
        )
        .bind(offering_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(offering)
    }

    /// Check if a contract status is cancellable
    fn is_cancellable_status(status: &str) -> bool {
        matches!(
            status,
            "requested" | "pending" | "accepted" | "provisioning" | "provisioned" | "active"
        )
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
    fn calculate_prorated_refund(
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

        // Calculate prorated refund: (time_remaining / total_duration) * payment_amount
        let refund_amount = (payment_amount_e9s as f64 * time_remaining_ns as f64
            / total_duration_ns as f64) as i64;

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
            "UPDATE contract_sign_requests SET icpay_transaction_id = ? WHERE contract_id = ?",
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
            "UPDATE contract_sign_requests SET icpay_payment_id = ?, payment_status = ? WHERE contract_id = ?",
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
            "UPDATE contract_sign_requests SET payment_status = ? WHERE contract_id = ?",
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
    #[cfg_attr(test, allow(dead_code))]
    async fn process_icpay_refund(
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
                        eprintln!(
                            "ICPay refund created: {} for contract {} (amount: {} e9s)",
                            refund_id, &contract.contract_id, net_refund_e9s
                        );
                        Ok((Some(net_refund_e9s), Some(refund_id)))
                    }
                    Err(e) => {
                        // Log error but don't fail cancellation
                        eprintln!(
                            "Failed to create ICPay refund for contract {}: {}",
                            &contract.contract_id, e
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
        let current_timestamp_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let (refund_amount_e9s, stripe_refund_id, icpay_refund_id) = if contract.payment_status
            == "succeeded"
        {
            match contract.payment_method.as_str() {
                "stripe" => {
                    if let Some(payment_intent_id) = &contract.stripe_payment_intent_id {
                        // Calculate prorated refund based on when service became active
                        let refund_e9s = Self::calculate_prorated_refund(
                            contract.payment_amount_e9s,
                            contract.provisioning_completed_at_ns,
                            contract.end_timestamp_ns,
                            current_timestamp_ns,
                        );

                        // Only process refund if amount is positive and stripe_client is provided
                        if refund_e9s > 0 {
                            if let Some(client) = stripe_client {
                                // Convert e9s to cents for Stripe (e9s / 10_000_000 = cents)
                                let refund_cents = refund_e9s / 10_000_000;

                                // Create refund via Stripe API
                                match client
                                    .create_refund(payment_intent_id, Some(refund_cents))
                                    .await
                                {
                                    Ok(refund_id) => {
                                        eprintln!(
                                            "Stripe refund created: {} for contract {} (amount: {} cents)",
                                            refund_id,
                                            hex::encode(contract_id),
                                            refund_cents
                                        );
                                        (Some(refund_e9s), Some(refund_id), None)
                                    }
                                    Err(e) => {
                                        // Log error but don't fail cancellation
                                        eprintln!(
                                            "Failed to create Stripe refund for contract {}: {}",
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
        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut tx = self.pool.begin().await?;

        // Update contract status to cancelled with refund info
        if refund_amount_e9s.is_some() || stripe_refund_id.is_some() || icpay_refund_id.is_some() {
            sqlx::query!(
                "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ?, payment_status = ?, refund_amount_e9s = ?, stripe_refund_id = ?, icpay_refund_id = ?, refund_created_at_ns = ? WHERE contract_id = ?",
                "cancelled",
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
                "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ? WHERE contract_id = ?",
                "cancelled",
                updated_at_ns,
                cancelled_by_pubkey,
                contract_id
            )
            .execute(&mut *tx)
            .await?;
        }

        // Record status change in history
        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES (?, ?, ?, ?, ?, ?)",
            contract_id,
            contract.status,
            "cancelled",
            cancelled_by_pubkey,
            updated_at_ns,
            cancel_memo
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Get active ICPay contracts ready for daily release
    pub async fn get_contracts_for_release(&self) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(hex(contract_id)) as "contract_id!: String", lower(hex(requester_pubkey)) as "requester_pubkey!: String", requester_ssh_pubkey as "requester_ssh_pubkey!", requester_contact as "requester_contact!", lower(hex(provider_pubkey)) as "provider_pubkey!: String",
               offering_id as "offering_id!", region_name, instance_config, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
               duration_hours, original_duration_hours, request_memo as "request_memo!", created_at_ns, status as "status!",
               provisioning_instance_details, provisioning_completed_at_ns, payment_method as "payment_method!", stripe_payment_intent_id, stripe_customer_id, icpay_transaction_id, payment_status as "payment_status!",
               currency as "currency!", refund_amount_e9s, stripe_refund_id, refund_created_at_ns, status_updated_at_ns, icpay_payment_id, icpay_refund_id, total_released_e9s, last_release_at_ns,
               tax_amount_e9s, tax_rate_percent, tax_type, tax_jurisdiction, customer_tax_id, reverse_charge, buyer_address, stripe_invoice_id, receipt_number, receipt_sent_at_ns,
               stripe_subscription_id, subscription_status, current_period_end_ns, COALESCE(cancel_at_period_end, 0) as "cancel_at_period_end!: bool"
               FROM contract_sign_requests
               WHERE payment_method = 'icpay'
               AND payment_status = 'succeeded'
               AND status IN ('active', 'provisioned')
               ORDER BY created_at_ns ASC"#
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
        let created_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let status = "pending";

        let result = sqlx::query!(
            r#"INSERT INTO payment_releases (contract_id, release_type, period_start_ns, period_end_ns, amount_e9s, provider_pubkey, status, created_at_ns)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
            contract_id,
            release_type,
            period_start_ns,
            period_end_ns,
            amount_e9s,
            provider_pubkey,
            status,
            created_at_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(PaymentRelease {
            id: result.last_insert_rowid(),
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
            "UPDATE contract_sign_requests SET last_release_at_ns = ?, total_released_e9s = ? WHERE contract_id = ?",
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
               WHERE provider_pubkey = ? AND status = 'pending'
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

        // Build placeholders for IN clause
        let placeholders = (0..release_ids.len())
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        let query = format!(
            "UPDATE payment_releases SET status = ?, payout_id = ? WHERE id IN ({})",
            placeholders
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
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let first_attempt_ns = now_ns + 60_000_000_000; // 1 minute

        sqlx::query!(
            "INSERT OR IGNORE INTO pending_stripe_receipts (contract_id, created_at_ns, next_attempt_at_ns, attempts) VALUES (?, ?, ?, 0)",
            contract_id,
            now_ns,
            first_attempt_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get pending Stripe receipts ready for processing
    pub async fn get_pending_stripe_receipts(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingStripeReceipt>> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let rows = sqlx::query!(
            r#"SELECT contract_id, created_at_ns, next_attempt_at_ns, attempts
               FROM pending_stripe_receipts
               WHERE next_attempt_at_ns <= ?
               ORDER BY next_attempt_at_ns ASC
               LIMIT ?"#,
            now_ns,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .filter_map(|r| {
                Some(PendingStripeReceipt {
                    contract_id: r.contract_id?,
                    created_at_ns: r.created_at_ns,
                    next_attempt_at_ns: r.next_attempt_at_ns,
                    attempts: r.attempts,
                })
            })
            .collect())
    }

    /// Update pending receipt for next retry (1 minute intervals, max 5 attempts)
    pub async fn update_pending_stripe_receipt_retry(&self, contract_id: &[u8]) -> Result<bool> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let next_attempt_ns = now_ns + 60_000_000_000; // 1 minute

        // Increment attempts and update next_attempt_at_ns
        // Only if attempts < 5
        let result = sqlx::query!(
            "UPDATE pending_stripe_receipts SET attempts = attempts + 1, next_attempt_at_ns = ? WHERE contract_id = ? AND attempts < 5",
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
            "DELETE FROM pending_stripe_receipts WHERE contract_id = ?",
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

    /// Auto-accept a rental contract when provider has auto_accept_rentals enabled.
    ///
    /// This is called after payment succeeds. If the provider has auto_accept_rentals=true,
    /// the contract transitions from "requested" to "accepted" without manual provider approval.
    ///
    /// Returns Ok(true) if contract was auto-accepted, Ok(false) if not eligible.
    /// Idempotent: safe to call multiple times (returns Ok(false) if already accepted).
    pub async fn try_auto_accept_contract(&self, contract_id: &[u8]) -> Result<bool> {
        // Get contract
        let contract = self
            .get_contract(contract_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        // Only auto-accept contracts in "requested" status
        if contract.status.to_lowercase() != "requested" {
            return Ok(false);
        }

        // Only auto-accept if payment succeeded
        if contract.payment_status.to_lowercase() != "succeeded" {
            return Ok(false);
        }

        // Check if provider has auto_accept_rentals enabled
        let provider_pubkey = hex::decode(&contract.provider_pubkey)
            .map_err(|_| anyhow::anyhow!("Invalid provider pubkey hex"))?;

        let auto_accept = self
            .get_provider_auto_accept_rentals(&provider_pubkey)
            .await?;

        if !auto_accept {
            return Ok(false);
        }

        // Auto-accept the contract
        let updated_at_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let new_status = "accepted";
        let change_memo = "Auto-accepted (provider has auto_accept_rentals enabled)";

        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = ?, status_updated_at_ns = ?, status_updated_by = ? WHERE contract_id = ?",
            new_status,
            updated_at_ns,
            provider_pubkey,
            contract_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES (?, ?, ?, ?, ?, ?)",
            contract_id,
            contract.status,
            new_status,
            provider_pubkey,
            updated_at_ns,
            change_memo
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        tracing::info!(
            "Auto-accepted contract {} for provider {}",
            hex::encode(contract_id),
            contract.provider_pubkey
        );

        Ok(true)
    }

    // ==================== Provisioning Locks ====================

    /// Acquire a provisioning lock on a contract.
    /// Returns Ok(true) if lock acquired, Ok(false) if already locked by another agent.
    /// Fails if contract not found or not in lockable status.
    ///
    /// Lock duration is typically 5 minutes; expired locks can be cleared by background job.
    pub async fn acquire_provisioning_lock(
        &self,
        contract_id: &[u8],
        agent_pubkey: &[u8],
        lock_duration_ns: i64,
    ) -> Result<bool> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let expires_ns = now_ns + lock_duration_ns;

        // Atomically try to acquire lock:
        // - Only lock if not already locked by another agent
        // - Allow re-locking by same agent (idempotent)
        // - Only lock contracts in accepted/provisioning status with succeeded payment
        let result = sqlx::query!(
            r#"UPDATE contract_sign_requests
               SET provisioning_lock_agent = ?,
                   provisioning_lock_at_ns = ?,
                   provisioning_lock_expires_ns = ?
               WHERE contract_id = ?
                 AND status IN ('accepted', 'provisioning')
                 AND payment_status = 'succeeded'
                 AND (provisioning_lock_agent IS NULL
                      OR provisioning_lock_agent = ?
                      OR provisioning_lock_expires_ns < ?)"#,
            agent_pubkey,
            now_ns,
            expires_ns,
            contract_id,
            agent_pubkey,
            now_ns
        )
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            Ok(true)
        } else {
            // Check if contract exists and why we couldn't lock
            let contract = self.get_contract(contract_id).await?;
            match contract {
                None => Err(anyhow::anyhow!(
                    "Contract not found: {}",
                    hex::encode(contract_id)
                )),
                Some(c) if c.status != "accepted" && c.status != "provisioning" => {
                    Err(anyhow::anyhow!(
                        "Contract {} is not in lockable status (status: {})",
                        hex::encode(contract_id),
                        c.status
                    ))
                }
                Some(c) if c.payment_status != "succeeded" => Err(anyhow::anyhow!(
                    "Contract {} payment not succeeded (status: {})",
                    hex::encode(contract_id),
                    c.payment_status
                )),
                _ => Ok(false), // Already locked by another agent
            }
        }
    }

    /// Release a provisioning lock held by the specified agent.
    /// Returns Ok(true) if lock was released, Ok(false) if agent didn't hold the lock.
    pub async fn release_provisioning_lock(
        &self,
        contract_id: &[u8],
        agent_pubkey: &[u8],
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"UPDATE contract_sign_requests
               SET provisioning_lock_agent = NULL,
                   provisioning_lock_at_ns = NULL,
                   provisioning_lock_expires_ns = NULL
               WHERE contract_id = ?
                 AND provisioning_lock_agent = ?"#,
            contract_id,
            agent_pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Clear expired provisioning locks.
    /// Should be called by a background job periodically.
    /// Returns the number of locks cleared.
    pub async fn clear_expired_provisioning_locks(&self) -> Result<u64> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let result = sqlx::query!(
            r#"UPDATE contract_sign_requests
               SET provisioning_lock_agent = NULL,
                   provisioning_lock_at_ns = NULL,
                   provisioning_lock_expires_ns = NULL
               WHERE provisioning_lock_expires_ns IS NOT NULL
                 AND provisioning_lock_expires_ns < ?
                 AND status IN ('accepted', 'provisioning')"#,
            now_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get pending contracts filtered by agent's pool.
    /// Returns contracts that:
    /// - Match the agent's pool (explicit pool_id match) OR
    /// - Match by location (offering datacenter_country maps to pool location)
    /// - Are not locked by another agent (or lock is expired)
    /// - Have status 'accepted' or 'provisioning' with payment succeeded
    ///
    /// Pool ID and location are now required parameters.
    pub async fn get_pending_provision_contracts_for_pool(
        &self,
        provider_pubkey: &[u8],
        pool_id: Option<&str>,
        pool_location: Option<&str>,
    ) -> Result<Vec<ContractWithSpecs>> {
        use crate::database::agent_pools::country_to_region;

        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let pool_id = pool_id.ok_or_else(|| anyhow::anyhow!("pool_id is required"))?;
        let pool_location = pool_location.unwrap_or("default");

        // Internal struct to include country for location matching
        #[derive(sqlx::FromRow)]
        struct ContractWithCountry {
            contract_id: String,
            offering_id: String,
            requester_ssh_pubkey: String,
            instance_config: Option<String>,
            cpu_cores: Option<i64>,
            memory_amount: Option<String>,
            storage_capacity: Option<String>,
            provisioner_type: Option<String>,
            provisioner_config: Option<String>,
            agent_pool_id: Option<String>,
            datacenter_country: Option<String>,
        }

        // Single query that fetches all candidates:
        // - Explicit pool match (agent_pool_id = pool_id)
        // - Location-matchable (agent_pool_id IS NULL with datacenter_country)
        let candidates = sqlx::query_as::<_, ContractWithCountry>(
            r#"SELECT
               lower(hex(c.contract_id)) as contract_id,
               c.offering_id,
               c.requester_ssh_pubkey,
               c.instance_config,
               o.processor_cores as cpu_cores,
               o.memory_amount,
               o.total_ssd_capacity as storage_capacity,
               o.provisioner_type,
               o.provisioner_config,
               o.agent_pool_id,
               o.datacenter_country
               FROM contract_sign_requests c
               LEFT JOIN provider_offerings o ON c.offering_id = o.offering_id AND c.provider_pubkey = o.pubkey
               WHERE c.provider_pubkey = ?
               AND c.status IN ('accepted', 'provisioning')
               AND c.payment_status = 'succeeded'
               AND (c.provisioning_lock_agent IS NULL OR c.provisioning_lock_expires_ns < ?)
               AND (o.agent_pool_id = ? OR o.agent_pool_id IS NULL)
               ORDER BY c.created_at_ns ASC"#,
        )
        .bind(provider_pubkey)
        .bind(now_ns)
        .bind(pool_id)
        .fetch_all(&self.pool)
        .await?;

        // Filter: explicit pool match OR location auto-match
        let contracts: Vec<ContractWithSpecs> = candidates
            .into_iter()
            .filter(|c| {
                // Explicit pool match
                if c.agent_pool_id.as_deref() == Some(pool_id) {
                    return true;
                }
                // Location auto-match: no explicit pool, country maps to pool location
                if c.agent_pool_id.is_none() {
                    if let Some(country) = &c.datacenter_country {
                        return country_to_region(country) == Some(pool_location);
                    }
                }
                false
            })
            .map(|c| ContractWithSpecs {
                contract_id: c.contract_id,
                offering_id: c.offering_id,
                requester_ssh_pubkey: c.requester_ssh_pubkey,
                instance_config: c.instance_config,
                cpu_cores: c.cpu_cores,
                memory_amount: c.memory_amount,
                storage_capacity: c.storage_capacity,
                provisioner_type: c.provisioner_type,
                provisioner_config: c.provisioner_config,
            })
            .collect();

        Ok(contracts)
    }

    // === Subscription Management ===

    /// Get contract by Stripe subscription ID
    pub async fn get_contract_by_subscription_id(
        &self,
        subscription_id: &str,
    ) -> Result<Option<Contract>> {
        let contract = sqlx::query_as!(
            Contract,
            r#"SELECT lower(hex(contract_id)) as "contract_id!: String", lower(hex(requester_pubkey)) as "requester_pubkey!: String", requester_ssh_pubkey as "requester_ssh_pubkey!", requester_contact as "requester_contact!", lower(hex(provider_pubkey)) as "provider_pubkey!: String",
               offering_id as "offering_id!", region_name, instance_config, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns,
               duration_hours, original_duration_hours, request_memo as "request_memo!", created_at_ns, status as "status!",
               provisioning_instance_details, provisioning_completed_at_ns, payment_method as "payment_method!", stripe_payment_intent_id, stripe_customer_id, icpay_transaction_id, payment_status as "payment_status!",
               currency as "currency!", refund_amount_e9s, stripe_refund_id, refund_created_at_ns, status_updated_at_ns, icpay_payment_id, icpay_refund_id, total_released_e9s, last_release_at_ns,
               tax_amount_e9s, tax_rate_percent, tax_type, tax_jurisdiction, customer_tax_id, reverse_charge, buyer_address, stripe_invoice_id, receipt_number, receipt_sent_at_ns,
               stripe_subscription_id, subscription_status, current_period_end_ns, COALESCE(cancel_at_period_end, 0) as "cancel_at_period_end!: bool"
               FROM contract_sign_requests WHERE stripe_subscription_id = ?"#,
            subscription_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(contract)
    }

    /// Update subscription status from Stripe webhooks
    pub async fn update_contract_subscription(
        &self,
        stripe_subscription_id: &str,
        status: &str,
        current_period_end_ns: i64,
        cancel_at_period_end: bool,
    ) -> Result<()> {
        let cancel_flag = if cancel_at_period_end { 1i32 } else { 0i32 };
        sqlx::query!(
            "UPDATE contract_sign_requests SET subscription_status = ?, current_period_end_ns = ?, cancel_at_period_end = ? WHERE stripe_subscription_id = ?",
            status,
            current_period_end_ns,
            cancel_flag,
            stripe_subscription_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // === Contract Usage Tracking ===

    /// Create a new billing period for a contract
    pub async fn create_contract_usage(
        &self,
        contract_id: &[u8],
        billing_period_start: i64,
        billing_period_end: i64,
        units_included: Option<f64>,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"INSERT INTO contract_usage (contract_id, billing_period_start, billing_period_end, units_included)
               VALUES (?, ?, ?, ?)
               RETURNING id as "id!: i64""#,
            contract_id,
            billing_period_start,
            billing_period_end,
            units_included
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.id)
    }

    /// Record a usage event for a contract
    pub async fn record_usage_event(
        &self,
        contract_id: &[u8],
        event_type: &str,
        units_delta: Option<f64>,
        heartbeat_at: Option<i64>,
        source: Option<&str>,
        metadata: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query!(
            r#"INSERT INTO contract_usage_events (contract_id, event_type, units_delta, heartbeat_at, source, metadata)
               VALUES (?, ?, ?, ?, ?, ?)
               RETURNING id as "id!: i64""#,
            contract_id,
            event_type,
            units_delta,
            heartbeat_at,
            source,
            metadata
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.id)
    }

    /// Get current billing period usage for a contract
    pub async fn get_current_usage(&self, contract_id: &[u8]) -> Result<Option<ContractUsage>> {
        let now = chrono::Utc::now().timestamp();
        let usage = sqlx::query_as!(
            ContractUsage,
            r#"SELECT
                id as "id!: i64",
                lower(hex(contract_id)) as "contract_id!: String",
                billing_period_start as "billing_period_start!: i64",
                billing_period_end as "billing_period_end!: i64",
                units_used as "units_used!: f64",
                units_included,
                overage_units as "overage_units!: f64",
                estimated_charge_cents,
                reported_to_stripe as "reported_to_stripe!: bool",
                stripe_usage_record_id,
                created_at as "created_at!: i64",
                updated_at as "updated_at!: i64"
            FROM contract_usage
            WHERE contract_id = ? AND billing_period_start <= ? AND billing_period_end > ?
            ORDER BY billing_period_start DESC
            LIMIT 1"#,
            contract_id,
            now,
            now
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(usage)
    }

    /// Update usage from heartbeat events for a contract
    /// Calculates units_used based on heartbeat intervals
    pub async fn update_usage_from_heartbeats(
        &self,
        contract_id: &[u8],
        usage_id: i64,
        billing_unit: &str,
    ) -> Result<f64> {
        // Get the billing period
        let usage = sqlx::query!(
            r#"SELECT billing_period_start as "billing_period_start!: i64",
                      billing_period_end as "billing_period_end!: i64",
                      units_included
               FROM contract_usage WHERE id = ?"#,
            usage_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Get heartbeat events in this billing period
        let heartbeats = sqlx::query!(
            r#"SELECT heartbeat_at as "heartbeat_at!: i64"
               FROM contract_usage_events
               WHERE contract_id = ?
                 AND event_type = 'heartbeat'
                 AND heartbeat_at >= ?
                 AND heartbeat_at < ?
               ORDER BY heartbeat_at ASC"#,
            contract_id,
            usage.billing_period_start,
            usage.billing_period_end
        )
        .fetch_all(&self.pool)
        .await?;

        // Calculate total units based on billing_unit
        let units_per_second = match billing_unit {
            "minute" => 1.0 / 60.0,
            "hour" => 1.0 / 3600.0,
            "day" => 1.0 / 86400.0,
            "month" => 1.0 / (30.0 * 86400.0),
            _ => 1.0 / 3600.0, // Default to hourly
        };

        let mut total_units = 0.0;
        let mut prev_ts: Option<i64> = None;

        for hb in &heartbeats {
            if let Some(prev) = prev_ts {
                let interval_seconds = (hb.heartbeat_at - prev) as f64;
                // Cap interval at 10 minutes (600 seconds) - if no heartbeat for longer, assume offline
                let capped_interval = interval_seconds.min(600.0);
                total_units += capped_interval * units_per_second;
            }
            prev_ts = Some(hb.heartbeat_at);
        }

        // Calculate overage
        let overage = if let Some(included) = usage.units_included {
            (total_units - included).max(0.0)
        } else {
            0.0
        };

        // Update the usage record
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        sqlx::query!(
            "UPDATE contract_usage SET units_used = ?, overage_units = ?, updated_at = ? WHERE id = ?",
            total_units,
            overage,
            now_ns,
            usage_id
        )
        .execute(&self.pool)
        .await?;

        Ok(total_units)
    }

    /// Mark usage as reported to Stripe
    pub async fn mark_usage_reported(
        &self,
        usage_id: i64,
        stripe_usage_record_id: &str,
    ) -> Result<()> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        sqlx::query!(
            "UPDATE contract_usage SET reported_to_stripe = 1, stripe_usage_record_id = ?, updated_at = ? WHERE id = ?",
            stripe_usage_record_id,
            now_ns,
            usage_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get unreported usage records that are past their billing period end
    pub async fn get_unreported_usage(&self) -> Result<Vec<ContractUsage>> {
        let now = chrono::Utc::now().timestamp();
        let usage = sqlx::query_as!(
            ContractUsage,
            r#"SELECT
                id as "id!: i64",
                lower(hex(contract_id)) as "contract_id!: String",
                billing_period_start as "billing_period_start!: i64",
                billing_period_end as "billing_period_end!: i64",
                units_used as "units_used!: f64",
                units_included,
                overage_units as "overage_units!: f64",
                estimated_charge_cents,
                reported_to_stripe as "reported_to_stripe!: bool",
                stripe_usage_record_id,
                created_at as "created_at!: i64",
                updated_at as "updated_at!: i64"
            FROM contract_usage
            WHERE reported_to_stripe = 0 AND billing_period_end <= ?
            ORDER BY billing_period_end ASC"#,
            now
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(usage)
    }
}

/// Contract usage tracking for billing periods
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(skip_serializing_if_is_none)]
pub struct ContractUsage {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "string")]
    pub contract_id: String,
    #[ts(type = "number")]
    pub billing_period_start: i64,
    #[ts(type = "number")]
    pub billing_period_end: i64,
    #[ts(type = "number")]
    pub units_used: f64,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub units_included: Option<f64>,
    #[ts(type = "number")]
    pub overage_units: f64,
    #[ts(type = "number | undefined")]
    #[oai(skip_serializing_if_is_none)]
    pub estimated_charge_cents: Option<i64>,
    pub reported_to_stripe: bool,
    #[oai(skip_serializing_if_is_none)]
    pub stripe_usage_record_id: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

/// Pending Stripe receipt for background processing
#[derive(Debug)]
#[allow(dead_code)]
pub struct PendingStripeReceipt {
    pub contract_id: Vec<u8>,
    pub created_at_ns: i64,
    pub next_attempt_at_ns: i64,
    pub attempts: i64,
}

#[cfg(test)]
mod tests;
