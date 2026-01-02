-- Bandwidth usage history for VMs
-- Stores periodic bandwidth snapshots reported by dc-agents

CREATE TABLE IF NOT EXISTS bandwidth_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Contract this bandwidth belongs to
    contract_id TEXT NOT NULL,
    -- Gateway slug (6-char identifier)
    gateway_slug TEXT NOT NULL,
    -- Provider pubkey (for querying by provider)
    provider_pubkey TEXT NOT NULL,
    -- Bytes received by the VM (cumulative since VM start)
    bytes_in INTEGER NOT NULL DEFAULT 0,
    -- Bytes sent by the VM (cumulative since VM start)
    bytes_out INTEGER NOT NULL DEFAULT 0,
    -- Timestamp when this record was created (nanoseconds)
    recorded_at_ns INTEGER NOT NULL
    -- Note: No foreign key constraint on contract_id to allow independent bandwidth tracking
);

-- Index for querying bandwidth by contract
CREATE INDEX IF NOT EXISTS idx_bandwidth_history_contract ON bandwidth_history(contract_id, recorded_at_ns DESC);

-- Index for querying bandwidth by provider (for dashboard)
CREATE INDEX IF NOT EXISTS idx_bandwidth_history_provider ON bandwidth_history(provider_pubkey, recorded_at_ns DESC);

-- Index for querying bandwidth by gateway slug
CREATE INDEX IF NOT EXISTS idx_bandwidth_history_slug ON bandwidth_history(gateway_slug, recorded_at_ns DESC);
