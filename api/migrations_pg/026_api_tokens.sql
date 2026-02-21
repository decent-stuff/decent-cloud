CREATE TABLE api_tokens (
    id UUID NOT NULL PRIMARY KEY DEFAULT gen_random_uuid(),
    user_pubkey BYTEA NOT NULL,
    name TEXT NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE,  -- SHA-256 hash of the raw token
    created_at BIGINT NOT NULL,
    last_used_at BIGINT,
    expires_at BIGINT,  -- NULL = never expires
    revoked_at BIGINT   -- NULL = active
);
CREATE INDEX api_tokens_user_pubkey_idx ON api_tokens(user_pubkey);
