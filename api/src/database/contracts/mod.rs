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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub offering_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub region_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub instance_config: Option<String>,
    #[ts(type = "number")]
    pub payment_amount_e9s: i64,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub start_timestamp_ns: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub end_timestamp_ns: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub duration_hours: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub original_duration_hours: Option<i64>,
    pub request_memo: String,
    #[ts(type = "number")]
    pub created_at_ns: i64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub provisioning_instance_details: Option<String>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub provisioning_completed_at_ns: Option<i64>,
    pub payment_method: String,
    /// Stripe Checkout Session ID (cs_*) - captured at checkout completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub stripe_checkout_session_id: Option<String>,
    /// Stripe PaymentIntent ID (pi_*) - read from session.payment_intent at checkout completion.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub stripe_payment_intent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub stripe_customer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub icpay_transaction_id: Option<String>,
    pub payment_status: String,
    pub currency: String,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub refund_amount_e9s: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub stripe_refund_id: Option<String>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub refund_created_at_ns: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub status_updated_at_ns: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub icpay_payment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub icpay_refund_id: Option<String>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub total_released_e9s: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub last_release_at_ns: Option<i64>,
    // Tax tracking (from Stripe Tax or manual entry)
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub tax_amount_e9s: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub tax_rate_percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub tax_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub tax_jurisdiction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub customer_tax_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub reverse_charge: Option<bool>,
    /// Buyer address for B2B invoices
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub buyer_address: Option<String>,
    /// Stripe invoice ID for invoice PDF retrieval
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub stripe_invoice_id: Option<String>,
    /// Receipt tracking
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub receipt_number: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub receipt_sent_at_ns: Option<i64>,
    // Subscription tracking (for recurring billing)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub stripe_subscription_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub subscription_status: Option<String>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub current_period_end_ns: Option<i64>,
    #[ts(type = "boolean")]
    #[sqlx(default)]
    pub cancel_at_period_end: bool,
    /// Tenant opt-in for automatic renewal when contract is about to expire
    #[ts(type = "boolean")]
    #[sqlx(default)]
    pub auto_renew: bool,
    // Gateway configuration (DC-level reverse proxy)
    /// Gateway slug (6-char alphanumeric) for subdomain routing
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub gateway_slug: Option<String>,
    /// Full gateway subdomain (e.g., "k7m2p4.a3x9f2b1.dev-gw.decent-cloud.org")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub gateway_subdomain: Option<String>,
    /// SSH port accessible via gateway (0-65535)
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub gateway_ssh_port: Option<i32>,
    /// Start of allocated port range (0-65535)
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub gateway_port_range_start: Option<i32>,
    /// End of allocated port range (0-65535)
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub gateway_port_range_end: Option<i32>,
    /// Timestamp (ns) when user requested a password reset; cleared by agent after completion.
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub password_reset_requested_at_ns: Option<i64>,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub ssh_key_rotation_requested_at_ns: Option<i64>,
    /// Selected operating system for the rented VM (e.g., "Ubuntu 22.04")
    #[ts(type = "string | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub operating_system: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub ssh_pubkey: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub contact_method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub request_memo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub duration_hours: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub payment_method: Option<String>,
    /// Buyer address for B2B invoices (street, city, postal code, country)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub buyer_address: Option<String>,
    /// Selected operating system for the rented VM (e.g., "Ubuntu 22.04")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub operating_system: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub instance_config: Option<String>,
    /// CPU cores from offering (processor_cores)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub cpu_cores: Option<i64>,
    /// Memory amount from offering (e.g. "16 GB")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub memory_amount: Option<String>,
    /// Storage capacity from offering (e.g. "100 GB")
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub storage_capacity: Option<String>,
    /// Provisioner type from offering (e.g. "proxmox", "script", "manual")
    /// NULL = use agent's default provisioner
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub provisioner_type: Option<String>,
    /// Provisioner config JSON from offering
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub provisioner_config: Option<String>,
    /// Script to execute via SSH after VM provisioning (uses shebang for interpreter)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub post_provision_script: Option<String>,
}

/// Contract pending termination for dc-agent
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
#[serde(rename_all = "camelCase")]
pub struct ContractPendingTermination {
    pub contract_id: String,
    /// Instance details JSON (contains external_id needed for termination)
    pub instance_details: String,
}

/// Contract pending SSH key rotation for dc-agent.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, Object)]
#[serde(rename_all = "camelCase")]
pub struct ContractPendingSshKeyRotation {
    pub contract_id: String,
    pub requester_ssh_pubkey: String,
}

impl Database {
    /// Get contracts for a user (as requester)
    pub async fn get_user_contracts(&self, pubkey: &[u8]) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(encode(c.contract_id, 'hex')) as "contract_id!: String", lower(encode(c.requester_pubkey, 'hex')) as "requester_pubkey!: String", c.requester_ssh_pubkey as "requester_ssh_pubkey!", c.requester_contact as "requester_contact!", lower(encode(c.provider_pubkey, 'hex')) as "provider_pubkey!: String",
               c.offering_id as "offering_id!", o.offer_name as "offering_name?", c.region_name, c.instance_config, c.payment_amount_e9s, c.start_timestamp_ns, c.end_timestamp_ns,
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
               LEFT JOIN provider_offerings o ON o.offering_id = c.offering_id AND o.pubkey = c.provider_pubkey
               WHERE c.requester_pubkey = $1 ORDER BY c.created_at_ns DESC"#,
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
            r#"SELECT lower(encode(c.contract_id, 'hex')) as "contract_id!: String", lower(encode(c.requester_pubkey, 'hex')) as "requester_pubkey!: String", c.requester_ssh_pubkey as "requester_ssh_pubkey!", c.requester_contact as "requester_contact!", lower(encode(c.provider_pubkey, 'hex')) as "provider_pubkey!: String",
               c.offering_id as "offering_id!", o.offer_name as "offering_name?", c.region_name, c.instance_config, c.payment_amount_e9s, c.start_timestamp_ns, c.end_timestamp_ns,
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
               LEFT JOIN provider_offerings o ON o.offering_id = c.offering_id AND o.pubkey = c.provider_pubkey
               WHERE c.provider_pubkey = $1 ORDER BY c.created_at_ns DESC"#,
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
            r#"SELECT lower(encode(c.contract_id, 'hex')) as "contract_id!: String", lower(encode(c.requester_pubkey, 'hex')) as "requester_pubkey!: String", c.requester_ssh_pubkey as "requester_ssh_pubkey!", c.requester_contact as "requester_contact!", lower(encode(c.provider_pubkey, 'hex')) as "provider_pubkey!: String",
               c.offering_id as "offering_id!", o.offer_name as "offering_name?", c.region_name, c.instance_config, c.payment_amount_e9s, c.start_timestamp_ns, c.end_timestamp_ns,
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
               LEFT JOIN provider_offerings o ON o.offering_id = c.offering_id AND o.pubkey = c.provider_pubkey
               WHERE c.provider_pubkey = $1 AND c.status IN ('requested', 'pending') ORDER BY c.created_at_ns DESC"#,
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
               lower(encode(contract_id, 'hex')) as "contract_id!: String",
               provisioning_instance_details as "instance_details!: String"
               FROM contract_sign_requests
               WHERE provider_pubkey = $1
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
        let terminated_at_ns = crate::now_ns()?;

        let result = sqlx::query!(
            "UPDATE contract_sign_requests SET terminated_at_ns = $1 WHERE contract_id = $2 AND status = 'cancelled'",
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
            r#"SELECT lower(encode(c.contract_id, 'hex')) as "contract_id!: String", lower(encode(c.requester_pubkey, 'hex')) as "requester_pubkey!: String", c.requester_ssh_pubkey as "requester_ssh_pubkey!", c.requester_contact as "requester_contact!", lower(encode(c.provider_pubkey, 'hex')) as "provider_pubkey!: String",
               c.offering_id as "offering_id!", o.offer_name as "offering_name?", c.region_name, c.instance_config, c.payment_amount_e9s, c.start_timestamp_ns, c.end_timestamp_ns,
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
               LEFT JOIN provider_offerings o ON o.offering_id = c.offering_id AND o.pubkey = c.provider_pubkey
               WHERE c.contract_id = $1"#,
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(contract)
    }

    /// Get all contracts with pagination
    pub async fn list_contracts(&self, limit: i64, offset: i64) -> Result<Vec<Contract>> {
        let contracts = sqlx::query_as!(
            Contract,
            r#"SELECT lower(encode(c.contract_id, 'hex')) as "contract_id!: String", lower(encode(c.requester_pubkey, 'hex')) as "requester_pubkey!: String", c.requester_ssh_pubkey as "requester_ssh_pubkey!", c.requester_contact as "requester_contact!", lower(encode(c.provider_pubkey, 'hex')) as "provider_pubkey!: String",
               c.offering_id as "offering_id!", o.offer_name as "offering_name?", c.region_name, c.instance_config, c.payment_amount_e9s, c.start_timestamp_ns, c.end_timestamp_ns,
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
               LEFT JOIN provider_offerings o ON o.offering_id = c.offering_id AND o.pubkey = c.provider_pubkey
               ORDER BY c.created_at_ns DESC LIMIT $1 OFFSET $2"#,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(contracts)
    }
}

mod dispute;
mod extensions;
mod payment;
mod provisioning;
mod rental;
mod timeouts;
mod usage;

pub use dispute::{dispute_refund_idempotency_key, ContractDisputeUpsert};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub units_included: Option<f64>,
    #[ts(type = "number")]
    pub overage_units: f64,
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub estimated_charge_cents: Option<i64>,
    pub reported_to_stripe: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub stripe_usage_record_id: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    /// Billing unit from the offering (minute, hour, day, month)
    pub billing_unit: String,
}

/// Pending Stripe receipt for background processing
#[derive(Debug)]
pub struct PendingStripeReceipt {
    pub contract_id: Vec<u8>,
    pub attempts: i64,
}

/// Provider health summary with uptime metrics
#[derive(Debug, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct ProviderHealthSummary {
    /// Total number of health checks in the period
    #[ts(type = "number")]
    pub total_checks: i64,
    /// Number of healthy checks
    #[ts(type = "number")]
    pub healthy_checks: i64,
    /// Number of unhealthy checks
    #[ts(type = "number")]
    pub unhealthy_checks: i64,
    /// Number of unknown status checks
    #[ts(type = "number")]
    pub unknown_checks: i64,
    /// Uptime percentage (0.0 - 100.0)
    pub uptime_percent: f64,
    /// Average latency in milliseconds (None if no latency data)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub avg_latency_ms: Option<f64>,
    /// Number of contracts with health data in the period
    #[ts(type = "number")]
    pub contracts_monitored: i64,
    /// Start of the measurement period (nanoseconds since epoch)
    #[ts(type = "number")]
    pub period_start_ns: i64,
    /// End of the measurement period (nanoseconds since epoch)
    #[ts(type = "number")]
    pub period_end_ns: i64,
}

/// Per-contract health summary with uptime metrics (all-time)
#[derive(Debug, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase")]
pub struct ContractHealthSummary {
    /// Total number of health checks recorded
    #[ts(type = "number")]
    pub total_checks: i64,
    /// Number of healthy checks
    #[ts(type = "number")]
    pub healthy_checks: i64,
    /// Number of unhealthy checks
    #[ts(type = "number")]
    pub unhealthy_checks: i64,
    /// Number of unknown status checks
    #[ts(type = "number")]
    pub unknown_checks: i64,
    /// Uptime percentage (0.0 - 100.0)
    pub uptime_percent: f64,
    /// Average latency in milliseconds (None if no latency data)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub avg_latency_ms: Option<f64>,
    /// Timestamp of the most recent check (nanoseconds since epoch), None if no checks
    #[ts(type = "number | null")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub last_checked_at: Option<i64>,
}

/// Health check result for a contract
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase", skip_serializing_if_is_none)]
pub struct ContractHealthCheck {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "string")]
    pub contract_id: String,
    /// Timestamp when the check was performed (nanoseconds since epoch)
    #[ts(type = "number")]
    pub checked_at: i64,
    /// Health status: "healthy", "unhealthy", or "unknown"
    pub status: String,
    /// Optional latency measurement in milliseconds
    #[ts(type = "number | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub latency_ms: Option<i32>,
    /// Optional JSON with additional diagnostic details
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub details: Option<String>,
    /// Timestamp when this record was created (nanoseconds since epoch)
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Timeline event for a contract (status changes, payment, provisioning, extensions)
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[serde(rename_all = "camelCase")]
#[oai(rename_all = "camelCase", skip_serializing_if_is_none)]
pub struct ContractEvent {
    #[ts(type = "number")]
    pub id: i64,
    /// Contract ID as hex string
    #[ts(type = "string")]
    pub contract_id: String,
    /// Event type: 'status_change', 'payment_confirmed', 'provisioned', 'extension', 'password_reset', 'note'
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub old_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub new_status: Option<String>,
    /// Who caused the event: 'tenant', 'provider', 'system'
    pub actor: String,
    /// Optional JSON or plain text details
    #[serde(skip_serializing_if = "Option::is_none")]
    #[oai(skip_serializing_if_is_none)]
    pub details: Option<String>,
    /// Nanoseconds since epoch
    #[ts(type = "number")]
    pub created_at: i64,
}

#[cfg(test)]
mod tests;
