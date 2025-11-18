-- Account Profiles Migration
-- Implements username-based account system with multi-key support
-- See docs/ACCOUNT_PROFILES_DESIGN.md for full specification

-- Accounts table
CREATE TABLE accounts (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    username TEXT UNIQUE NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    CHECK (
        username GLOB '[a-z0-9][a-z0-9._@-]*[a-z0-9]'
        AND length(username) >= 3
        AND length(username) <= 64
    )
);

CREATE INDEX idx_accounts_username ON accounts(username);

-- Account public keys table (1-10 keys per account for multi-device access)
CREATE TABLE account_public_keys (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    public_key BLOB UNIQUE NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    added_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    disabled_at INTEGER,
    disabled_by_key_id BLOB REFERENCES account_public_keys(id),
    CHECK (length(public_key) = 32),
    UNIQUE(account_id, public_key)
);

CREATE INDEX idx_keys_account ON account_public_keys(account_id);
CREATE INDEX idx_keys_pubkey ON account_public_keys(public_key);
CREATE INDEX idx_keys_active ON account_public_keys(account_id, is_active);

-- Signature audit trail (tracks all signed operations + replay prevention)
CREATE TABLE signature_audit (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    account_id BLOB REFERENCES accounts(id),
    action TEXT NOT NULL,
    payload TEXT NOT NULL,
    signature BLOB NOT NULL,
    public_key BLOB NOT NULL,
    timestamp INTEGER NOT NULL,
    nonce BLOB NOT NULL,
    is_admin_action INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    CHECK (length(signature) = 64),
    CHECK (length(public_key) = 32),
    CHECK (length(nonce) = 16)
);

CREATE INDEX idx_audit_nonce_time ON signature_audit(nonce, created_at);
CREATE INDEX idx_audit_account ON signature_audit(account_id);
CREATE INDEX idx_audit_created ON signature_audit(created_at);
