//! Agent pools database operations.
//!
//! Handles agent pool management for load distribution and location-based routing.

use super::agent_delegations::{AgentDelegation, DelegationRow};
use super::types::Database;
use anyhow::{anyhow, Result};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// Re-export region utilities for convenience
pub use crate::regions::{country_to_region, is_valid_region, REGIONS};

/// Agent pool for grouping agents by location and provisioner type
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AgentPool {
    /// Unique pool identifier
    pub pool_id: String,
    /// Provider's public key (hex)
    pub provider_pubkey: String,
    /// Human-readable name (e.g., "eu-proxmox")
    pub name: String,
    /// Location/region identifier (e.g., "europe", "na", "apac")
    pub location: String,
    /// Provisioner type (e.g., "proxmox", "script", "manual")
    pub provisioner_type: String,
    /// When pool was created
    #[ts(type = "number")]
    pub created_at_ns: i64,
}

/// Agent pool with agent statistics
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct AgentPoolWithStats {
    /// Pool details
    #[serde(flatten)]
    #[oai(flatten)]
    pub pool: AgentPool,
    /// Total number of agents in pool
    #[ts(type = "number")]
    pub agent_count: i64,
    /// Number of online agents
    #[ts(type = "number")]
    pub online_count: i64,
    /// Total active contracts across all agents
    #[ts(type = "number")]
    pub active_contracts: i64,
}

/// Setup token for agent registration
#[derive(Debug, Clone, Serialize, Deserialize, TS, Object)]
#[ts(export, export_to = "../../website/src/lib/types/generated/")]
#[oai(rename_all = "camelCase")]
#[serde(rename_all = "camelCase")]
pub struct SetupToken {
    /// Unique token (format: apt_{location}_{uuid})
    pub token: String,
    /// Pool this token is for
    pub pool_id: String,
    /// Optional label for the agent
    pub label: Option<String>,
    /// When token was created
    #[ts(type = "number")]
    pub created_at_ns: i64,
    /// When token expires
    #[ts(type = "number")]
    pub expires_at_ns: i64,
    /// When token was used (null if unused)
    #[ts(type = "number | null")]
    pub used_at_ns: Option<i64>,
    /// Agent pubkey that used this token (hex, null if unused)
    pub used_by_agent: Option<String>,
}

/// Internal row for pool queries
#[derive(Debug, sqlx::FromRow)]
struct PoolRow {
    pool_id: String,
    provider_pubkey: Vec<u8>,
    name: String,
    location: String,
    provisioner_type: String,
    created_at_ns: i64,
}

/// Internal row for pool with stats queries
#[derive(Debug, sqlx::FromRow)]
struct PoolWithStatsRow {
    pool_id: String,
    provider_pubkey: Vec<u8>,
    name: String,
    location: String,
    provisioner_type: String,
    created_at_ns: i64,
    agent_count: i64,
    online_count: i64,
    active_contracts: i64,
}

/// Internal row for setup token queries
#[derive(Debug, sqlx::FromRow)]
struct SetupTokenRow {
    token: String,
    pool_id: String,
    label: Option<String>,
    created_at_ns: i64,
    expires_at_ns: i64,
    used_at_ns: Option<i64>,
    used_by_agent: Option<Vec<u8>>,
}


impl Database {
    // ==================== Pool CRUD ====================

    /// Create a new agent pool.
    pub async fn create_agent_pool(
        &self,
        pool_id: &str,
        provider_pubkey: &[u8],
        name: &str,
        location: &str,
        provisioner_type: &str,
    ) -> Result<AgentPool> {
        // Validate location is a known region identifier
        if !is_valid_region(location) {
            let valid_regions: Vec<&str> = REGIONS.iter().map(|(code, _)| *code).collect();
            return Err(anyhow!(
                "Invalid location '{}': must be one of: {}",
                location,
                valid_regions.join(", ")
            ));
        }

        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        sqlx::query!(
            r#"INSERT INTO agent_pools (pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns)
               VALUES (?, ?, ?, ?, ?, ?)"#,
            pool_id,
            provider_pubkey,
            name,
            location,
            provisioner_type,
            now_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(AgentPool {
            pool_id: pool_id.to_string(),
            provider_pubkey: hex::encode(provider_pubkey),
            name: name.to_string(),
            location: location.to_string(),
            provisioner_type: provisioner_type.to_string(),
            created_at_ns: now_ns,
        })
    }

    /// Get a pool by ID.
    pub async fn get_agent_pool(&self, pool_id: &str) -> Result<Option<AgentPool>> {
        let row = sqlx::query_as::<_, PoolRow>(
            "SELECT pool_id, provider_pubkey, name, location, provisioner_type, created_at_ns FROM agent_pools WHERE pool_id = ?",
        )
        .bind(pool_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| AgentPool {
            pool_id: r.pool_id,
            provider_pubkey: hex::encode(&r.provider_pubkey),
            name: r.name,
            location: r.location,
            provisioner_type: r.provisioner_type,
            created_at_ns: r.created_at_ns,
        }))
    }

    /// List all pools for a provider with agent statistics.
    pub async fn list_agent_pools_with_stats(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<Vec<AgentPoolWithStats>> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let five_mins_ns = 5 * 60 * 1_000_000_000i64;
        let cutoff_ns = now_ns - five_mins_ns;

        let rows = sqlx::query_as::<_, PoolWithStatsRow>(
            r#"SELECT
                p.pool_id, p.provider_pubkey, p.name, p.location, p.provisioner_type, p.created_at_ns,
                COUNT(DISTINCT d.agent_pubkey) as agent_count,
                COUNT(DISTINCT CASE WHEN s.online = 1 AND s.last_heartbeat_ns > ? THEN d.agent_pubkey END) as online_count,
                COALESCE(SUM(s.active_contracts), 0) as active_contracts
            FROM agent_pools p
            LEFT JOIN provider_agent_delegations d ON d.pool_id = p.pool_id AND d.revoked_at_ns IS NULL
            LEFT JOIN provider_agent_status s ON s.provider_pubkey = p.provider_pubkey
            WHERE p.provider_pubkey = ?
            GROUP BY p.pool_id
            ORDER BY p.created_at_ns DESC"#,
        )
        .bind(cutoff_ns)
        .bind(provider_pubkey)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| AgentPoolWithStats {
                pool: AgentPool {
                    pool_id: r.pool_id,
                    provider_pubkey: hex::encode(&r.provider_pubkey),
                    name: r.name,
                    location: r.location,
                    provisioner_type: r.provisioner_type,
                    created_at_ns: r.created_at_ns,
                },
                agent_count: r.agent_count,
                online_count: r.online_count,
                active_contracts: r.active_contracts,
            })
            .collect())
    }

    /// Update a pool's name, location, or provisioner type.
    pub async fn update_agent_pool(
        &self,
        pool_id: &str,
        provider_pubkey: &[u8],
        name: Option<&str>,
        location: Option<&str>,
        provisioner_type: Option<&str>,
    ) -> Result<bool> {
        // Validate location if provided
        if let Some(loc) = location {
            if !is_valid_region(loc) {
                let valid_regions: Vec<&str> = REGIONS.iter().map(|(code, _)| *code).collect();
                return Err(anyhow!(
                    "Invalid location '{}': must be one of: {}",
                    loc,
                    valid_regions.join(", ")
                ));
            }
        }

        // Build dynamic update - only update provided fields
        let mut updates = Vec::new();
        if name.is_some() {
            updates.push("name = ?");
        }
        if location.is_some() {
            updates.push("location = ?");
        }
        if provisioner_type.is_some() {
            updates.push("provisioner_type = ?");
        }

        if updates.is_empty() {
            return Ok(false);
        }

        let query = format!(
            "UPDATE agent_pools SET {} WHERE pool_id = ? AND provider_pubkey = ?",
            updates.join(", ")
        );

        let mut q = sqlx::query(&query);
        if let Some(n) = name {
            q = q.bind(n);
        }
        if let Some(l) = location {
            q = q.bind(l);
        }
        if let Some(pt) = provisioner_type {
            q = q.bind(pt);
        }
        q = q.bind(pool_id).bind(provider_pubkey);

        let result = q.execute(&self.pool).await?;
        Ok(result.rows_affected() > 0)
    }

    /// Delete a pool. Fails if pool has any agents assigned.
    pub async fn delete_agent_pool(&self, pool_id: &str, provider_pubkey: &[u8]) -> Result<bool> {
        // Check if pool has agents
        let agent_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM provider_agent_delegations WHERE pool_id = ? AND revoked_at_ns IS NULL",
        )
        .bind(pool_id)
        .fetch_one(&self.pool)
        .await?;

        if agent_count > 0 {
            return Err(anyhow!(
                "Cannot delete pool with {} active agents. Revoke agents first.",
                agent_count
            ));
        }

        let result = sqlx::query!(
            "DELETE FROM agent_pools WHERE pool_id = ? AND provider_pubkey = ?",
            pool_id,
            provider_pubkey
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Setup Tokens ====================

    /// Create a setup token for agent registration.
    /// Token format: apt_{location}_{uuid}
    pub async fn create_setup_token(
        &self,
        pool_id: &str,
        label: Option<&str>,
        expires_in_hours: u32,
    ) -> Result<SetupToken> {
        // Get pool to include location in token prefix
        let pool = self
            .get_agent_pool(pool_id)
            .await?
            .ok_or_else(|| anyhow!("Pool not found: {}", pool_id))?;

        let uuid = uuid::Uuid::new_v4().to_string().replace('-', "");
        let token = format!("apt_{}_{}", pool.location, &uuid[..16]);

        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let expires_at_ns = now_ns + (expires_in_hours as i64 * 3600 * 1_000_000_000);

        sqlx::query!(
            r#"INSERT INTO agent_setup_tokens (token, pool_id, label, created_at_ns, expires_at_ns)
               VALUES (?, ?, ?, ?, ?)"#,
            token,
            pool_id,
            label,
            now_ns,
            expires_at_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(SetupToken {
            token,
            pool_id: pool_id.to_string(),
            label: label.map(|s| s.to_string()),
            created_at_ns: now_ns,
            expires_at_ns,
            used_at_ns: None,
            used_by_agent: None,
        })
    }

    /// Validate and consume a setup token. Returns pool info if valid.
    /// Marks token as used atomically to prevent reuse.
    pub async fn validate_and_use_setup_token(
        &self,
        token: &str,
        agent_pubkey: &[u8],
    ) -> Result<(AgentPool, Option<String>)> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        // Atomically mark token as used and get pool info
        let row = sqlx::query_as::<_, SetupTokenRow>(
            r#"UPDATE agent_setup_tokens
               SET used_at_ns = ?, used_by_agent = ?
               WHERE token = ? AND used_at_ns IS NULL AND expires_at_ns > ?
               RETURNING token, pool_id, label, created_at_ns, expires_at_ns, used_at_ns, used_by_agent"#,
        )
        .bind(now_ns)
        .bind(agent_pubkey)
        .bind(token)
        .bind(now_ns)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(token_row) => {
                let pool = self
                    .get_agent_pool(&token_row.pool_id)
                    .await?
                    .ok_or_else(|| anyhow!("Pool not found for token"))?;
                Ok((pool, token_row.label))
            }
            None => {
                // Check why it failed
                let existing = sqlx::query_as::<_, SetupTokenRow>(
                    "SELECT token, pool_id, label, created_at_ns, expires_at_ns, used_at_ns, used_by_agent FROM agent_setup_tokens WHERE token = ?",
                )
                .bind(token)
                .fetch_optional(&self.pool)
                .await?;

                match existing {
                    None => Err(anyhow!("Invalid setup token")),
                    Some(t) if t.used_at_ns.is_some() => Err(anyhow!("Setup token already used")),
                    Some(t) if t.expires_at_ns <= now_ns => Err(anyhow!("Setup token expired")),
                    _ => Err(anyhow!("Setup token validation failed")),
                }
            }
        }
    }

    /// List pending (unused, unexpired) setup tokens for a pool.
    pub async fn list_pending_setup_tokens(&self, pool_id: &str) -> Result<Vec<SetupToken>> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let rows = sqlx::query_as::<_, SetupTokenRow>(
            r#"SELECT token, pool_id, label, created_at_ns, expires_at_ns, used_at_ns, used_by_agent
               FROM agent_setup_tokens
               WHERE pool_id = ? AND used_at_ns IS NULL AND expires_at_ns > ?
               ORDER BY created_at_ns DESC"#,
        )
        .bind(pool_id)
        .bind(now_ns)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| SetupToken {
                token: r.token,
                pool_id: r.pool_id,
                label: r.label,
                created_at_ns: r.created_at_ns,
                expires_at_ns: r.expires_at_ns,
                used_at_ns: r.used_at_ns,
                used_by_agent: r.used_by_agent.map(|b| hex::encode(&b)),
            })
            .collect())
    }

    /// Delete a setup token.
    pub async fn delete_setup_token(&self, token: &str) -> Result<bool> {
        let result = sqlx::query!("DELETE FROM agent_setup_tokens WHERE token = ?", token)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Cleanup expired and unused setup tokens.
    pub async fn cleanup_expired_setup_tokens(&self) -> Result<u64> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);

        let result = sqlx::query!(
            "DELETE FROM agent_setup_tokens WHERE expires_at_ns < ? AND used_at_ns IS NULL",
            now_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // ==================== Pool Matching ====================

    /// Get agent's pool ID from their delegation.
    pub async fn get_agent_pool_id(&self, agent_pubkey: &[u8]) -> Result<Option<String>> {
        let pool_id: Option<String> = sqlx::query_scalar(
            "SELECT pool_id FROM provider_agent_delegations WHERE agent_pubkey = ? AND revoked_at_ns IS NULL",
        )
        .bind(agent_pubkey)
        .fetch_optional(&self.pool)
        .await?
        .flatten();

        Ok(pool_id)
    }

    /// List all active agent delegations for a specific pool.
    pub async fn list_agents_in_pool(&self, pool_id: &str) -> Result<Vec<AgentDelegation>> {
        let now_ns = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let rows = sqlx::query_as::<_, DelegationRow>(
            "SELECT agent_pubkey, provider_pubkey, permissions, expires_at_ns, created_at_ns, revoked_at_ns, label, pool_id, signature FROM provider_agent_delegations WHERE pool_id = ? AND revoked_at_ns IS NULL ORDER BY created_at_ns DESC"
        )
        .bind(pool_id)
        .fetch_all(&self.pool)
        .await?;

        let delegations = rows
            .into_iter()
            .map(|row| {
                let perms: Vec<String> = serde_json::from_str(&row.permissions).unwrap_or_default();
                let active = row.revoked_at_ns.is_none()
                    && (row.expires_at_ns.is_none() || row.expires_at_ns.unwrap() > now_ns);
                AgentDelegation {
                    agent_pubkey: hex::encode(row.agent_pubkey),
                    provider_pubkey: hex::encode(row.provider_pubkey),
                    permissions: perms,
                    expires_at_ns: row.expires_at_ns,
                    created_at_ns: row.created_at_ns,
                    active,
                    pool_id: row.pool_id,
                    label: row.label,
                }
            })
            .collect();
        Ok(delegations)
    }
}

// Tests for region utilities are in api/src/regions.rs
