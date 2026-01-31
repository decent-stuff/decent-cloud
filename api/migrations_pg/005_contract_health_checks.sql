-- Contract health checks reported by dc-agent
-- Used for uptime tracking and provider reputation (Phase 6)

CREATE TABLE contract_health_checks (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL REFERENCES contract_sign_requests(contract_id),
    checked_at BIGINT NOT NULL,  -- timestamp in nanoseconds when check was performed
    status TEXT NOT NULL CHECK (status IN ('healthy', 'unhealthy', 'unknown')),
    latency_ms INTEGER,  -- optional latency measurement in milliseconds
    details TEXT,  -- optional JSON with additional diagnostic info
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
);

-- Index for querying health checks by contract
CREATE INDEX idx_contract_health_checks_contract ON contract_health_checks(contract_id);

-- Index for querying recent health checks (for uptime calculation)
CREATE INDEX idx_contract_health_checks_recent ON contract_health_checks(contract_id, checked_at DESC);

-- Index for querying health status distribution
CREATE INDEX idx_contract_health_checks_status ON contract_health_checks(status, checked_at);
