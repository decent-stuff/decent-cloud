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
                cr.public_ip, cr.ssh_port, cr.ssh_username,
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
                cr.public_ip, cr.ssh_port, cr.ssh_username,
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
                public_ip, ssh_port, ssh_username,
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
                gateway_slug = $3,
                gateway_ssh_port = $4,
                gateway_port_range_start = $5,
                gateway_port_range_end = $6,
                updated_at = NOW()
            WHERE id = $1
            "#
        )
        .bind(id)
        .bind(public_ip)
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
    pub async fn get_pending_termination_resources(&self, limit: i64) -> Result<Vec<(Uuid, String, String, String)>> {
        let rows = sqlx::query_as::<_, (Uuid, String, String, String)>(
            r#"
            SELECT 
                cr.id, cr.external_id,
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

    #[allow(clippy::type_complexity)]
    pub async fn get_pending_provisioning_resources(&self, limit: i64) -> Result<Vec<(Uuid, Uuid, String, String, String, String, String, String, String, String)>> {
        let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, String, String, String, String)>(
            r#"
            SELECT 
                cr.id, cr.cloud_account_id, cr.external_id, cr.name,
                cr.server_type, cr.location, cr.image, cr.ssh_pubkey,
                ca.backend_type, ca.credentials_encrypted
            FROM cloud_resources cr
            JOIN cloud_accounts ca ON cr.cloud_account_id = ca.id
            WHERE cr.status = 'provisioning'
              AND ca.is_valid = true
              AND (cr.provisioning_locked_at IS NULL 
                   OR cr.provisioning_locked_at < NOW() - INTERVAL '10 minutes')
            ORDER BY cr.created_at ASC
            LIMIT $1
            "#
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_constants() {
        assert_eq!("running", "running");
        assert_eq!("provisioning", "provisioning");
    }
}
