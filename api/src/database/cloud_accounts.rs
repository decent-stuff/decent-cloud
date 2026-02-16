//! Cloud accounts database operations.
//!
//! Handles cloud account management for self-provisioning.

use super::types::Database;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

use crate::cloud::types::BackendType;

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CloudAccount {
    pub id: String,
    pub account_id: String,
    pub backend_type: String,
    pub name: String,
    pub config: Option<String>,
    pub is_valid: bool,
    pub last_validated_at: Option<String>,
    pub validation_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CreateCloudAccountInput {
    pub backend_type: String,
    pub name: String,
    pub credentials: String,
    pub config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct CloudAccountWithCatalog {
    #[serde(flatten)]
    #[oai(flatten)]
    pub account: CloudAccount,
    pub catalog: Option<crate::cloud::types::BackendCatalog>,
}

impl Database {
    pub async fn list_cloud_accounts(&self, account_id: &[u8]) -> Result<Vec<CloudAccount>> {
        let rows = sqlx::query_as::<_, (Uuid, Vec<u8>, String, String, Option<String>, bool, Option<DateTime<Utc>>, Option<String>, DateTime<Utc>, DateTime<Utc>)>(
            r#"
            SELECT id, account_id, backend_type, name, config, is_valid, 
                   last_validated_at, validation_error, created_at, updated_at
            FROM cloud_accounts
            WHERE account_id = $1
            ORDER BY created_at DESC
            "#
        )
        .bind(account_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(id, acc_id, backend_type, name, config, is_valid, last_validated_at, validation_error, created_at, updated_at)| {
                CloudAccount {
                    id: id.to_string(),
                    account_id: hex::encode(&acc_id),
                    backend_type,
                    name,
                    config,
                    is_valid,
                    last_validated_at: last_validated_at.map(|t| t.to_rfc3339()),
                    validation_error,
                    created_at: created_at.to_rfc3339(),
                    updated_at: updated_at.to_rfc3339(),
                }
            })
            .collect())
    }

    pub async fn get_cloud_account(&self, id: &Uuid, account_id: &[u8]) -> Result<Option<CloudAccount>> {
        let row = sqlx::query_as::<_, (Uuid, Vec<u8>, String, String, Option<String>, bool, Option<DateTime<Utc>>, Option<String>, DateTime<Utc>, DateTime<Utc>)>(
            r#"
            SELECT id, account_id, backend_type, name, config, is_valid, 
                   last_validated_at, validation_error, created_at, updated_at
            FROM cloud_accounts
            WHERE id = $1 AND account_id = $2
            "#
        )
        .bind(id)
        .bind(account_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(id, acc_id, backend_type, name, config, is_valid, last_validated_at, validation_error, created_at, updated_at)| {
            CloudAccount {
                id: id.to_string(),
                account_id: hex::encode(&acc_id),
                backend_type,
                name,
                config,
                is_valid,
                last_validated_at: last_validated_at.map(|t| t.to_rfc3339()),
                validation_error,
                created_at: created_at.to_rfc3339(),
                updated_at: updated_at.to_rfc3339(),
            }
        }))
    }

    pub async fn create_cloud_account(
        &self,
        account_id: &[u8],
        backend_type: BackendType,
        name: &str,
        credentials_encrypted: &str,
        config: Option<&str>,
    ) -> Result<CloudAccount> {
        let row = sqlx::query_as::<_, (Uuid, Vec<u8>, String, String, Option<String>, bool, Option<DateTime<Utc>>, Option<String>, DateTime<Utc>, DateTime<Utc>)>(
            r#"
            INSERT INTO cloud_accounts (account_id, backend_type, name, credentials_encrypted, config)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, account_id, backend_type, name, config, is_valid, 
                      last_validated_at, validation_error, created_at, updated_at
            "#
        )
        .bind(account_id)
        .bind(backend_type.to_string())
        .bind(name)
        .bind(credentials_encrypted)
        .bind(config)
        .fetch_one(&self.pool)
        .await?;

        Ok(CloudAccount {
            id: row.0.to_string(),
            account_id: hex::encode(&row.1),
            backend_type: row.2,
            name: row.3,
            config: row.4,
            is_valid: row.5,
            last_validated_at: row.6.map(|t| t.to_rfc3339()),
            validation_error: row.7,
            created_at: row.8.to_rfc3339(),
            updated_at: row.9.to_rfc3339(),
        })
    }

    pub async fn update_cloud_account_validation(
        &self,
        id: &Uuid,
        account_id: &[u8],
        is_valid: bool,
        validation_error: Option<&str>,
    ) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE cloud_accounts
            SET is_valid = $3, 
                validation_error = $4,
                last_validated_at = NOW(),
                updated_at = NOW()
            WHERE id = $1 AND account_id = $2
            "#
        )
        .bind(id)
        .bind(account_id)
        .bind(is_valid)
        .bind(validation_error)
        .execute(&self.pool)
        .await?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow!("Cloud account not found"));
        }

        Ok(())
    }

    pub async fn delete_cloud_account(&self, id: &Uuid, account_id: &[u8]) -> Result<bool> {
        // Block deletion if any non-terminal resources exist (prevents orphaned VMs on Hetzner)
        let active_count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM cloud_resources
            WHERE cloud_account_id = $1
              AND status NOT IN ('deleted', 'failed')
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        if active_count.0 > 0 {
            return Err(anyhow!(
                "Cannot delete cloud account: {} active resource(s) exist. Terminate all resources first.",
                active_count.0
            ));
        }

        let rows_affected = sqlx::query(
            r#"
            DELETE FROM cloud_accounts
            WHERE id = $1 AND account_id = $2
            "#
        )
        .bind(id)
        .bind(account_id)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    pub async fn get_cloud_account_credentials(&self, id: &Uuid) -> Result<Option<(Vec<u8>, String, String)>> {
        let row = sqlx::query_as::<_, (Vec<u8>, String, String)>(
            r#"
            SELECT account_id, backend_type, credentials_encrypted
            FROM cloud_accounts
            WHERE id = $1
            "#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    #[test]
    fn test_backend_type_display() {
        assert_eq!(BackendType::Hetzner.to_string(), "hetzner");
        assert_eq!(BackendType::ProxmoxApi.to_string(), "proxmox_api");
    }

    #[tokio::test]
    async fn test_delete_cloud_account_blocked_by_active_resources() {
        let db = setup_test_db().await;

        let pubkey = [20u8; 32];
        let account = db
            .create_account("delete_test", &pubkey, "del@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                BackendType::Hetzner,
                "del-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();
        let ca_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();

        // Create a resource in 'provisioning' status
        db.create_cloud_resource(
            &ca_uuid,
            "pending-test",
            "dc-del-test",
            "cx22",
            "nbg1",
            "ubuntu-24.04",
            "ssh-ed25519 AAAA test",
        )
        .await
        .unwrap();

        // Deletion must fail
        let err = db.delete_cloud_account(&ca_uuid, &account.id).await.unwrap_err();
        assert!(
            err.to_string().contains("active resource(s) exist"),
            "Expected active resources error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_delete_cloud_account_succeeds_after_resources_terminated() {
        let db = setup_test_db().await;

        let pubkey = [21u8; 32];
        let account = db
            .create_account("del_ok_test", &pubkey, "delok@example.com")
            .await
            .unwrap();
        let cloud_account = db
            .create_cloud_account(
                &account.id,
                BackendType::Hetzner,
                "delok-hetzner",
                "encrypted",
                None,
            )
            .await
            .unwrap();
        let ca_uuid: uuid::Uuid = cloud_account.id.parse().unwrap();

        let resource = db
            .create_cloud_resource(
                &ca_uuid,
                "pending-test",
                "dc-delok-test",
                "cx22",
                "nbg1",
                "ubuntu-24.04",
                "ssh-ed25519 AAAA test",
            )
            .await
            .unwrap();
        let resource_id: uuid::Uuid = resource.id.parse().unwrap();

        // Mark resource as deleted
        db.update_cloud_resource_status(&resource_id, "deleted")
            .await
            .unwrap();

        // Now deletion should succeed
        let deleted = db
            .delete_cloud_account(&ca_uuid, &account.id)
            .await
            .unwrap();
        assert!(deleted);
    }
}
