-- Add last_login_at column to accounts table for activity tracking
ALTER TABLE accounts ADD COLUMN last_login_at INTEGER;

-- Create index for efficient activity queries
CREATE INDEX idx_accounts_last_login ON accounts(last_login_at);
