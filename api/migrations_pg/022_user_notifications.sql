CREATE TABLE user_notifications (
    id BIGSERIAL PRIMARY KEY,
    user_pubkey BYTEA NOT NULL,
    type TEXT NOT NULL,
    title TEXT NOT NULL,
    body TEXT NOT NULL,
    contract_id TEXT,
    read_at BIGINT,
    created_at BIGINT NOT NULL
);
CREATE INDEX idx_user_notifications_pubkey ON user_notifications(user_pubkey, created_at DESC);
CREATE INDEX idx_user_notifications_unread ON user_notifications(user_pubkey, read_at) WHERE read_at IS NULL;
