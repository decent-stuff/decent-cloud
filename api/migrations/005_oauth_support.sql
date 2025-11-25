-- OAuth Support Migration
-- Adds Google OAuth authentication alongside existing seed phrase auth

-- Add auth provider field to accounts table
ALTER TABLE accounts ADD COLUMN auth_provider TEXT NOT NULL DEFAULT 'seed_phrase';
CREATE INDEX idx_accounts_auth_provider ON accounts(auth_provider);

-- Add email field to accounts table for account linking
-- Note: Cannot add UNIQUE constraint directly in SQLite, so we add it as nullable
-- and create a unique index on non-null values only
ALTER TABLE accounts ADD COLUMN email TEXT;
CREATE UNIQUE INDEX idx_accounts_email_unique ON accounts(email) WHERE email IS NOT NULL;

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
