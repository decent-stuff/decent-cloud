CREATE TABLE provider_offering_sla_targets (
    offering_id BIGINT PRIMARY KEY REFERENCES provider_offerings(id) ON DELETE CASCADE,
    provider_pubkey BYTEA NOT NULL REFERENCES provider_registrations(pubkey) ON DELETE CASCADE,
    sla_target_percent DOUBLE PRECISION NOT NULL,
    updated_at_ns BIGINT NOT NULL,
    CHECK (sla_target_percent >= 1 AND sla_target_percent <= 100)
);

CREATE INDEX provider_offering_sla_targets_provider_idx
    ON provider_offering_sla_targets(provider_pubkey);

CREATE TABLE provider_offering_sli_reports (
    offering_id BIGINT NOT NULL REFERENCES provider_offerings(id) ON DELETE CASCADE,
    provider_pubkey BYTEA NOT NULL REFERENCES provider_registrations(pubkey) ON DELETE CASCADE,
    report_date DATE NOT NULL,
    uptime_percent DOUBLE PRECISION NOT NULL,
    response_sli_percent DOUBLE PRECISION,
    incident_count INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    created_at_ns BIGINT NOT NULL,
    updated_at_ns BIGINT NOT NULL,
    PRIMARY KEY (offering_id, report_date),
    CHECK (uptime_percent >= 0 AND uptime_percent <= 100),
    CHECK (response_sli_percent IS NULL OR (response_sli_percent >= 0 AND response_sli_percent <= 100)),
    CHECK (incident_count >= 0)
);

CREATE INDEX provider_offering_sli_reports_provider_idx
    ON provider_offering_sli_reports(provider_pubkey, report_date DESC);
