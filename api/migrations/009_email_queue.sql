-- Email Queue and Account Recovery Migration
-- Stores emails for reliable delivery with retry logic

CREATE TABLE email_queue (
    id BLOB PRIMARY KEY NOT NULL,
    to_addr TEXT NOT NULL,       -- RFC 2822 format: "Name <email@example.com>" or multiple
    from_addr TEXT NOT NULL,     -- RFC 2822 format: "Name <email@example.com>"
    subject TEXT NOT NULL,
    body TEXT NOT NULL,
    is_html INTEGER NOT NULL DEFAULT 0,
    email_type TEXT NOT NULL DEFAULT 'general', -- 'recovery', 'welcome', 'general'
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending', 'sent', 'failed'
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 6,  -- Default for general emails, overridden per type
    last_error TEXT,
    created_at INTEGER NOT NULL,
    last_attempted_at INTEGER,
    sent_at INTEGER
);

CREATE INDEX idx_email_queue_status ON email_queue(status);
CREATE INDEX idx_email_queue_created_at ON email_queue(created_at);
CREATE INDEX idx_email_queue_type_status ON email_queue(email_type, status);

-- Account Recovery Tokens
-- Stores one-time tokens for account recovery via email
CREATE TABLE recovery_tokens (
    token BLOB PRIMARY KEY NOT NULL,
    account_id BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    used_at INTEGER,
    FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
);

CREATE INDEX idx_recovery_tokens_account_id ON recovery_tokens(account_id);
CREATE INDEX idx_recovery_tokens_expires_at ON recovery_tokens(expires_at);
