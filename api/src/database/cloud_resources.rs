//! Cloud resources database operations.
//!
//! Handles self-provisioned resource management.

use super::types::Database;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CloudResource {
    pub id: String,
    pub cloud_account_id: String,
    pub external_id: String,
    pub name: String,
    pub server_type: String,
    pub location: String,
    pub image: String,
    pub ssh_pubkey: String,
    pub status: String,
    pub public_ip: Option<String>,
    pub ssh_port: i32,
    pub ssh_username: String,
    pub external_ssh_key_id: Option<String>,
    pub gateway_slug: Option<String>,
    pub gateway_subdomain: Option<String>,
    pub gateway_ssh_port: Option<i32>,
    pub gateway_port_range_start: Option<i32>,
    pub gateway_port_range_end: Option<i32>,
    pub offering_id: Option<i64>,
    pub listing_mode: String,
    pub error_message: Option<String>,
    /// Platform fee for this resource in e9s (1/1e9 USD). Always 0 for self-provisioned resources —
    /// users pay the cloud provider directly.
    #[ts(type = "number")]
    pub platform_fee_e9s: i64,
    pub created_at: String,
    pub updated_at: String,
    pub terminated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreateCloudResourceInput {
    pub cloud_account_id: String,
    pub name: String,
    pub server_type: String,
    pub location: String,
    pub image: String,
    pub ssh_pubkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CloudResourceWithDetails {
    #[serde(flatten)]
    #[oai(flatten)]
    pub resource: CloudResource,
    pub cloud_account_name: String,
    pub cloud_account_backend: String,
}

#[derive(Debug, FromRow)]
struct CloudResourceRow {
    id: Uuid,
    cloud_account_id: Uuid,
    external_id: String,
    name: String,
    server_type: String,
    location: String,
    image: String,
    ssh_pubkey: String,
    status: String,
    public_ip: Option<String>,
    ssh_port: i32,
    ssh_username: String,
    external_ssh_key_id: Option<String>,
    gateway_slug: Option<String>,
    gateway_subdomain: Option<String>,
    gateway_ssh_port: Option<i32>,
    gateway_port_range_start: Option<i32>,
    gateway_port_range_end: Option<i32>,
    offering_id: Option<i64>,
    listing_mode: String,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    terminated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct CloudResourceWithAccountRow {
    #[sqlx(flatten)]
    resource: CloudResourceRow,
    cloud_account_name: String,
    cloud_account_backend: String,
}

impl From<CloudResourceRow> for CloudResource {
    fn from(row: CloudResourceRow) -> Self {
        CloudResource {
            id: row.id.to_string(),
            cloud_account_id: row.cloud_account_id.to_string(),
            external_id: row.external_id,
            name: row.name,
            server_type: row.server_type,
            location: row.location,
            image: row.image,
            ssh_pubkey: row.ssh_pubkey,
            status: row.status,
            public_ip: row.public_ip,
            ssh_port: row.ssh_port,
            ssh_username: row.ssh_username,
            external_ssh_key_id: row.external_ssh_key_id,
            gateway_slug: row.gateway_slug,
            gateway_subdomain: row.gateway_subdomain,
            gateway_ssh_port: row.gateway_ssh_port,
            gateway_port_range_start: row.gateway_port_range_start,
            gateway_port_range_end: row.gateway_port_range_end,
            offering_id: row.offering_id,
            listing_mode: row.listing_mode,
            error_message: row.error_message,
            platform_fee_e9s: 0,
            created_at: row.created_at.to_rfc3339(),
            updated_at: row.updated_at.to_rfc3339(),
            terminated_at: row.terminated_at.map(|t| t.to_rfc3339()),
        }
    }
}

impl From<CloudResourceWithAccountRow> for CloudResourceWithDetails {
    fn from(row: CloudResourceWithAccountRow) -> Self {
        CloudResourceWithDetails {
            resource: CloudResource::from(row.resource),
            cloud_account_name: row.cloud_account_name,
            cloud_account_backend: row.cloud_account_backend,
        }
    }
}

impl Database {
    pub async fn list_cloud_resources(
        &self,
        account_id: &[u8],
    ) -> Result<Vec<CloudResourceWithDetails>> {
        let rows: Vec<CloudResourceWithAccountRow> = sqlx::query_as(
            r#"
            SELECT 
                cr.id, cr.cloud_account_id, cr.external_id, cr.name, 
                cr.server_type, cr.location, cr.image, cr.ssh_pubkey, cr.status,
                cr.public_ip, cr.ssh_port, cr.ssh_username, cr.external_ssh_key_id,
                cr.gateway_slug, cr.gateway_subdomain, cr.gateway_ssh_port, cr.gateway_port_range_start, cr.gateway_port_range_end,
                cr.offering_id, cr.listing_mode, cr.error_message,
                cr.created_at, cr.updated_at, cr.terminated_at,
                ca.name as cloud_account_name, ca.backend_type as cloud_account_backend
            FROM cloud_resources cr
            JOIN cloud_accounts ca ON cr.cloud_account_id = ca.id
            WHERE ca.account_id = $1 AND cr.terminated_at IS NULL
            ORDER BY cr.created_at DESC
            "#
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(CloudResourceWithDetails::from)
            .collect())
    }

    pub async fn get_cloud_resource(
        &self,
        id: &Uuid,
        account_id: &[u8],
    ) -> Result<Option<CloudResourceWithDetails>> {
        let row: Option<CloudResourceWithAccountRow> = sqlx::query_as(
            r#"
            SELECT 
                cr.id, cr.cloud_account_id, cr.external_id, cr.name, 
                cr.server_type, cr.location, cr.image, cr.ssh_pubkey, cr.status,
                cr.public_ip, cr.ssh_port, cr.ssh_username, cr.external_ssh_key_id,
                cr.gateway_slug, cr.gateway_subdomain, cr.gateway_ssh_port, cr.gateway_port_range_start, cr.gateway_port_range_end,
                cr.offering_id, cr.listing_mode, cr.error_message,
                cr.created_at, cr.updated_at, cr.terminated_at,
                ca.name as cloud_account_name, ca.backend_type as cloud_account_backend
            FROM cloud_resources cr
            JOIN cloud_accounts ca ON cr.cloud_account_id = ca.id
            WHERE cr.id = $1 AND ca.account_id = $2 AND cr.terminated_at IS NULL
            "#
        )
        .bind(id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(CloudResourceWithDetails::from))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_cloud_resource(
        &self,
        cloud_account_id: &Uuid,
        external_id: &str,
        name: &str,
        server_type: &str,
        location: &str,
        image: &str,
        ssh_pubkey: &str,
    ) -> Result<CloudResource> {
        let row: CloudResourceRow = sqlx::query_as(
            r#"
            INSERT INTO cloud_resources (
                cloud_account_id, external_id, name, server_type, location, image, ssh_pubkey,
                status, ssh_port, ssh_username, listing_mode
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'provisioning', 22, 'root', 'personal')
            RETURNING
                id, cloud_account_id, external_id, name,
                server_type, location, image, ssh_pubkey, status,
                public_ip, ssh_port, ssh_username, external_ssh_key_id,
                gateway_slug, gateway_subdomain, gateway_ssh_port, gateway_port_range_start, gateway_port_range_end,
                offering_id, listing_mode, error_message,
                created_at, updated_at, terminated_at
            "#
        )
        .bind(cloud_account_id)
        .bind(external_id)
        .bind(name)
        .bind(server_type)
        .bind(location)
        .bind(image)
        .bind(ssh_pubkey)
        .fetch_one(&self.pool)
        .await?;

        Ok(CloudResource::from(row))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_cloud_resource_provisioned(
        &self,
        id: &Uuid,
        external_id: &str,
        public_ip: &str,
        external_ssh_key_id: &str,
        gateway_slug: &str,
        gateway_subdomain: Option<&str>,
        gateway_ssh_port: i32,
        gateway_port_range_start: i32,
        gateway_port_range_end: i32,
    ) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET status = 'running',
                external_id = $2,
                public_ip = $3,
                external_ssh_key_id = $4,
                gateway_slug = $5,
                gateway_subdomain = $6,
                gateway_ssh_port = $7,
                gateway_port_range_start = $8,
                gateway_port_range_end = $9,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(external_id)
        .bind(public_ip)
        .bind(external_ssh_key_id)
        .bind(gateway_slug)
        .bind(gateway_subdomain)
        .bind(gateway_ssh_port)
        .bind(gateway_port_range_start)
        .bind(gateway_port_range_end)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("Cloud resource not found"));
        }

        Ok(())
    }

    pub async fn update_cloud_resource_status(&self, id: &Uuid, status: &str) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("Cloud resource not found"));
        }

        Ok(())
    }

    /// Transition a cloud resource from one status to another atomically.
    /// Returns error if the resource is not in the expected `from_status`.
    pub async fn transition_cloud_resource_status(
        &self,
        id: &Uuid,
        account_id: &[u8],
        from_status: &str,
        to_status: &str,
    ) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources cr
            SET status = $3, updated_at = NOW()
            FROM cloud_accounts ca
            WHERE cr.id = $1
              AND cr.cloud_account_id = ca.id
              AND ca.account_id = $2
              AND cr.status = $4
              AND cr.terminated_at IS NULL
            "#,
        )
        .bind(id)
        .bind(account_id)
        .bind(to_status)
        .bind(from_status)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!(
                "Resource not found or not in '{from_status}' status"
            ));
        }

        Ok(())
    }

    /// Get the external_id and cloud account credentials for a resource, verifying ownership.
    /// Used by start/stop operations that need to call the cloud backend.
    pub async fn get_cloud_resource_action_context(
        &self,
        resource_id: &Uuid,
        account_id: &[u8],
    ) -> Result<Option<CloudResourceActionContext>> {
        let row = sqlx::query_as::<_, CloudResourceActionContext>(
            r#"
            SELECT
                cr.external_id,
                cr.status,
                ca.backend_type,
                ca.credentials_encrypted
            FROM cloud_resources cr
            JOIN cloud_accounts ca ON cr.cloud_account_id = ca.id
            WHERE cr.id = $1
              AND ca.account_id = $2
              AND cr.terminated_at IS NULL
            "#,
        )
        .bind(resource_id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    /// Mark a cloud resource as failed with an error message visible to the user.
    pub async fn mark_cloud_resource_failed(&self, id: &Uuid, error_message: &str) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET status = 'failed', error_message = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(error_message)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("Cloud resource not found"));
        }

        Ok(())
    }

    pub async fn acquire_cloud_resource_lock(&self, id: &Uuid, lock_holder: &str) -> Result<bool> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET provisioning_locked_at = NOW(),
                provisioning_locked_by = $2
            WHERE id = $1 
              AND (provisioning_locked_at IS NULL 
                   OR provisioning_locked_at < NOW() - INTERVAL '10 minutes')
            "#,
        )
        .bind(id)
        .bind(lock_holder)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn release_cloud_resource_lock(&self, id: &Uuid) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE cloud_resources
            SET provisioning_locked_at = NULL,
                provisioning_locked_by = NULL
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_cloud_resource(&self, id: &Uuid, account_id: &[u8]) -> Result<bool> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources cr
            SET status = 'deleting',
                updated_at = NOW()
            FROM cloud_accounts ca
            WHERE cr.cloud_account_id = ca.id
              AND cr.id = $1 
              AND ca.account_id = $2
              AND cr.terminated_at IS NULL
            "#,
        )
        .bind(id)
        .bind(account_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn mark_cloud_resource_terminated(&self, id: &Uuid) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET status = 'deleted',
                terminated_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("Cloud resource not found"));
        }

        Ok(())
    }

    pub async fn get_pending_termination_resources(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingTerminationResource>> {
        let rows = sqlx::query_as::<_, PendingTerminationResource>(
            r#"
            SELECT
                cr.id, cr.external_id, cr.external_ssh_key_id,
                ca.backend_type, ca.credentials_encrypted,
                cr.gateway_slug, cr.location
            FROM cloud_resources cr
            JOIN cloud_accounts ca ON cr.cloud_account_id = ca.id
            WHERE cr.status = 'deleting'
              AND ca.is_valid = true
              AND (cr.provisioning_locked_at IS NULL
                   OR cr.provisioning_locked_at < NOW() - INTERVAL '10 minutes')
            ORDER BY cr.updated_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Pending provisioning resource row with optional contract/script info.
    #[allow(clippy::type_complexity)]
    pub async fn get_pending_provisioning_resources(
        &self,
        limit: i64,
    ) -> Result<Vec<PendingProvisioningResource>> {
        let rows = sqlx::query_as::<_, PendingProvisioningResource>(
            r#"
            SELECT
                cr.id, cr.cloud_account_id, cr.external_id, cr.name,
                cr.server_type, cr.location, cr.image, cr.ssh_pubkey,
                ca.backend_type, ca.credentials_encrypted,
                cr.contract_id, cr.post_provision_script
            FROM cloud_resources cr
            JOIN cloud_accounts ca ON cr.cloud_account_id = ca.id
            WHERE cr.status = 'provisioning'
              AND ca.is_valid = true
              AND (cr.provisioning_locked_at IS NULL
                   OR cr.provisioning_locked_at < NOW() - INTERVAL '10 minutes')
            ORDER BY cr.created_at ASC
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    /// Create a cloud_resource linked to a marketplace contract.
    /// Called when a recipe contract with a Hetzner offering is auto-accepted.
    #[allow(clippy::too_many_arguments)]
    pub async fn create_cloud_resource_for_contract(
        &self,
        contract_id: &[u8],
        cloud_account_id: &Uuid,
        name: &str,
        server_type: &str,
        location: &str,
        image: &str,
        ssh_pubkey: &str,
        post_provision_script: Option<&str>,
    ) -> Result<Uuid> {
        let pending_external_id = format!("pending-{}", Uuid::new_v4());
        let row: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO cloud_resources (
                cloud_account_id, external_id, name, server_type, location, image, ssh_pubkey,
                status, ssh_port, ssh_username, listing_mode, contract_id, post_provision_script
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'provisioning', 22, 'root', 'personal', $8, $9)
            RETURNING id
            "#,
        )
        .bind(cloud_account_id)
        .bind(&pending_external_id)
        .bind(name)
        .bind(server_type)
        .bind(location)
        .bind(image)
        .bind(ssh_pubkey)
        .bind(contract_id)
        .bind(post_provision_script)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(
            resource_id = %row.0,
            contract_id = %hex::encode(contract_id),
            "Created cloud_resource for contract"
        );

        Ok(row.0)
    }

    /// Mark the cloud_resource linked to a contract for deletion.
    /// The existing termination loop picks it up and deletes the VM.
    pub async fn mark_contract_resource_for_deletion(&self, contract_id: &[u8]) -> Result<bool> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET status = 'deleting', updated_at = NOW()
            WHERE contract_id = $1
              AND status NOT IN ('deleting', 'deleted', 'failed')
              AND terminated_at IS NULL
            "#,
        )
        .bind(contract_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected > 0 {
            tracing::info!(
                contract_id = %hex::encode(contract_id),
                "Marked cloud_resource for deletion (contract cancelled/expired)"
            );
        }

        Ok(rows_affected > 0)
    }

    /// Find and expire active cloud contracts past their end_timestamp_ns.
    /// For each: marks contract as 'expired' and its cloud_resource as 'deleting'.
    /// Returns the number of contracts expired.
    pub async fn expire_and_cleanup_cloud_contracts(&self) -> Result<u64> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // Find active contracts with cloud resources that are past expiration
        let expired_contracts: Vec<(Vec<u8>,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT csr.contract_id
            FROM contract_sign_requests csr
            JOIN cloud_resources cr ON cr.contract_id = csr.contract_id
            WHERE csr.status = 'active'
              AND csr.end_timestamp_ns > 0
              AND csr.end_timestamp_ns < $1
              AND cr.status NOT IN ('deleting', 'deleted', 'failed')
              AND cr.terminated_at IS NULL
            "#,
        )
        .bind(now_ns)
        .fetch_all(&self.pool)
        .await?;

        if expired_contracts.is_empty() {
            return Ok(0);
        }

        let mut count = 0u64;
        let expired_status = dcc_common::ContractStatus::Expired.to_string();

        for (contract_id,) in &expired_contracts {
            let mut tx = self.pool.begin().await?;

            // Update contract status to expired
            sqlx::query(
                r#"UPDATE contract_sign_requests
                   SET status = $1, status_updated_at_ns = $2
                   WHERE contract_id = $3 AND status = 'active'"#,
            )
            .bind(&expired_status)
            .bind(now_ns)
            .bind(contract_id)
            .execute(&mut *tx)
            .await?;

            // Record status history
            let system_actor: &[u8] = b"system";
            sqlx::query(
                "INSERT INTO contract_status_history (contract_id, old_status, new_status, changed_by, changed_at_ns, change_memo) VALUES ($1, 'active', $2, $3, $4, 'Contract expired (end_timestamp_ns reached)')",
            )
            .bind(contract_id)
            .bind(&expired_status)
            .bind(system_actor)
            .bind(now_ns)
            .execute(&mut *tx)
            .await?;

            // Mark cloud resource for deletion
            sqlx::query(
                r#"UPDATE cloud_resources
                   SET status = 'deleting', updated_at = NOW()
                   WHERE contract_id = $1
                     AND status NOT IN ('deleting', 'deleted', 'failed')
                     AND terminated_at IS NULL"#,
            )
            .bind(contract_id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            tracing::info!(
                contract_id = %hex::encode(contract_id),
                "Expired cloud contract and marked resource for deletion"
            );
            count += 1;
        }

        Ok(count)
    }

    /// Find the Hetzner cloud_account for a provider pubkey.
    /// Looks up: provider_pubkey → account_id → cloud_accounts (backend_type='hetzner').
    pub async fn find_hetzner_cloud_account_for_provider(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Option<Uuid>> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT ca.id
            FROM cloud_accounts ca
            JOIN account_public_keys apk ON ca.account_id = apk.account_id
            WHERE apk.public_key = $1
              AND apk.is_active = TRUE
              AND ca.backend_type = 'hetzner'
              AND ca.is_valid = TRUE
            ORDER BY ca.created_at ASC
            LIMIT 1
            "#,
        )
        .bind(provider_pubkey)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| r.0))
    }
}

/// Row returned by `get_pending_termination_resources`.
#[derive(Debug, FromRow)]
pub struct PendingTerminationResource {
    pub id: Uuid,
    pub external_id: String,
    pub external_ssh_key_id: Option<String>,
    pub backend_type: String,
    pub credentials_encrypted: String,
    pub gateway_slug: Option<String>,
    pub location: String,
}

/// Row returned by `get_pending_provisioning_resources`.
#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct PendingProvisioningResource {
    pub id: Uuid,
    pub cloud_account_id: Uuid,
    pub external_id: String,
    pub name: String,
    pub server_type: String,
    pub location: String,
    pub image: String,
    pub ssh_pubkey: String,
    pub backend_type: String,
    pub credentials_encrypted: String,
    pub contract_id: Option<Vec<u8>>,
    pub post_provision_script: Option<String>,
}

/// Context needed for start/stop operations on a cloud resource.
#[derive(Debug, FromRow)]
pub struct CloudResourceActionContext {
    pub external_id: String,
    pub status: String,
    pub backend_type: String,
    pub credentials_encrypted: String,
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_create_cloud_resource_for_contract_sets_fields() {
        let db = setup_test_db().await;

        // Create account + cloud_account
        let account = db
            .create_account("hetzner_test", &[1u8; 32], "test@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "test-hetzner",
                "encrypted-token-data",
                None,
            )
            .await
            .unwrap();

        let contract_id = vec![0xABu8; 32];

        // Insert a dummy contract_sign_requests row for the FK
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns, duration_hours, original_duration_hours, request_memo, created_at_ns, status, payment_method, payment_status, currency) VALUES ($1, $2, '', '', $3, 'test', 0, 0, 0, 1, 1, '', 0, 'accepted', 'stripe', 'succeeded', 'USD')"
        )
        .bind(&contract_id)
        .bind(&[2u8; 32][..])
        .bind(&[1u8; 32][..])
        .execute(&db.pool)
        .await
        .unwrap();

        let cloud_account_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();
        let resource_id = db
            .create_cloud_resource_for_contract(
                &contract_id,
                &cloud_account_uuid,
                "dc-recipe-test",
                "cx22",
                "fsn1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
                Some("#!/bin/bash\necho hello"),
            )
            .await
            .unwrap();

        // Verify the resource was created with correct fields
        let resource = db
            .get_cloud_resource(&resource_id, &account.id)
            .await
            .unwrap()
            .expect("Resource should exist");

        assert_eq!(resource.resource.name, "dc-recipe-test");
        assert_eq!(resource.resource.server_type, "cx22");
        assert_eq!(resource.resource.location, "fsn1");
        assert_eq!(resource.resource.image, "ubuntu-24.04");
        assert_eq!(resource.resource.ssh_pubkey, "ssh-ed25519 AAAA test");
        assert_eq!(resource.resource.status, "provisioning");
    }

    #[tokio::test]
    async fn test_mark_contract_resource_for_deletion() {
        let db = setup_test_db().await;

        let account = db
            .create_account("del_test", &[3u8; 32], "del@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "del-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();

        let contract_id = vec![0xCDu8; 32];

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns, duration_hours, original_duration_hours, request_memo, created_at_ns, status, payment_method, payment_status, currency) VALUES ($1, $2, '', '', $3, 'test', 0, 0, 0, 1, 1, '', 0, 'active', 'stripe', 'succeeded', 'USD')"
        )
        .bind(&contract_id)
        .bind(&[4u8; 32][..])
        .bind(&[3u8; 32][..])
        .execute(&db.pool)
        .await
        .unwrap();

        let cloud_account_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();
        let resource_id = db
            .create_cloud_resource_for_contract(
                &contract_id,
                &cloud_account_uuid,
                "dc-recipe-del",
                "cx22",
                "fsn1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
                None,
            )
            .await
            .unwrap();

        // Mark for deletion
        let marked = db
            .mark_contract_resource_for_deletion(&contract_id)
            .await
            .unwrap();
        assert!(marked);

        // Verify status changed to 'deleting'
        let resource = db
            .get_cloud_resource(&resource_id, &account.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resource.resource.status, "deleting");

        // Marking again should return false (already deleting)
        let marked_again = db
            .mark_contract_resource_for_deletion(&contract_id)
            .await
            .unwrap();
        assert!(!marked_again);
    }

    #[tokio::test]
    async fn test_mark_contract_resource_for_deletion_no_resource() {
        let db = setup_test_db().await;

        // Non-existent contract_id - should return false, not error
        let marked = db
            .mark_contract_resource_for_deletion(&[0xEEu8; 32])
            .await
            .unwrap();
        assert!(!marked);
    }

    #[tokio::test]
    async fn test_find_hetzner_cloud_account_for_provider() {
        let db = setup_test_db().await;

        let pubkey = [5u8; 32];
        let account = db
            .create_account("prov_test", &pubkey, "prov@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "prov-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();

        let found = db
            .find_hetzner_cloud_account_for_provider(&pubkey)
            .await
            .unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().to_string(), cloud_account.id);

        // Non-existent provider should return None
        let not_found = db
            .find_hetzner_cloud_account_for_provider(&[99u8; 32])
            .await
            .unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_pending_provisioning_includes_contract_fields() {
        let db = setup_test_db().await;

        let account = db
            .create_account("pending_test", &[6u8; 32], "pending@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "pending-hetzner",
                "encrypted-token",
                None,
            )
            .await
            .unwrap();

        let contract_id = vec![0xBBu8; 32];
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns, duration_hours, original_duration_hours, request_memo, created_at_ns, status, payment_method, payment_status, currency) VALUES ($1, $2, '', '', $3, 'test', 0, 0, 0, 1, 1, '', 0, 'accepted', 'stripe', 'succeeded', 'USD')"
        )
        .bind(&contract_id)
        .bind(&[7u8; 32][..])
        .bind(&[6u8; 32][..])
        .execute(&db.pool)
        .await
        .unwrap();

        let cloud_account_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();
        db.create_cloud_resource_for_contract(
            &contract_id,
            &cloud_account_uuid,
            "dc-recipe-pending",
            "cx22",
            "fsn1",
            "ubuntu-24.04",
            "ssh-ed25519 AAAA test",
            Some("#!/bin/bash\necho setup"),
        )
        .await
        .unwrap();

        let pending = db.get_pending_provisioning_resources(10).await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].contract_id, Some(contract_id));
        assert_eq!(
            pending[0].post_provision_script.as_deref(),
            Some("#!/bin/bash\necho setup")
        );
        assert_eq!(pending[0].name, "dc-recipe-pending");
    }

    #[tokio::test]
    async fn test_expire_and_cleanup_cloud_contracts() {
        let db = setup_test_db().await;

        let account = db
            .create_account("expire_test", &[10u8; 32], "expire@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "expire-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();

        let contract_id = vec![0xDDu8; 32];
        let past_end_ns = 1_000_000i64; // far in the past

        // Insert an active contract with end_timestamp_ns in the past
        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns, duration_hours, original_duration_hours, request_memo, created_at_ns, status, payment_method, payment_status, currency) VALUES ($1, $2, '', '', $3, 'test', 0, 1, $4, 1, 1, '', 0, 'active', 'stripe', 'succeeded', 'USD')"
        )
        .bind(&contract_id)
        .bind(&[11u8; 32][..])
        .bind(&[10u8; 32][..])
        .bind(past_end_ns)
        .execute(&db.pool)
        .await
        .unwrap();

        // Create a running cloud resource linked to the contract
        let cloud_account_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();
        let resource_id = db
            .create_cloud_resource_for_contract(
                &contract_id,
                &cloud_account_uuid,
                "dc-expire-test",
                "cx22",
                "nbg1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
                None,
            )
            .await
            .unwrap();

        // Mark resource as running (simulate successful provision)
        db.update_cloud_resource_provisioned(
            &resource_id,
            "ext-123",
            "1.2.3.4",
            "key-1",
            "abc123",
            Some("abc123.hz-nbg1.dev-gw.decent-cloud.org"),
            22,
            22,
            22,
        )
        .await
        .unwrap();

        // Run expiration
        let count = db.expire_and_cleanup_cloud_contracts().await.unwrap();
        assert_eq!(count, 1, "Should expire exactly one contract");

        // Verify contract status changed to expired
        let status: (String,) =
            sqlx::query_as("SELECT status FROM contract_sign_requests WHERE contract_id = $1")
                .bind(&contract_id)
                .fetch_one(&db.pool)
                .await
                .unwrap();
        assert_eq!(status.0, "expired");

        // Verify cloud resource is now deleting
        let resource = db
            .get_cloud_resource(&resource_id, &account.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(resource.resource.status, "deleting");

        // Running again should find nothing
        let count2 = db.expire_and_cleanup_cloud_contracts().await.unwrap();
        assert_eq!(count2, 0, "No contracts to expire on second run");
    }

    #[tokio::test]
    async fn test_expire_skips_future_contracts() {
        let db = setup_test_db().await;

        let account = db
            .create_account("future_test", &[12u8; 32], "future@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "future-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();

        let contract_id = vec![0xEEu8; 32];
        let future_end_ns = i64::MAX; // far in the future

        sqlx::query(
            "INSERT INTO contract_sign_requests (contract_id, requester_pubkey, requester_ssh_pubkey, requester_contact, provider_pubkey, offering_id, payment_amount_e9s, start_timestamp_ns, end_timestamp_ns, duration_hours, original_duration_hours, request_memo, created_at_ns, status, payment_method, payment_status, currency) VALUES ($1, $2, '', '', $3, 'test', 0, 1, $4, 1, 1, '', 0, 'active', 'stripe', 'succeeded', 'USD')"
        )
        .bind(&contract_id)
        .bind(&[13u8; 32][..])
        .bind(&[12u8; 32][..])
        .bind(future_end_ns)
        .execute(&db.pool)
        .await
        .unwrap();

        let cloud_account_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();
        db.create_cloud_resource_for_contract(
            &contract_id,
            &cloud_account_uuid,
            "dc-future-test",
            "cx22",
            "nbg1",
            "ubuntu-24.04",
            "ssh-ed25519 AAAA test",
            None,
        )
        .await
        .unwrap();

        let count = db.expire_and_cleanup_cloud_contracts().await.unwrap();
        assert_eq!(count, 0, "Should not expire future contracts");
    }

    #[tokio::test]
    async fn test_transition_cloud_resource_status_running_to_stopped() {
        let db = setup_test_db().await;

        let pubkey = [30u8; 32];
        let account = db
            .create_account("transition_test", &pubkey, "trans@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "trans-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();
        let ca_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();

        let resource = db
            .create_cloud_resource(
                &ca_uuid,
                "ext-1",
                "trans-vm",
                "cx22",
                "nbg1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
            )
            .await
            .unwrap();
        let resource_id: uuid::Uuid = resource.id.parse().unwrap();

        // Move to running first
        db.update_cloud_resource_status(&resource_id, "running")
            .await
            .unwrap();

        // Transition running -> stopped
        db.transition_cloud_resource_status(&resource_id, &account.id, "running", "stopped")
            .await
            .unwrap();

        let updated = db
            .get_cloud_resource(&resource_id, &account.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.resource.status, "stopped");
    }

    #[tokio::test]
    async fn test_transition_rejects_wrong_current_status() {
        let db = setup_test_db().await;

        let pubkey = [31u8; 32];
        let account = db
            .create_account("reject_test", &pubkey, "reject@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "reject-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();
        let ca_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();

        let resource = db
            .create_cloud_resource(
                &ca_uuid,
                "ext-2",
                "reject-vm",
                "cx22",
                "nbg1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
            )
            .await
            .unwrap();
        let resource_id: uuid::Uuid = resource.id.parse().unwrap();

        // Resource is in 'provisioning' status, try to stop it — should fail
        let err = db
            .transition_cloud_resource_status(&resource_id, &account.id, "running", "stopped")
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("not in 'running' status"),
            "Expected status mismatch error, got: {err}"
        );
    }

    #[tokio::test]
    async fn test_transition_rejects_wrong_owner() {
        let db = setup_test_db().await;

        let pubkey = [32u8; 32];
        let account = db
            .create_account("owner_test", &pubkey, "owner@example.com")
            .await
            .unwrap();
        let other_pubkey = [33u8; 32];
        let other_account = db
            .create_account("other_test", &other_pubkey, "other@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "owner-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();
        let ca_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();

        let resource = db
            .create_cloud_resource(
                &ca_uuid,
                "ext-3",
                "owner-vm",
                "cx22",
                "nbg1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
            )
            .await
            .unwrap();
        let resource_id: uuid::Uuid = resource.id.parse().unwrap();

        db.update_cloud_resource_status(&resource_id, "running")
            .await
            .unwrap();

        // Other user tries to stop it — should fail
        let err = db
            .transition_cloud_resource_status(&resource_id, &other_account.id, "running", "stopped")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not in 'running' status"));
    }

    #[tokio::test]
    async fn test_get_cloud_resource_action_context() {
        let db = setup_test_db().await;

        let pubkey = [34u8; 32];
        let account = db
            .create_account("ctx_test", &pubkey, "ctx@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                crate::cloud::types::BackendType::Hetzner,
                "ctx-hetzner",
                "encrypted-token-xyz",
                None,
            )
            .await
            .unwrap();
        let ca_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();

        let resource = db
            .create_cloud_resource(
                &ca_uuid,
                "hetzner-12345",
                "ctx-vm",
                "cx22",
                "nbg1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
            )
            .await
            .unwrap();
        let resource_id: uuid::Uuid = resource.id.parse().unwrap();

        db.update_cloud_resource_status(&resource_id, "running")
            .await
            .unwrap();

        // Get action context as owner
        let ctx = db
            .get_cloud_resource_action_context(&resource_id, &account.id)
            .await
            .unwrap()
            .expect("Should find resource");

        assert_eq!(ctx.external_id, "hetzner-12345");
        assert_eq!(ctx.status, "running");
        assert_eq!(ctx.backend_type, "hetzner");
        assert_eq!(ctx.credentials_encrypted, "encrypted-token-xyz");

        // Non-owner should get None
        let other = db
            .get_cloud_resource_action_context(&resource_id, &[99u8; 32])
            .await
            .unwrap();
        assert!(other.is_none());
    }
}
