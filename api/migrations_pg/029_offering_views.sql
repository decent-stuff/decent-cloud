CREATE TABLE offering_views (
    id BIGSERIAL PRIMARY KEY,
    offering_id BIGINT NOT NULL,
    viewer_pubkey BYTEA,  -- null if not authenticated
    ip_hash BYTEA NOT NULL,  -- SHA-256 hash of IP + daily salt, for dedup without storing IP
    viewed_at BIGINT NOT NULL  -- Unix timestamp millis
);
CREATE INDEX idx_offering_views_offering_id ON offering_views(offering_id, viewed_at DESC);
-- Unique constraint to deduplicate: same offering + same IP hash + same day
CREATE UNIQUE INDEX idx_offering_views_dedup ON offering_views(offering_id, ip_hash, (viewed_at / 86400000));
