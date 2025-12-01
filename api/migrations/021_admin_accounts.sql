-- Admin Accounts Migration
-- Add is_admin flag to accounts table for admin access control

ALTER TABLE accounts ADD COLUMN is_admin INTEGER NOT NULL DEFAULT 0;

-- Create index for efficient admin queries
CREATE INDEX idx_accounts_is_admin ON accounts(is_admin);
