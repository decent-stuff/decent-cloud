-- OAuth Support Migration
-- Adds Google OAuth authentication alongside existing seed phrase auth

-- Add auth provider field to accounts table
ALTER TABLE accounts ADD COLUMN auth_provider TEXT NOT NULL DEFAULT 'seed_phrase';
CREATE INDEX idx_accounts_auth_provider ON accounts(auth_provider);

-- OAuth accounts table - links external OAuth provider IDs to Decent Cloud accounts
CREATE TABLE oauth_accounts (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    external_id TEXT NOT NULL,
    email TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    UNIQUE(provider, external_id),
    CHECK (provider IN ('google_oauth'))
);

CREATE INDEX idx_oauth_accounts_account ON oauth_accounts(account_id);
CREATE INDEX idx_oauth_accounts_provider_external ON oauth_accounts(provider, external_id);
CREATE INDEX idx_oauth_accounts_email ON oauth_accounts(email);

-- Tower-sessions table for session management
-- Session data is stored as JSON blob containing Ed25519 keypair for OAuth users
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY NOT NULL,
    data BLOB NOT NULL,
    expiry_date INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_expiry ON sessions(expiry_date);
