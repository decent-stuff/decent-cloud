-- SLA tracking columns for chatwoot_message_events
ALTER TABLE chatwoot_message_events ADD COLUMN sla_breached INTEGER NOT NULL DEFAULT 0;
ALTER TABLE chatwoot_message_events ADD COLUMN sla_alert_sent INTEGER NOT NULL DEFAULT 0;

-- Provider SLA configuration
CREATE TABLE IF NOT EXISTS provider_sla_config (
    provider_pubkey BLOB PRIMARY KEY,
    response_time_seconds INTEGER NOT NULL DEFAULT 14400,  -- 4 hours default
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
