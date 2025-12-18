-- Agent delegation system for provider provisioning agents
-- Allows providers to delegate limited permissions to agent keypairs

-- Track agent delegations from provider main keys
CREATE TABLE provider_agent_delegations (
    id INTEGER PRIMARY KEY,
    -- Provider's main public key (the delegator)
    provider_pubkey BLOB NOT NULL,
    -- Agent's public key (the delegatee) - unique across all delegations
    agent_pubkey BLOB NOT NULL UNIQUE,
    -- JSON array of permissions: ["provision", "health_check", "heartbeat", "fetch_contracts"]
    permissions TEXT NOT NULL,
    -- Optional expiration timestamp (nanoseconds), NULL means no expiry
    expires_at_ns INTEGER,
    -- Human-readable label for this agent (e.g., "proxmox-server-1")
    label TEXT,
    -- Signature by provider's main key over: agent_pubkey + provider_pubkey + permissions + expires_at_ns + label
    signature BLOB NOT NULL,
    -- When delegation was created
    created_at_ns INTEGER NOT NULL,
    -- When delegation was revoked (NULL if active)
    revoked_at_ns INTEGER,
    FOREIGN KEY (provider_pubkey) REFERENCES provider_registrations(pubkey)
);

CREATE INDEX idx_agent_delegations_agent ON provider_agent_delegations(agent_pubkey) WHERE revoked_at_ns IS NULL;
CREATE INDEX idx_agent_delegations_provider ON provider_agent_delegations(provider_pubkey);

-- Track provider agent online status (heartbeats)
CREATE TABLE provider_agent_status (
    -- Provider's main public key
    provider_pubkey BLOB PRIMARY KEY,
    -- Whether agent is considered online (1 = online, 0 = offline)
    online INTEGER NOT NULL DEFAULT 0,
    -- Last heartbeat timestamp (nanoseconds)
    last_heartbeat_ns INTEGER,
    -- Agent version string
    version TEXT,
    -- Provisioner type being used (e.g., "proxmox", "hetzner", "docker")
    provisioner_type TEXT,
    -- JSON array of capabilities (e.g., ["vm", "health_check"])
    capabilities TEXT,
    -- Number of active contracts this agent is managing
    active_contracts INTEGER DEFAULT 0,
    -- Last update timestamp
    updated_at_ns INTEGER NOT NULL
);
