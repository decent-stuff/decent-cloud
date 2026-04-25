use super::*;
use crate::database::types::Database;
use anyhow::{Context, Result};
use dcc_common::ContractStatus;

impl Database {
    /// Add provisioning details to a contract
    /// Credentials expiration: 7 days after provisioning
    const CREDENTIALS_EXPIRATION_DAYS: i64 = 7;

    pub async fn add_provisioning_details(
        &self,
        contract_id: &[u8],
        instance_details: &str,
    ) -> Result<()> {
        let provisioned_at_ns = crate::now_ns()?;

        // Extract gateway fields and root_password from instance JSON
        #[derive(serde::Deserialize)]
        struct InstanceFields {
            gateway_slug: Option<String>,
            gateway_subdomain: Option<String>,
            gateway_ssh_port: Option<u16>,
            gateway_port_range_start: Option<u16>,
            gateway_port_range_end: Option<u16>,
            root_password: Option<String>,
        }

        let instance =
            serde_json::from_str::<InstanceFields>(instance_details).unwrap_or(InstanceFields {
                gateway_slug: None,
                gateway_subdomain: None,
                gateway_ssh_port: None,
                gateway_port_range_start: None,
                gateway_port_range_end: None,
                root_password: None,
            });

        let gateway_ssh_port = instance.gateway_ssh_port.map(|p| p as i32);
        let gateway_port_range_start = instance.gateway_port_range_start.map(|p| p as i32);
        let gateway_port_range_end = instance.gateway_port_range_end.map(|p| p as i32);

        // If root_password is present, encrypt it for the requester
        let (encrypted_credentials, credentials_expires_at_ns) =
            if let Some(ref password) = instance.root_password {
                // Get requester's pubkey from the contract
                let requester_pubkey: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT requester_pubkey FROM contract_sign_requests WHERE contract_id = $1",
                )
                .bind(contract_id)
                .fetch_optional(&self.pool)
                .await?;

                match requester_pubkey {
                    Some(pubkey) if pubkey.len() == 32 => {
                        match crate::crypto::encrypt_credentials_with_aad(
                            password,
                            &pubkey,
                            contract_id,
                        ) {
                            Ok(encrypted) => {
                                let expires_at = provisioned_at_ns
                                    + (Self::CREDENTIALS_EXPIRATION_DAYS
                                        * 24
                                        * 60
                                        * 60
                                        * 1_000_000_000);
                                (Some(encrypted.to_json()), Some(expires_at))
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to encrypt credentials for contract {}: {:#?}",
                                    hex::encode(contract_id),
                                    e
                                );
                                (None, None)
                            }
                        }
                    }
                    Some(pubkey) => {
                        tracing::warn!(
                            "Invalid requester pubkey length for contract {}: {} bytes",
                            hex::encode(contract_id),
                            pubkey.len()
                        );
                        (None, None)
                    }
                    None => {
                        tracing::warn!(
                            "No requester pubkey found for contract {}",
                            hex::encode(contract_id)
                        );
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };

        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            r#"UPDATE contract_sign_requests
               SET provisioning_instance_details = $1,
                   provisioning_completed_at_ns = $2,
                   gateway_slug = $3,
                   gateway_subdomain = $4,
                   gateway_ssh_port = $5,
                   gateway_port_range_start = $6,
                   gateway_port_range_end = $7
               WHERE contract_id = $8"#,
            instance_details,
            provisioned_at_ns,
            instance.gateway_slug,
            instance.gateway_subdomain,
            gateway_ssh_port,
            gateway_port_range_start,
            gateway_port_range_end,
            contract_id
        )
        .execute(&mut *tx)
        .await?;

        let empty_instance_ip: Option<&str> = None;
        sqlx::query(
            r#"INSERT INTO contract_provisioning_details (contract_id, instance_ip, instance_credentials, connection_instructions, provisioned_at_ns, credentials_expires_at_ns)
               VALUES ($1, $2, $3, $4, $5, $6)
               ON CONFLICT(contract_id) DO UPDATE SET
                   instance_ip = excluded.instance_ip,
                   instance_credentials = excluded.instance_credentials,
                   connection_instructions = excluded.connection_instructions,
                   provisioned_at_ns = excluded.provisioned_at_ns,
                   credentials_expires_at_ns = excluded.credentials_expires_at_ns"#,
        )
        .bind(contract_id)
        .bind(empty_instance_ip)
        .bind(&encrypted_credentials)
        .bind(instance_details)
        .bind(provisioned_at_ns)
        .bind(credentials_expires_at_ns)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.insert_contract_event(contract_id, "provisioned", None, None, "system", None)
            .await?;

        Ok(())
    }

    pub async fn try_activate_self_provisioned_contract(&self, contract_id: &[u8]) -> Result<bool> {
        let contract = self
            .get_contract(contract_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        if contract.status.to_lowercase() != "accepted" {
            return Ok(false);
        }

        let resource = match self
            .get_reserved_self_provisioned_resource(contract_id)
            .await?
        {
            Some(resource) => resource,
            None => return Ok(false),
        };

        let instance_details = serde_json::json!({
            "public_ip": resource.public_ip,
            "ssh_port": resource.ssh_port,
            "ssh_username": resource.ssh_username,
            "gateway_slug": resource.gateway_slug,
            "gateway_subdomain": resource.gateway_subdomain,
            "gateway_ssh_port": resource.gateway_ssh_port,
            "gateway_port_range_start": resource.gateway_port_range_start,
            "gateway_port_range_end": resource.gateway_port_range_end,
        })
        .to_string();

        self.update_contract_provisioned_by_cloud_resource(
            contract_id,
            &instance_details,
            resource.gateway_slug.as_deref(),
            resource.gateway_subdomain.as_deref(),
            resource.gateway_ssh_port,
        )
        .await?;

        Ok(true)
    }

    /// Delete expired credentials (should be called periodically)
    pub async fn cleanup_expired_credentials(&self) -> Result<i64> {
        let now_ns = crate::now_ns()?;

        let result = sqlx::query(
            r#"UPDATE contract_provisioning_details
               SET instance_credentials = NULL, credentials_expires_at_ns = NULL
               WHERE credentials_expires_at_ns IS NOT NULL AND credentials_expires_at_ns < $1"#,
        )
        .bind(now_ns)
        .execute(&self.pool)
        .await?;

        let deleted = result.rows_affected() as i64;
        if deleted > 0 {
            tracing::info!("Cleaned up {} expired credential(s)", deleted);
        }

        Ok(deleted)
    }

    /// Purge terminal contracts older than `retention_days`.
    ///
    /// Deletes contracts in terminal states (rejected, cancelled, expired) whose
    /// `status_updated_at_ns` is older than the retention period, along with all
    /// related data across 15+ tables.
    ///
    /// Safety: skips contracts with active cloud resources or unreported billing usage.
    pub async fn purge_terminal_contracts(&self, retention_days: i64) -> Result<u64> {
        let cutoff_ns = crate::now_ns()? - (retention_days * 24 * 60 * 60 * 1_000_000_000);

        let terminal_statuses = [
            ContractStatus::Rejected.to_string(),
            ContractStatus::Cancelled.to_string(),
            ContractStatus::Expired.to_string(),
        ];

        // Find purgeable contract IDs
        let purgeable: Vec<(Vec<u8>,)> = sqlx::query_as(
            r#"
            SELECT csr.contract_id
            FROM contract_sign_requests csr
            WHERE csr.status = ANY($1)
              AND csr.status_updated_at_ns IS NOT NULL
              AND csr.status_updated_at_ns < $2
              -- Skip contracts with active cloud resources
              AND NOT EXISTS (
                  SELECT 1 FROM cloud_resources cr
                  WHERE cr.contract_id = csr.contract_id
                    AND cr.status NOT IN ('deleted', 'failed')
              )
              -- Skip contracts with unreported usage (billing must complete first)
              AND NOT EXISTS (
                  SELECT 1 FROM contract_usage cu
                  WHERE cu.contract_id = csr.contract_id
                    AND cu.reported_to_stripe = FALSE
              )
            "#,
        )
        .bind(&terminal_statuses[..])
        .bind(cutoff_ns)
        .fetch_all(&self.pool)
        .await?;

        if purgeable.is_empty() {
            return Ok(0);
        }

        let contract_ids: Vec<Vec<u8>> = purgeable.into_iter().map(|(id,)| id).collect();
        let count = contract_ids.len() as u64;

        // Hex-encoded IDs for TEXT-typed contract_id columns
        let hex_ids: Vec<String> = contract_ids.iter().map(hex::encode).collect();

        let mut tx = self.pool.begin().await?;

        // Delete from non-cascading tables (order matters: child tables first)
        // tax_tracking references invoices.id, must go before invoices
        sqlx::query(
            "DELETE FROM tax_tracking WHERE invoice_id IN (SELECT id FROM invoices WHERE contract_id = ANY($1))",
        )
        .bind(&contract_ids)
        .execute(&mut *tx)
        .await
        .context("purge: tax_tracking")?;

        // Non-cascading BYTEA contract_id tables
        let bytea_tables = [
            "contract_events",
            "contract_usage_events",
            "contract_usage",
            "contract_health_checks",
            "invoices",
            "cloud_resources",
            "escrow",
            "reseller_commissions",
            "reseller_orders",
            "receipt_tracking",
            "pending_stripe_receipts",
        ];
        for table in bytea_tables {
            sqlx::query(&format!(
                "DELETE FROM {} WHERE contract_id = ANY($1)",
                table
            ))
            .bind(&contract_ids)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("purge: {}", table))?;
        }

        // TEXT-typed contract_id tables (hex-encoded strings)
        let text_tables = ["chatwoot_message_events", "bandwidth_history"];
        for table in text_tables {
            sqlx::query(&format!(
                "DELETE FROM {} WHERE contract_id = ANY($1)",
                table
            ))
            .bind(&hex_ids)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("purge: {}", table))?;
        }

        // Delete from main table (cascades: contract_provisioning_details,
        // contract_status_history, contract_payment_entries, contract_sign_replies,
        // contract_extensions, payment_releases, contract_feedback)
        sqlx::query("DELETE FROM contract_sign_requests WHERE contract_id = ANY($1)")
            .bind(&contract_ids)
            .execute(&mut *tx)
            .await
            .context("purge: contract_sign_requests")?;

        tx.commit().await?;
        Ok(count)
    }

    /// Get encrypted credentials for a contract (only returns if not expired)
    pub async fn get_encrypted_credentials(&self, contract_id: &[u8]) -> Result<Option<String>> {
        let now_ns = crate::now_ns()?;

        let credentials: Option<String> = sqlx::query_scalar(
            r#"SELECT instance_credentials FROM contract_provisioning_details
               WHERE contract_id = $1
               AND instance_credentials IS NOT NULL
               AND (credentials_expires_at_ns IS NULL OR credentials_expires_at_ns > $2)"#,
        )
        .bind(contract_id)
        .bind(now_ns)
        .fetch_optional(&self.pool)
        .await?;

        Ok(credentials)
    }

    /// Update encrypted credentials for a contract (for password reset).
    /// Encrypts the new password with the requester's public key and updates the expiration.
    pub async fn update_encrypted_credentials(
        &self,
        contract_id: &[u8],
        new_password: &str,
    ) -> Result<()> {
        let now_ns = crate::now_ns()?;

        let requester_pubkey: Option<Vec<u8>> = sqlx::query_scalar(
            "SELECT requester_pubkey FROM contract_sign_requests WHERE contract_id = $1",
        )
        .bind(contract_id)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        let requester_pubkey = requester_pubkey
            .ok_or_else(|| anyhow::anyhow!("Contract not found or has no requester pubkey"))?;

        if requester_pubkey.len() != 32 {
            anyhow::bail!(
                "Invalid requester pubkey length: {} bytes",
                requester_pubkey.len()
            );
        }

        let encrypted = crate::crypto::encrypt_credentials_with_aad(
            new_password,
            &requester_pubkey,
            contract_id,
        )
        .context("Failed to encrypt credentials")?;

        let expires_at_ns =
            now_ns + (Self::CREDENTIALS_EXPIRATION_DAYS * 24 * 60 * 60 * 1_000_000_000);

        let result = sqlx::query(
            r#"UPDATE contract_provisioning_details
               SET instance_credentials = $1,
                   credentials_expires_at_ns = $2
               WHERE contract_id = $3"#,
        )
        .bind(encrypted.to_json())
        .bind(expires_at_ns)
        .bind(contract_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "No provisioning details found for contract {}",
                hex::encode(contract_id)
            );
        }

        Ok(())
    }

    /// Request a password reset for a contract.
    /// Sets password_reset_requested_at_ns to current time.
    pub async fn request_password_reset(&self, contract_id: &[u8]) -> Result<()> {
        let now_ns = crate::now_ns()?;

        let result = sqlx::query(
            r#"UPDATE contract_provisioning_details
               SET password_reset_requested_at_ns = $1
               WHERE contract_id = $2"#,
        )
        .bind(now_ns)
        .bind(contract_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "No provisioning details found for contract {}",
                hex::encode(contract_id)
            );
        }

        self.insert_contract_event(contract_id, "password_reset", None, None, "tenant", None)
            .await?;

        Ok(())
    }

    /// Clear password reset request after it's been handled.
    pub async fn clear_password_reset_request(&self, contract_id: &[u8]) -> Result<()> {
        sqlx::query(
            r#"UPDATE contract_provisioning_details
               SET password_reset_requested_at_ns = NULL
               WHERE contract_id = $1"#,
        )
        .bind(contract_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get contracts with pending password reset requests for a provider.
    /// Returns contracts where password_reset_requested_at_ns is set and contract is active.
    pub async fn get_pending_password_resets(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<Contract>> {
        let rows = sqlx::query_as::<_, Contract>(
            r#"SELECT c.* FROM contract_sign_requests c
               INNER JOIN contract_provisioning_details pd ON c.contract_id = pd.contract_id
               WHERE c.provider_pubkey = $1
               AND c.status IN ('provisioned', 'active')
               AND pd.password_reset_requested_at_ns IS NOT NULL
               ORDER BY pd.password_reset_requested_at_ns ASC"#,
        )
        .bind(provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    pub async fn request_ssh_key_rotation(
        &self,
        contract_id: &[u8],
        new_ssh_pubkey: &str,
    ) -> Result<()> {
        let now_ns = crate::now_ns()?;

        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            r#"UPDATE contract_provisioning_details
               SET pending_requester_ssh_pubkey = $1,
                   ssh_key_rotation_requested_at_ns = $2
               WHERE contract_id = $3"#,
        )
        .bind(new_ssh_pubkey)
        .bind(now_ns)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            anyhow::bail!(
                "No provisioning details found for contract {}",
                hex::encode(contract_id)
            );
        }

        tx.commit().await?;

        self.insert_contract_event(contract_id, "ssh_key_rotation", None, None, "tenant", None)
            .await?;

        Ok(())
    }

    pub async fn complete_ssh_key_rotation(&self, contract_id: &[u8]) -> Result<String> {
        let mut tx = self.pool.begin().await?;

        let pending_key: Option<String> = sqlx::query_scalar(
            r#"SELECT pending_requester_ssh_pubkey
               FROM contract_provisioning_details
               WHERE contract_id = $1"#,
        )
        .bind(contract_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();

        let pending_key = pending_key
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("No pending SSH key rotation found"))?;

        sqlx::query(
            r#"UPDATE contract_sign_requests
               SET requester_ssh_pubkey = $1
               WHERE contract_id = $2"#,
        )
        .bind(&pending_key)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"UPDATE contract_provisioning_details
               SET pending_requester_ssh_pubkey = NULL,
                   ssh_key_rotation_requested_at_ns = NULL
               WHERE contract_id = $1"#,
        )
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(pending_key)
    }

    pub async fn get_pending_ssh_key_rotations(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<ContractPendingSshKeyRotation>> {
        let rows = sqlx::query_as::<_, ContractPendingSshKeyRotation>(
            r#"SELECT lower(encode(c.contract_id, 'hex')) as contract_id,
                      pd.pending_requester_ssh_pubkey as requester_ssh_pubkey
               FROM contract_sign_requests c
               INNER JOIN contract_provisioning_details pd ON c.contract_id = pd.contract_id
               WHERE c.provider_pubkey = $1
               AND c.status IN ('provisioned', 'active')
               AND pd.pending_requester_ssh_pubkey IS NOT NULL
               AND pd.ssh_key_rotation_requested_at_ns IS NOT NULL
               ORDER BY pd.ssh_key_rotation_requested_at_ns ASC"#,
        )
        .bind(provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
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
        let current_status: ContractStatus = match contract.status.parse() {
            Ok(s) => s,
            Err(_) => return Ok(false), // Invalid status, skip auto-accept
        };
        if current_status != ContractStatus::Requested {
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

        // Check per-offering rules (if any exist for this offering)
        let rule_matches = self
            .check_auto_accept_rule_matches(
                &provider_pubkey,
                &contract.offering_id,
                contract.duration_hours,
            )
            .await?;

        if !rule_matches {
            tracing::debug!(
                "Contract {} not auto-accepted: offering {} duration {:?}h outside rule range",
                hex::encode(contract_id),
                contract.offering_id,
                contract.duration_hours,
            );
            return Ok(false);
        }

        // Auto-accept the contract
        let updated_at_ns = crate::now_ns()?;
        let new_status = ContractStatus::Accepted.to_string();
        let change_memo = "Auto-accepted (provider has auto_accept_rentals enabled)";

        let mut tx = self.pool.begin().await?;
        sqlx::query!(
            "UPDATE contract_sign_requests SET status = $1, status_updated_at_ns = $2, status_updated_by = $3 WHERE contract_id = $4",
            new_status,
            updated_at_ns,
            provider_pubkey,
            contract_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
            contract_id,
            contract.status,
            new_status,
            provider_pubkey,
            updated_at_ns,
            change_memo
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"INSERT INTO contract_events (contract_id, event_type, old_status, new_status, actor, details, created_at)
               VALUES ($1, 'status_change', $2, $3, 'system', $4, $5)"#,
            contract_id,
            contract.status,
            new_status,
            change_memo,
            updated_at_ns
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

    /// After auto-accept, check if the offering uses a cloud provisioner (Hetzner, Vultr)
    /// and create a cloud_resource linked to the contract. The provisioning service will pick it up.
    pub async fn try_trigger_cloud_provisioning(&self, contract_id: &[u8]) -> Result<bool> {
        let contract = self
            .get_contract(contract_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Contract not found"))?;

        if contract.status.to_lowercase() != "accepted" {
            return Ok(false);
        }

        let offering_db_id: i64 = contract.offering_id.parse().map_err(|_| {
            anyhow::anyhow!("Invalid offering_id in contract: {}", contract.offering_id)
        })?;

        let offering = self
            .get_offering(offering_db_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Offering {} not found", offering_db_id))?;

        let provisioner_type = match offering.provisioner_type.as_deref() {
            Some(t) => t,
            _ => return Ok(false),
        };

        let provider_pubkey = hex::decode(&contract.provider_pubkey)
            .map_err(|_| anyhow::anyhow!("Invalid provider pubkey hex"))?;

        let (cloud_account_id, server_type, location, image) = match provisioner_type {
            "hetzner" => {
                let cloud_account_id = self
                    .find_hetzner_cloud_account_for_provider(&provider_pubkey)
                    .await?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Provider {} has no valid Hetzner cloud account configured",
                            contract.provider_pubkey
                        )
                    })?;

                let resolved = crate::cloud::hetzner::resolve_provisioner_config(
                    offering.provisioner_config.as_deref(),
                    &offering.datacenter_city,
                    offering.template_name.as_deref(),
                )?;

                (
                    cloud_account_id,
                    resolved.server_type,
                    resolved.location,
                    resolved.image,
                )
            }
            "vultr" => {
                let cloud_account_id = self
                    .find_vultr_cloud_account_for_provider(&provider_pubkey)
                    .await?
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Provider {} has no valid Vultr cloud account configured",
                            contract.provider_pubkey
                        )
                    })?;

                let resolved = crate::cloud::vultr::resolve_provisioner_config(
                    offering.provisioner_config.as_deref(),
                    &offering.datacenter_city,
                    offering.template_name.as_deref(),
                )?;

                (
                    cloud_account_id,
                    resolved.plan,
                    resolved.region,
                    resolved.os_id.to_string(),
                )
            }
            _ => return Ok(false),
        };

        let name = format!("dc-recipe-{}", &hex::encode(contract_id)[..12]);

        self.create_cloud_resource_for_contract(
            contract_id,
            &cloud_account_id,
            &name,
            &server_type,
            &location,
            &image,
            &contract.requester_ssh_pubkey,
            offering.post_provision_script.as_deref(),
        )
        .await?;

        tracing::info!(
            contract_id = %hex::encode(contract_id),
            cloud_account_id = %cloud_account_id,
            provisioner_type = %provisioner_type,
            "Triggered {} provisioning for recipe contract", provisioner_type
        );

        Ok(true)
    }

    // ==================== Cloud Resource Provisioning Bridge ====================

    /// Update contract to active after cloud_resource provisioning completes.
    /// Called by the cloud provisioning service (system-level, no auth check).
    pub async fn update_contract_provisioned_by_cloud_resource(
        &self,
        contract_id: &[u8],
        instance_details: &str,
        gateway_slug: Option<&str>,
        gateway_subdomain: Option<&str>,
        gateway_ssh_port: Option<i32>,
    ) -> Result<()> {
        let now_ns = crate::now_ns()?;
        let new_status = ContractStatus::Active.to_string();

        let mut tx = self.pool.begin().await?;

        // Get current status for history
        let current_status: Option<(String,)> =
            sqlx::query_as("SELECT status FROM contract_sign_requests WHERE contract_id = $1")
                .bind(contract_id)
                .fetch_optional(&mut *tx)
                .await?;

        let old_status = current_status
            .map(|r| r.0)
            .ok_or_else(|| anyhow::anyhow!("Contract not found: {}", hex::encode(contract_id)))?;

        sqlx::query(
            r#"UPDATE contract_sign_requests
               SET status = $1,
                   status_updated_at_ns = $2,
                   provisioning_instance_details = $3,
                   provisioning_completed_at_ns = $4,
                   gateway_slug = $5,
                   gateway_subdomain = $6,
                   gateway_ssh_port = $7
               WHERE contract_id = $8"#,
        )
        .bind(&new_status)
        .bind(now_ns)
        .bind(instance_details)
        .bind(now_ns)
        .bind(gateway_slug)
        .bind(gateway_subdomain)
        .bind(gateway_ssh_port)
        .bind(contract_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO contract_provisioning_details (contract_id, instance_ip, instance_credentials, connection_instructions, provisioned_at_ns, credentials_expires_at_ns)
               VALUES ($1, NULL, NULL, $2, $3, NULL)
               ON CONFLICT(contract_id) DO UPDATE SET
                   connection_instructions = excluded.connection_instructions,
                   provisioned_at_ns = excluded.provisioned_at_ns"#,
        )
        .bind(contract_id)
        .bind(instance_details)
        .bind(now_ns)
        .execute(&mut *tx)
        .await?;

        let system_actor: &[u8] = b"system";
        sqlx::query(
            "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(contract_id)
        .bind(&old_status)
        .bind(&new_status)
        .bind(system_actor)
        .bind(now_ns)
        .bind("Cloud resource provisioned by api-server")
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        tracing::info!(
            contract_id = %hex::encode(contract_id),
            "Contract updated to active after cloud resource provisioning"
        );

        Ok(())
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
        let now_ns = crate::now_ns()?;
        let expires_ns = now_ns + lock_duration_ns;

        // Atomically try to acquire lock:
        // - Only lock if not already locked by another agent
        // - Allow re-locking by same agent (idempotent)
        // - Only lock contracts in accepted/provisioning status with succeeded payment
        let result = sqlx::query!(
            r#"UPDATE contract_sign_requests
               SET provisioning_lock_agent = $1,
                   provisioning_lock_at_ns = $2,
                   provisioning_lock_expires_ns = $3
               WHERE contract_id = $4
                 AND status IN ('accepted', 'provisioning')
                 AND payment_status = 'succeeded'
                 AND (provisioning_lock_agent IS NULL
                      OR provisioning_lock_agent = $5
                      OR provisioning_lock_expires_ns < $6)"#,
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
                Some(c) => {
                    let status: ContractStatus = c.status.parse().map_err(|_| {
                        anyhow::anyhow!(
                            "Contract {} has invalid status: {}",
                            hex::encode(contract_id),
                            c.status
                        )
                    })?;
                    // Can only lock contracts that are accepted or provisioning
                    if status != ContractStatus::Accepted && status != ContractStatus::Provisioning
                    {
                        return Err(anyhow::anyhow!(
                            "Contract {} is not in lockable status (status: {})",
                            hex::encode(contract_id),
                            c.status
                        ));
                    }
                    // Check payment status
                    if c.payment_status != "succeeded" {
                        return Err(anyhow::anyhow!(
                            "Contract {} payment not succeeded (status: {})",
                            hex::encode(contract_id),
                            c.payment_status
                        ));
                    }
                    Ok(false) // Already locked by another agent
                }
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
               WHERE contract_id = $1
                 AND provisioning_lock_agent = $2"#,
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
        let now_ns = crate::now_ns()?;

        let result = sqlx::query!(
            r#"UPDATE contract_sign_requests
               SET provisioning_lock_agent = NULL,
                   provisioning_lock_at_ns = NULL,
                   provisioning_lock_expires_ns = NULL
               WHERE provisioning_lock_expires_ns IS NOT NULL
                 AND provisioning_lock_expires_ns < $1
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

        let now_ns = crate::now_ns()?;

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
            post_provision_script: Option<String>,
            agent_pool_id: Option<String>,
            datacenter_country: Option<String>,
        }

        // Single query that fetches all candidates:
        // - Explicit pool match (agent_pool_id = pool_id)
        // - Location-matchable (agent_pool_id IS NULL with datacenter_country)
        let candidates = sqlx::query_as::<_, ContractWithCountry>(
            r#"SELECT
               lower(encode(c.contract_id, 'hex')) as contract_id,
               c.offering_id,
               c.requester_ssh_pubkey,
               c.instance_config,
               o.processor_cores as cpu_cores,
               o.memory_amount,
               o.total_ssd_capacity as storage_capacity,
               o.provisioner_type,
               o.provisioner_config,
               o.post_provision_script,
               o.agent_pool_id,
               o.datacenter_country
               FROM contract_sign_requests c
               LEFT JOIN provider_offerings o ON c.offering_id = o.offering_id AND c.provider_pubkey = o.pubkey
               WHERE c.provider_pubkey = $1
               AND c.status IN ('accepted', 'provisioning')
               AND c.payment_status = 'succeeded'
               AND (c.provisioning_lock_agent IS NULL OR c.provisioning_lock_expires_ns < $2)
               AND (o.agent_pool_id = $3 OR o.agent_pool_id IS NULL)
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
                post_provision_script: c.post_provision_script,
            })
            .collect();

        Ok(contracts)
    }
}
