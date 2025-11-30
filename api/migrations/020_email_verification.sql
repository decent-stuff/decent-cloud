-- Email Verification Migration
-- Add email verification tracking to accounts and create verification tokens table

-- Add email_verified column to accounts table
ALTER TABLE accounts ADD COLUMN email_verified INTEGER NOT NULL DEFAULT 0;

-- Email Verification Tokens
-- Stores one-time tokens for email verification
CREATE TABLE email_verification_tokens (
    token BLOB PRIMARY KEY NOT NULL,
    account_id BLOB NOT NULL,
    email TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    used_at INTEGER,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE INDEX idx_email_verification_tokens_account_id ON email_verification_tokens(account_id);
CREATE INDEX idx_email_verification_tokens_expires_at ON email_verification_tokens(expires_at);
CREATE INDEX idx_email_verification_tokens_email ON email_verification_tokens(email);
