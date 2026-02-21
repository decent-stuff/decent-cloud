-- Migration 019: Change provider_agent_status to be keyed by agent_pubkey
-- Previously keyed by provider_pubkey, which caused all agents for the same
-- provider to share a single row. Now each agent has its own status row.

-- Rename existing table as backup, preserving existing data
ALTER TABLE provider_agent_status RENAME TO provider_agent_status_backup;

-- Create new table keyed by agent_pubkey
CREATE TABLE provider_agent_status (
    agent_pubkey BYTEA PRIMARY KEY,
    provider_pubkey BYTEA NOT NULL,
    online BOOLEAN NOT NULL DEFAULT FALSE,
    last_heartbeat_ns BIGINT,
    version TEXT,
    provisioner_type TEXT,
    capabilities TEXT,
    active_contracts BIGINT DEFAULT 0,
    updated_at_ns BIGINT NOT NULL,
    resources JSONB
);

CREATE INDEX idx_agent_status_provider ON provider_agent_status(provider_pubkey);

-- Migrate existing data: for legacy rows, set agent_pubkey = provider_pubkey
-- (best approximation for pre-migration data where agent keys were unknown)
INSERT INTO provider_agent_status
    (agent_pubkey, provider_pubkey, online, last_heartbeat_ns, version,
     provisioner_type, capabilities, active_contracts, updated_at_ns, resources)
SELECT
    provider_pubkey, provider_pubkey, online, last_heartbeat_ns, version,
    provisioner_type, capabilities, COALESCE(active_contracts, 0), updated_at_ns, resources
FROM provider_agent_status_backup;

-- Drop backup table after successful migration
DROP TABLE provider_agent_status_backup;
