//! Agent delegation database operations.
//!
//! Handles delegated keypairs for provider provisioning agents.

use super::types::Database;
use anyhow::Result;
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
}

impl AgentPermission {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentPermission::Provision => "provision",
            AgentPermission::HealthCheck => "health_check",
            AgentPermission::Heartbeat => "heartbeat",
            AgentPermission::FetchContracts => "fetch_contracts",
        }
    }

    /// All permissions for a standard provisioning agent
    pub fn all() -> Vec<Self> {
        vec![
            AgentPermission::Provision,
            AgentPermission::HealthCheck,
            AgentPermission::Heartbeat,
            AgentPermission::FetchContracts,
        ]
    }
}

impl std::str::FromStr for AgentPermission {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "provision" => Ok(AgentPermission::Provision),
            "health_check" => Ok(AgentPermission::HealthCheck),
            "heartbeat" => Ok(AgentPermission::Heartbeat),
            "fetch_contracts" => Ok(AgentPermission::FetchContracts),
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
}

/// Internal row type for database queries
#[derive(Debug, sqlx::FromRow)]
struct DelegationRow {
    agent_pubkey: Vec<u8>,
    provider_pubkey: Vec<u8>,
    permissions: String,
    expires_at_ns: Option<i64>,
    label: Option<String>,
    signature: Vec<u8>,
    created_at_ns: i64,
    revoked_at_ns: Option<i64>,
}

/// Internal row type for agent status queries
#[derive(Debug, sqlx::FromRow)]
struct AgentStatusRow {
    provider_pubkey: Vec<u8>,
    online: i64,
    last_heartbeat_ns: Option<i64>,
    version: Option<String>,
    provisioner_type: Option<String>,
    capabilities: Option<String>,
    active_contracts: Option<i64>,
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
        provider_pubkey: &[u8],
        agent_pubkey: &[u8],
        permissions: &[AgentPermission],
        expires_at_ns: Option<i64>,
        label: Option<&str>,
        signature: &[u8],
    ) -> Result<()> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let permissions_json =
            serde_json::to_string(&permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>())?;

        sqlx::query!(
            r#"INSERT INTO provider_agent_delegations
               (provider_pubkey, agent_pubkey, permissions, expires_at_ns, label, signature, created_at_ns)
               VALUES (?, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(agent_pubkey) DO UPDATE SET
                   provider_pubkey = excluded.provider_pubkey,
                   permissions = excluded.permissions,
                   expires_at_ns = excluded.expires_at_ns,
                   label = excluded.label,
                   signature = excluded.signature,
                   revoked_at_ns = NULL"#,
            provider_pubkey,
            agent_pubkey,
            permissions_json,
            expires_at_ns,
            label,
            signature,
            now_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get an active delegation for an agent pubkey.
    /// Returns None if delegation doesn't exist, is revoked, or is expired.
    pub async fn get_active_delegation(
        &self,
        agent_pubkey: &[u8],
    ) -> Result<Option<(Vec<u8>, Vec<AgentPermission>, Vec<u8>)>> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let row = sqlx::query_as!(
            DelegationRow,
            r#"SELECT agent_pubkey, provider_pubkey, permissions, expires_at_ns, label, signature, created_at_ns, revoked_at_ns
               FROM provider_agent_delegations
               WHERE agent_pubkey = ?
                 AND revoked_at_ns IS NULL
                 AND (expires_at_ns IS NULL OR expires_at_ns > ?)"#,
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
                Ok(Some((r.provider_pubkey, permissions, r.signature)))
            }
            None => Ok(None),
        }
    }

    /// List all delegations for a provider (including revoked/expired for audit).
    pub async fn list_agent_delegations(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<AgentDelegation>> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let rows = sqlx::query_as!(
            DelegationRow,
            r#"SELECT agent_pubkey, provider_pubkey, permissions, expires_at_ns, label, signature, created_at_ns, revoked_at_ns
               FROM provider_agent_delegations
               WHERE provider_pubkey = ?
               ORDER BY created_at_ns DESC"#,
            provider_pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        let delegations = rows
            .into_iter()
            .map(|r| {
                let perms: Vec<String> = serde_json::from_str(&r.permissions).unwrap_or_default();
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
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let result = sqlx::query!(
            r#"UPDATE provider_agent_delegations
               SET revoked_at_ns = ?
               WHERE provider_pubkey = ? AND agent_pubkey = ? AND revoked_at_ns IS NULL"#,
            now_ns,
            provider_pubkey,
            agent_pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Update agent heartbeat status.
    pub async fn update_agent_heartbeat(
        &self,
        provider_pubkey: &[u8],
        version: Option<&str>,
        provisioner_type: Option<&str>,
        capabilities: Option<&[String]>,
        active_contracts: i64,
    ) -> Result<()> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let caps_json = capabilities.map(|c| serde_json::to_string(c).unwrap_or_default());

        sqlx::query!(
            r#"INSERT INTO provider_agent_status
               (provider_pubkey, online, last_heartbeat_ns, version, provisioner_type, capabilities, active_contracts, updated_at_ns)
               VALUES (?, 1, ?, ?, ?, ?, ?, ?)
               ON CONFLICT(provider_pubkey) DO UPDATE SET
                   online = 1,
                   last_heartbeat_ns = excluded.last_heartbeat_ns,
                   version = COALESCE(excluded.version, provider_agent_status.version),
                   provisioner_type = COALESCE(excluded.provisioner_type, provider_agent_status.provisioner_type),
                   capabilities = COALESCE(excluded.capabilities, provider_agent_status.capabilities),
                   active_contracts = excluded.active_contracts,
                   updated_at_ns = excluded.updated_at_ns"#,
            provider_pubkey,
            now_ns,
            version,
            provisioner_type,
            caps_json,
            active_contracts,
            now_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get agent status for a provider.
    pub async fn get_agent_status(&self, provider_pubkey: &[u8]) -> Result<Option<AgentStatus>> {
        let row = sqlx::query_as::<_, AgentStatusRow>(
            r#"SELECT provider_pubkey, online, last_heartbeat_ns, version, provisioner_type, capabilities, active_contracts
               FROM provider_agent_status
               WHERE provider_pubkey = ?"#,
        )
        .bind(provider_pubkey)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => {
                let caps: Option<Vec<String>> = r
                    .capabilities
                    .as_ref()
                    .and_then(|c| serde_json::from_str::<Vec<String>>(c).ok());

                // Check if agent is still online (heartbeat within last 5 minutes)
                let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
                let five_mins_ns = 5 * 60 * 1_000_000_000i64;
                let online = r.online == 1
                    && r.last_heartbeat_ns
                        .map(|h| now_ns - h < five_mins_ns)
                        .unwrap_or(false);

                Ok(Some(AgentStatus {
                    provider_pubkey: hex::encode(&r.provider_pubkey),
                    online,
                    last_heartbeat_ns: r.last_heartbeat_ns,
                    version: r.version,
                    provisioner_type: r.provisioner_type,
                    capabilities: caps,
                    active_contracts: r.active_contracts.unwrap_or(0),
                }))
            }
            None => Ok(None),
        }
    }

    /// Mark agents as offline if no heartbeat in last 5 minutes.
    /// This should be called periodically (e.g., every minute) by a background job.
    #[allow(dead_code)]
    pub async fn mark_stale_agents_offline(&self) -> Result<u64> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let five_mins_ns = 5 * 60 * 1_000_000_000i64;
        let cutoff_ns = now_ns - five_mins_ns;

        let result = sqlx::query!(
            r#"UPDATE provider_agent_status
               SET online = 0, updated_at_ns = ?
               WHERE online = 1 AND (last_heartbeat_ns IS NULL OR last_heartbeat_ns < ?)"#,
            now_ns,
            cutoff_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Verify a delegation signature.
    /// Message format: agent_pubkey || provider_pubkey || permissions_json || expires_at_ns || label
    pub fn verify_delegation_signature(
        provider_pubkey: &[u8],
        agent_pubkey: &[u8],
        permissions: &[AgentPermission],
        expires_at_ns: Option<i64>,
        label: Option<&str>,
        signature: &[u8],
    ) -> Result<()> {
        use dcc_common::DccIdentity;

        // Build message to verify
        let mut message = Vec::new();
        message.extend_from_slice(agent_pubkey);
        message.extend_from_slice(provider_pubkey);
        let permissions_json =
            serde_json::to_string(&permissions.iter().map(|p| p.as_str()).collect::<Vec<_>>())?;
        message.extend_from_slice(permissions_json.as_bytes());
        if let Some(exp) = expires_at_ns {
            message.extend_from_slice(&exp.to_le_bytes());
        }
        if let Some(lbl) = label {
            message.extend_from_slice(lbl.as_bytes());
        }

        // Verify signature
        let identity = DccIdentity::new_verifying_from_bytes(provider_pubkey)?;
        identity.verify_bytes(&message, signature)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
