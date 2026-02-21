-- Per-provider uptime SLA configuration
ALTER TABLE provider_sla_config
    ADD COLUMN IF NOT EXISTS uptime_threshold_percent INTEGER NOT NULL DEFAULT 95,
    ADD COLUMN IF NOT EXISTS sla_alert_window_hours INTEGER NOT NULL DEFAULT 24;

-- Track when last alert was sent per contract (rate limiting: max 1 per hour per contract)
CREATE TABLE sla_breach_alerts (
    contract_id BYTEA NOT NULL,
    provider_pubkey BYTEA NOT NULL,
    uptime_percent INTEGER NOT NULL,
    threshold_percent INTEGER NOT NULL,
    alert_sent_at BIGINT NOT NULL,
    PRIMARY KEY (contract_id)
);
