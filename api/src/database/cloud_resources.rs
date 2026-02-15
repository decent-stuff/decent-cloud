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
    pub gateway_ssh_port: Option<i32>,
    pub gateway_port_range_start: Option<i32>,
    pub gateway_port_range_end: Option<i32>,
    pub offering_id: Option<i64>,
    pub listing_mode: String,
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
    gateway_ssh_port: Option<i32>,
    gateway_port_range_start: Option<i32>,
    gateway_port_range_end: Option<i32>,
    offering_id: Option<i64>,
    listing_mode: String,
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
            gateway_ssh_port: row.gateway_ssh_port,
            gateway_port_range_start: row.gateway_port_range_start,
            gateway_port_range_end: row.gateway_port_range_end,
            offering_id: row.offering_id,
            listing_mode: row.listing_mode,
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
    pub async fn list_cloud_resources(&self, account_id: &[u8]) -> Result<Vec<CloudResourceWithDetails>> {
        let rows: Vec<CloudResourceWithAccountRow> = sqlx::query_as(
            r#"
            SELECT 
                cr.id, cr.cloud_account_id, cr.external_id, cr.name, 
                cr.server_type, cr.location, cr.image, cr.ssh_pubkey, cr.status,
                cr.public_ip, cr.ssh_port, cr.ssh_username, cr.external_ssh_key_id,
                cr.gateway_slug, cr.gateway_ssh_port, cr.gateway_port_range_start, cr.gateway_port_range_end,
                cr.offering_id, cr.listing_mode,
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

        Ok(rows.into_iter().map(CloudResourceWithDetails::from).collect())
    }

    pub async fn get_cloud_resource(&self, id: &Uuid, account_id: &[u8]) -> Result<Option<CloudResourceWithDetails>> {
        let row: Option<CloudResourceWithAccountRow> = sqlx::query_as(
            r#"
            SELECT 
                cr.id, cr.cloud_account_id, cr.external_id, cr.name, 
                cr.server_type, cr.location, cr.image, cr.ssh_pubkey, cr.status,
                cr.public_ip, cr.ssh_port, cr.ssh_username, cr.external_ssh_key_id,
                cr.gateway_slug, cr.gateway_ssh_port, cr.gateway_port_range_start, cr.gateway_port_range_end,
                cr.offering_id, cr.listing_mode,
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
                gateway_slug, gateway_ssh_port, gateway_port_range_start, gateway_port_range_end,
                offering_id, listing_mode,
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

    pub async fn update_cloud_resource_provisioned(
        &self,
        id: &Uuid,
        public_ip: &str,
        external_ssh_key_id: &str,
        gateway_slug: &str,
        gateway_ssh_port: i32,
        gateway_port_range_start: i32,
        gateway_port_range_end: i32,
    ) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET status = 'running',
                public_ip = $2,
                external_ssh_key_id = $3,
                gateway_slug = $4,
                gateway_ssh_port = $5,
                gateway_port_range_start = $6,
                gateway_port_range_end = $7,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(id)
        .bind(public_ip)
        .bind(external_ssh_key_id)
        .bind(gateway_slug)
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

    pub async fn update_cloud_resource_status(
        &self,
        id: &Uuid,
        status: &str,
    ) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET status = $2, updated_at = NOW()
            WHERE id = $1
            "#
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

    pub async fn acquire_cloud_resource_lock(&self, id: &Uuid, lock_holder: &str) -> Result<bool> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_resources
            SET provisioning_locked_at = NOW(),
                provisioning_locked_by = $2
            WHERE id = $1 
              AND (provisioning_locked_at IS NULL 
                   OR provisioning_locked_at < NOW() - INTERVAL '10 minutes')
            "#
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
            "#
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
            "#
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
            "#
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

    #[allow(clippy::type_complexity)]
    pub async fn get_pending_termination_resources(&self, limit: i64) -> Result<Vec<(Uuid, String, Option<String>, String, String)>> {
        let rows = sqlx::query_as::<_, (Uuid, String, Option<String>, String, String)>(
            r#"
            SELECT 
                cr.id, cr.external_id, cr.external_ssh_key_id,
                ca.backend_type, ca.credentials_encrypted
            FROM cloud_resources cr
            JOIN cloud_accounts ca ON cr.cloud_account_id = ca.id
            WHERE cr.status = 'deleting'
              AND ca.is_valid = true
              AND (cr.provisioning_locked_at IS NULL 
                   OR cr.provisioning_locked_at < NOW() - INTERVAL '10 minutes')
            ORDER BY cr.updated_at ASC
            LIMIT $1
            "#
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
        let marked = db.mark_contract_resource_for_deletion(&contract_id).await.unwrap();
        assert!(marked);

        // Verify status changed to 'deleting'
        let resource = db.get_cloud_resource(&resource_id, &account.id).await.unwrap().unwrap();
        assert_eq!(resource.resource.status, "deleting");

        // Marking again should return false (already deleting)
        let marked_again = db.mark_contract_resource_for_deletion(&contract_id).await.unwrap();
        assert!(!marked_again);
    }

    #[tokio::test]
    async fn test_mark_contract_resource_for_deletion_no_resource() {
        let db = setup_test_db().await;

        // Non-existent contract_id - should return false, not error
        let marked = db.mark_contract_resource_for_deletion(&[0xEEu8; 32]).await.unwrap();
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

        let found = db.find_hetzner_cloud_account_for_provider(&pubkey).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().to_string(), cloud_account.id);

        // Non-existent provider should return None
        let not_found = db.find_hetzner_cloud_account_for_provider(&[99u8; 32]).await.unwrap();
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
}
