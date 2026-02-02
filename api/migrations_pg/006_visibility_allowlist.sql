-- Visibility allowlist for shared offerings (Phase 2)
-- Allows providers to grant specific users access to non-public offerings

CREATE TABLE visibility_allowlist (
    id BIGSERIAL PRIMARY KEY,
    offering_id BIGINT NOT NULL REFERENCES provider_offerings(id) ON DELETE CASCADE,
    allowed_pubkey BYTEA NOT NULL,  -- Ed25519 public key of allowed user (32 bytes)
    created_at BIGINT NOT NULL DEFAULT (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT,
    -- Each pubkey can only be allowed once per offering
    UNIQUE(offering_id, allowed_pubkey)
);

-- Index for querying allowlist by offering (most common query)
CREATE INDEX idx_visibility_allowlist_offering ON visibility_allowlist(offering_id);

-- Index for querying all offerings a user has access to
CREATE INDEX idx_visibility_allowlist_pubkey ON visibility_allowlist(allowed_pubkey);
