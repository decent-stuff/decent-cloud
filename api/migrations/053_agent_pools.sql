-- Agent Pools: Load distribution with location routing
-- Enables multiple DC-Agents per provider with proper contract routing

-- Agent pools for grouping agents by location/type
CREATE TABLE agent_pools (
    pool_id TEXT PRIMARY KEY,
    provider_pubkey BLOB NOT NULL,
    name TEXT NOT NULL,                    -- "eu-proxmox", "us-hetzner"
    location TEXT NOT NULL,                -- "eu", "us", "asia" (region identifier)
    provisioner_type TEXT NOT NULL,        -- "proxmox", "script", "manual"
    created_at_ns INTEGER NOT NULL,
    FOREIGN KEY (provider_pubkey) REFERENCES provider_registrations(pubkey)
);

CREATE INDEX idx_agent_pools_provider ON agent_pools(provider_pubkey);

-- Setup tokens for agent registration (one-time use)
CREATE TABLE agent_setup_tokens (
    token TEXT PRIMARY KEY,                -- Unique token (UUID-based)
    pool_id TEXT NOT NULL,                 -- Which pool this token is for
    label TEXT,                            -- Optional label for the agent
    created_at_ns INTEGER NOT NULL,
    expires_at_ns INTEGER NOT NULL,        -- Token expiry (e.g., 24 hours)
    used_at_ns INTEGER,                    -- When token was used (NULL if unused)
    used_by_agent BLOB,                    -- Agent pubkey that used this token
    FOREIGN KEY (pool_id) REFERENCES agent_pools(pool_id) ON DELETE CASCADE
);

CREATE INDEX idx_setup_tokens_pool ON agent_setup_tokens(pool_id);
CREATE INDEX idx_setup_tokens_unused ON agent_setup_tokens(pool_id) WHERE used_at_ns IS NULL;

-- Link agents to pools (agent can belong to one pool)
ALTER TABLE provider_agent_delegations ADD COLUMN pool_id TEXT REFERENCES agent_pools(pool_id);

-- Offering can explicitly specify pool (overrides location matching)
ALTER TABLE provider_offerings ADD COLUMN agent_pool_id TEXT REFERENCES agent_pools(pool_id);

-- Contract provisioning locks (two-phase commit to prevent races)
ALTER TABLE contract_sign_requests ADD COLUMN provisioning_lock_agent BLOB;
ALTER TABLE contract_sign_requests ADD COLUMN provisioning_lock_at_ns INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN provisioning_lock_expires_ns INTEGER;

CREATE INDEX idx_contracts_lock ON contract_sign_requests(provisioning_lock_expires_ns)
    WHERE provisioning_lock_agent IS NOT NULL;
