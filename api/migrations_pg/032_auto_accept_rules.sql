CREATE TABLE auto_accept_rules (
    id BIGSERIAL PRIMARY KEY,
    provider_pubkey BYTEA NOT NULL REFERENCES provider_profiles(pubkey) ON DELETE CASCADE,
    offering_id TEXT NOT NULL,
    min_duration_hours BIGINT,
    max_duration_hours BIGINT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(provider_pubkey, offering_id)
);
