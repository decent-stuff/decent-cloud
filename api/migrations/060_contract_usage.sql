-- Contract usage tracking for usage-based billing
-- Tracks accumulated usage per billing period for each contract

CREATE TABLE contract_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,

    -- Billing period (per-contract, starts at contract creation)
    billing_period_start INTEGER NOT NULL,  -- Unix timestamp (seconds)
    billing_period_end INTEGER NOT NULL,    -- Unix timestamp (seconds)

    -- Usage tracking
    units_used REAL NOT NULL DEFAULT 0,     -- Accumulated usage in billing_unit
    units_included REAL,                    -- Snapshot of included_units from offering

    -- Calculated fields (updated on usage report)
    overage_units REAL NOT NULL DEFAULT 0,  -- max(0, units_used - units_included)
    estimated_charge_cents INTEGER,         -- Estimated charge based on current usage

    -- Stripe integration
    reported_to_stripe INTEGER NOT NULL DEFAULT 0,  -- Boolean: usage reported
    stripe_usage_record_id TEXT,                    -- Stripe usage record ID after reporting

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),

    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

-- Index for finding usage by contract
CREATE INDEX idx_contract_usage_contract ON contract_usage(contract_id);

-- Index for finding unreported usage (for billing job)
CREATE INDEX idx_contract_usage_unreported ON contract_usage(reported_to_stripe, billing_period_end);

-- Usage events log for audit trail and heartbeat-based calculation
CREATE TABLE contract_usage_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,

    -- Event details
    event_type TEXT NOT NULL,       -- 'heartbeat', 'start', 'stop', 'manual_report'
    units_delta REAL,               -- Change in usage (can be NULL for heartbeat)

    -- For heartbeat-based calculation
    heartbeat_at INTEGER,           -- When heartbeat was received

    -- Metadata
    source TEXT,                    -- 'dc-agent', 'api', 'system'
    metadata TEXT,                  -- JSON for additional context

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),

    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

CREATE INDEX idx_contract_usage_events_contract ON contract_usage_events(contract_id);
CREATE INDEX idx_contract_usage_events_type ON contract_usage_events(event_type, created_at);
