CREATE TABLE saved_offerings (
    user_pubkey BYTEA NOT NULL,
    offering_id BIGINT NOT NULL,
    saved_at BIGINT NOT NULL,
    PRIMARY KEY (user_pubkey, offering_id)
);
CREATE INDEX idx_saved_offerings_user ON saved_offerings(user_pubkey, saved_at DESC);
