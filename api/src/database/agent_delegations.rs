//! Agent delegation database operations.
//!
//! Handles delegated keypairs for provider provisioning agents.

use super::types::Database;
use anyhow::{Context, Result};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Agent permission types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentPermission {
    /// Can report provisioning status
    Provision,
    /// Can report health checks
    HealthCheck,
    /// Can send heartbeats
    Heartbeat,
    /// Can fetch pending contracts
    FetchContracts,
    /// Can manage gateway DNS records
    DnsManage,
}

impl AgentPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentPermission::Provision => "provision",
            AgentPermission::HealthCheck => "health_check",
            AgentPermission::Heartbeat => "heartbeat",
            AgentPermission::FetchContracts => "fetch_contracts",
            AgentPermission::DnsManage => "dns_manage",
        }
    }

    /// All permissions for a standard provisioning agent
    pub fn all() -> Vec<Self> {
        vec![
            AgentPermission::Provision,
            AgentPermission::HealthCheck,
            AgentPermission::Heartbeat,
            AgentPermission::FetchContracts,
            AgentPermission::DnsManage,
        ]
    }
}

/// Parameters for creating an agent delegation
pub struct CreateAgentDelegationParams<'a> {
    pub provider_pubkey: &'a [u8],
    pub agent_pubkey: &'a [u8],
    pub permissions: &'a [AgentPermission],
    pub expires_at_ns: Option<i64>,
    pub label: Option<&'a str>,
    pub signature: &'a [u8],
    pub pool_id: Option<&'a str>,
}

impl std::str::FromStr for AgentPermission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "provision" => Ok(AgentPermission::Provision),
            "health_check" => Ok(AgentPermission::HealthCheck),
            "heartbeat" => Ok(AgentPermission::Heartbeat),
            "fetch_contracts" => Ok(AgentPermission::FetchContracts),
            "dns_manage" => Ok(AgentPermission::DnsManage),
            _ => Err(()),
        }
    }
}

/// Agent delegation record
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AgentDelegation {
    /// Agent's public key (hex)
    pub agent_pubkey: String,
    /// Provider's main public key (hex)
    pub provider_pubkey: String,
    /// Permissions granted (JSON array)
    pub permissions: Vec<String>,
    /// Expiration timestamp (nanoseconds), null means no expiry
    #[ts(type = "number | null")]
    pub expires_at_ns: Option<i64>,
    /// Human-readable label
    pub label: Option<String>,
    /// When delegation was created
    #[ts(type = "number")]
    pub created_at_ns: i64,
    /// Whether delegation is active (not revoked and not expired)
    pub active: bool,
    /// Pool ID this agent belongs to (null for legacy agents)
    pub pool_id: Option<String>,
    /// Whether agent is currently online (from last heartbeat)
    pub online: bool,
    /// Agent version (from last heartbeat)
    pub version: Option<String>,
    /// Last heartbeat timestamp
    #[ts(type = "number | null")]
    pub last_heartbeat_ns: Option<i64>,
}

/// Agent status record
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    /// Provider's public key (hex)
    pub provider_pubkey: String,
    /// Whether agent is online
    pub online: bool,
    /// Last heartbeat timestamp
    #[ts(type = "number | null")]
    pub last_heartbeat_ns: Option<i64>,
    /// Agent version
    pub version: Option<String>,
    /// Provisioner type (e.g., "proxmox", "hetzner")
    pub provisioner_type: Option<String>,
    /// Capabilities (JSON array)
    pub capabilities: Option<Vec<String>>,
    /// Number of active contracts
    #[ts(type = "number")]
    pub active_contracts: i64,
    /// Hardware resource inventory (JSONB)
    #[ts(as = "Option<dcc_common::api_types::ResourceInventory>")]
    pub resources: Option<serde_json::Value>,
}

/// Internal row type for delegation verification queries
#[derive(Debug, sqlx::FromRow)]
pub struct DelegationRow {
    pub provider_pubkey: Vec<u8>,
    pub permissions: String,
    pub signature: Vec<u8>,
    pub pool_id: Option<String>,
}

/// Internal row type for delegation queries with status info
#[derive(Debug, sqlx::FromRow)]
pub struct DelegationWithStatusRow {
    pub agent_pubkey: Vec<u8>,
    pub provider_pubkey: Vec<u8>,
    pub permissions: String,
    pub expires_at_ns: Option<i64>,
    pub label: Option<String>,
    pub created_at_ns: i64,
    pub revoked_at_ns: Option<i64>,
    pub pool_id: Option<String>,
    pub version: Option<String>,
    pub last_heartbeat_ns: Option<i64>,
    pub online: bool,
}

/// Internal row type for agent status queries
#[derive(Debug, sqlx::FromRow)]
struct AgentStatusRow {
    agent_pubkey: Vec<u8>,
    online: bool,
    last_heartbeat_ns: Option<i64>,
    version: Option<String>,
    provisioner_type: Option<String>,
    capabilities: Option<String>,
    active_contracts: Option<i64>,
    resources: Option<serde_json::Value>,
}

impl Database {
    /// Create or update an agent delegation.
    ///
    /// If a delegation already exists for this agent_pubkey, it will be updated
    /// (including un-revoking if previously revoked). This allows re-registration.
    ///
    /// The signature must be verified by the caller before calling this function.
    pub async fn create_agent_delegation(
        &self,
        params: CreateAgentDelegationParams<'_>,
    ) -> Result<()> {
        let CreateAgentDelegationParams {
            provider_pubkey,
            agent_pubkey,
            permissions,
            expires_at_ns,
            label,
            signature,
            pool_id,
        } = params;
        let now_ns = crate::now_ns()?;
        let permissions_json =
            serde_json::to_string(&permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>())?;

        sqlx::query!(
            r#"INSERT INTO provider_agent_delegations
               (provider_pubkey, agent_pubkey, permissions, expires_at_ns, label, signature, created_at_ns, pool_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               ON CONFLICT(agent_pubkey) DO UPDATE SET
                   provider_pubkey = excluded.provider_pubkey,
                   permissions = excluded.permissions,
                   expires_at_ns = excluded.expires_at_ns,
                   label = excluded.label,
                   signature = excluded.signature,
                   pool_id = excluded.pool_id,
                   revoked_at_ns = NULL"#,
            provider_pubkey,
            agent_pubkey,
            permissions_json,
            expires_at_ns,
            label,
            signature,
            now_ns,
            pool_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get an active delegation for an agent pubkey.
    /// Returns None if delegation doesn't exist, is revoked, or is expired.
    /// Returns (provider_pubkey, permissions, signature, pool_id)
    pub async fn get_active_delegation(
        &self,
        agent_pubkey: &[u8],
    ) -> Result<Option<(Vec<u8>, Vec<AgentPermission>, Vec<u8>, Option<String>)>> {
        let now_ns = crate::now_ns()?;

        let row = sqlx::query_as!(
            DelegationRow,
            r#"SELECT provider_pubkey, permissions, signature, pool_id
               FROM provider_agent_delegations
               WHERE agent_pubkey = $1
                 AND revoked_at_ns IS NULL
                 AND (expires_at_ns IS NULL OR expires_at_ns > $2)"#,
            agent_pubkey,
            now_ns
        )
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let perms: Vec<String> = serde_json::from_str(&r.permissions)?;
                let permissions: Vec<AgentPermission> =
                    perms.iter().filter_map(|s| s.parse().ok()).collect();
                Ok(Some((
                    r.provider_pubkey,
                    permissions,
                    r.signature,
                    r.pool_id,
                )))
            }
            None => Ok(None),
        }
    }

    /// List all delegations for a provider (including revoked/expired for audit).
    pub async fn list_agent_delegations(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<AgentDelegation>> {
        let now_ns = crate::now_ns()?;
        // 5 minutes in nanoseconds for online check
        let online_threshold_ns = now_ns - (5 * 60 * 1_000_000_000i64);

        // Join with status table to get version and online status
        let rows = sqlx::query_as::<_, DelegationWithStatusRow>(
            r#"SELECT
                d.agent_pubkey, d.provider_pubkey, d.permissions, d.expires_at_ns,
                d.label, d.created_at_ns, d.revoked_at_ns, d.pool_id,
                s.version, s.last_heartbeat_ns,
                COALESCE(s.online = TRUE AND s.last_heartbeat_ns > $2, FALSE) as online
               FROM provider_agent_delegations d
               LEFT JOIN provider_agent_status s ON d.agent_pubkey = s.agent_pubkey
               WHERE d.provider_pubkey = $1
               ORDER BY d.created_at_ns DESC"#,
        )
        .bind(provider_pubkey)
        .bind(online_threshold_ns)
        .fetch_all(&self.pool)
        .await?;

        let delegations = rows
            .into_iter()
            .map(|r| {
                let perms: Vec<String> = match serde_json::from_str(&r.permissions) {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to parse permissions JSON for agent {}: {e}",
                            hex::encode(&r.agent_pubkey)
                        );
                        vec![]
                    }
                };
                let active = r.revoked_at_ns.is_none()
                    && r.expires_at_ns.map(|e| e > now_ns).unwrap_or(true);
                AgentDelegation {
                    agent_pubkey: hex::encode(&r.agent_pubkey),
                    provider_pubkey: hex::encode(&r.provider_pubkey),
                    permissions: perms,
                    expires_at_ns: r.expires_at_ns,
                    label: r.label,
                    created_at_ns: r.created_at_ns,
                    active,
                    pool_id: r.pool_id,
                    online: r.online,
                    version: r.version,
                    last_heartbeat_ns: r.last_heartbeat_ns,
                }
            })
            .collect();

        Ok(delegations)
    }

    /// Revoke an agent delegation.
    pub async fn revoke_agent_delegation(
        &self,
        provider_pubkey: &[u8],
        agent_pubkey: &[u8],
    ) -> Result<bool> {
        let now_ns = crate::now_ns()?;

        let result = sqlx::query!(
            r#"UPDATE provider_agent_delegations
               SET revoked_at_ns = $1
               WHERE provider_pubkey = $2 AND agent_pubkey = $3 AND revoked_at_ns IS NULL"#,
            now_ns,
            provider_pubkey,
            agent_pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update agent delegation label.
    pub async fn update_agent_delegation_label(
        &self,
        provider_pubkey: &[u8],
        agent_pubkey: &[u8],
        label: &str,
    ) -> Result<bool> {
        let result = sqlx::query!(
            r#"UPDATE provider_agent_delegations
               SET label = $1
               WHERE provider_pubkey = $2 AND agent_pubkey = $3"#,
            label,
            provider_pubkey,
            agent_pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update agent heartbeat status, keyed by agent_pubkey for per-agent isolation.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_agent_heartbeat(
        &self,
        agent_pubkey: &[u8],
        provider_pubkey: &[u8],
        version: Option<&str>,
        provisioner_type: Option<&str>,
        capabilities: Option<&[String]>,
        active_contracts: i64,
        resources: Option<&serde_json::Value>,
    ) -> Result<()> {
        let now_ns = crate::now_ns()?;
        let caps_json = capabilities
            .map(|c| serde_json::to_string(c).context("Failed to serialize capabilities"))
            .transpose()?;

        sqlx::query!(
            r#"INSERT INTO provider_agent_status
               (agent_pubkey, provider_pubkey, online, last_heartbeat_ns, version, provisioner_type, capabilities, active_contracts, updated_at_ns, resources)
               VALUES ($1, $2, TRUE, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT(agent_pubkey) DO UPDATE SET
                   provider_pubkey = excluded.provider_pubkey,
                   online = TRUE,
                   last_heartbeat_ns = excluded.last_heartbeat_ns,
                   version = COALESCE(excluded.version, provider_agent_status.version),
                   provisioner_type = COALESCE(excluded.provisioner_type, provider_agent_status.provisioner_type),
                   capabilities = COALESCE(excluded.capabilities, provider_agent_status.capabilities),
                   active_contracts = excluded.active_contracts,
                   updated_at_ns = excluded.updated_at_ns,
                   resources = COALESCE(excluded.resources, provider_agent_status.resources)"#,
            agent_pubkey,
            provider_pubkey,
            now_ns,
            version,
            provisioner_type,
            caps_json,
            active_contracts,
            now_ns,
            resources
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agent status for a provider.
    ///
    /// Returns the most recently seen agent's status for the given provider.
    /// Use `list_agent_delegations` to get per-agent status for all agents.
    pub async fn get_agent_status(&self, provider_pubkey: &[u8]) -> Result<Option<AgentStatus>> {
        let row = sqlx::query_as::<_, AgentStatusRow>(
            r#"SELECT agent_pubkey, online, last_heartbeat_ns, version, provisioner_type, capabilities, active_contracts, resources
               FROM provider_agent_status
               WHERE provider_pubkey = $1
               ORDER BY last_heartbeat_ns DESC NULLS LAST
               LIMIT 1"#,
        )
        .bind(provider_pubkey)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let caps: Option<Vec<String>> = r.capabilities.as_ref().and_then(|c| {
                    match serde_json::from_str::<Vec<String>>(c) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to parse capabilities JSON for agent {}: {e}",
                                hex::encode(&r.agent_pubkey)
                            );
                            None
                        }
                    }
                });

                // Check if agent is still online (heartbeat within last 5 minutes)
                let now_ns = crate::now_ns()?;
                let five_mins_ns = 5 * 60 * 1_000_000_000i64;
                let online = r.online
                    && r.last_heartbeat_ns
                        .map(|h| now_ns - h < five_mins_ns)
                        .unwrap_or(false);

                Ok(Some(AgentStatus {
                    provider_pubkey: hex::encode(provider_pubkey),
                    online,
                    last_heartbeat_ns: r.last_heartbeat_ns,
                    version: r.version,
                    provisioner_type: r.provisioner_type,
                    capabilities: caps,
                    active_contracts: r.active_contracts.unwrap_or(0),
                    resources: r.resources,
                }))
            }
            None => Ok(None),
        }
    }

    /// Mark agents as offline if no heartbeat in last 5 minutes.
    /// Called periodically by CleanupService background job.
    pub async fn mark_stale_agents_offline(&self) -> Result<u64> {
        let now_ns = crate::now_ns()?;
        let five_mins_ns = 5 * 60 * 1_000_000_000i64;
        let cutoff_ns = now_ns - five_mins_ns;

        let result = sqlx::query!(
            r#"UPDATE provider_agent_status
               SET online = FALSE, updated_at_ns = $1
               WHERE online = TRUE AND (last_heartbeat_ns IS NULL OR last_heartbeat_ns < $2)"#,
            now_ns,
            cutoff_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;

    /// Two agents for the same provider must produce separate heartbeat rows.
    /// Before the fix (provider_pubkey as PK), the second heartbeat would overwrite the first.
    #[tokio::test]
    async fn test_per_agent_heartbeat_isolation() {
        let db = setup_test_db().await;
        let provider_pubkey = vec![0xAAu8; 32];
        let agent1_pubkey = vec![0x01u8; 32];
        let agent2_pubkey = vec![0x02u8; 32];

        // Register provider
        sqlx::query(
            "INSERT INTO provider_registrations (pubkey, signature, created_at_ns) VALUES ($1, '\\x00', 0)",
        )
        .bind(&provider_pubkey)
        .execute(&db.pool)
        .await
        .unwrap();

        // Agent 1 heartbeat
        db.update_agent_heartbeat(
            &agent1_pubkey,
            &provider_pubkey,
            Some("v1.0.0"),
            Some("proxmox"),
            None,
            3,
            None,
        )
        .await
        .unwrap();

        // Agent 2 heartbeat — must NOT overwrite agent 1's row
        db.update_agent_heartbeat(
            &agent2_pubkey,
            &provider_pubkey,
            Some("v1.1.0"),
            Some("proxmox"),
            None,
            7,
            None,
        )
        .await
        .unwrap();

        // Count rows for this provider — must be 2, not 1
        let row_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM provider_agent_status WHERE provider_pubkey = $1",
        )
        .bind(&provider_pubkey)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(row_count, 2, "each agent must have its own status row");

        // Agent 1's data must be preserved
        let agent1_contracts: i64 = sqlx::query_scalar(
            "SELECT COALESCE(active_contracts, 0) FROM provider_agent_status WHERE agent_pubkey = $1",
        )
        .bind(&agent1_pubkey)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(agent1_contracts, 3, "agent1 active_contracts must be intact");

        // Agent 2's data must be correct
        let agent2_contracts: i64 = sqlx::query_scalar(
            "SELECT COALESCE(active_contracts, 0) FROM provider_agent_status WHERE agent_pubkey = $1",
        )
        .bind(&agent2_pubkey)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        assert_eq!(agent2_contracts, 7, "agent2 active_contracts must be intact");
    }

    #[test]
    fn test_permission_roundtrip() {
        for perm in AgentPermission::all() {
            let s = perm.as_str();
            let parsed: AgentPermission = s.parse().unwrap();
            assert_eq!(perm, parsed);
        }
    }

    #[test]
    fn test_permission_json() {
        let perms = AgentPermission::all();
        let json = serde_json::to_string(&perms).unwrap();
        assert!(json.contains("provision"));
        assert!(json.contains("health_check"));

        let parsed: Vec<AgentPermission> = serde_json::from_str(&json).unwrap();
        assert_eq!(perms, parsed);
    }
}
