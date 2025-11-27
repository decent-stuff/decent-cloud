-- Allow uppercase letters in usernames
-- Update CHECK constraint to accept both uppercase and lowercase letters
-- Add case-insensitive unique constraint (preserve case but prevent duplicates like "Alice" and "alice")

-- SQLite doesn't support altering CHECK constraints directly, so we need to recreate the table
-- Must disable foreign keys temporarily to allow table recreation

PRAGMA foreign_keys = OFF;

-- Create new accounts table with updated constraint
-- Note: Removed UNIQUE from username column, will use case-insensitive index instead
CREATE TABLE accounts_new (
    id BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    username TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    auth_provider TEXT,
    email TEXT,
    display_name TEXT,
    bio TEXT,
    avatar_url TEXT,
    profile_updated_at INTEGER,
    CHECK (
        username GLOB '[a-zA-Z0-9][a-zA-Z0-9._@-]*[a-zA-Z0-9]'
        AND length(username) >= 3
        AND length(username) <= 64
    )
);

-- Copy data from old table (preserve all columns from oauth migration)
INSERT INTO accounts_new SELECT * FROM accounts;

-- Drop old table
DROP TABLE accounts;

-- Rename new table
ALTER TABLE accounts_new RENAME TO accounts;

-- Create case-insensitive unique index on username (stores original case, enforces uniqueness case-insensitively)
CREATE UNIQUE INDEX idx_accounts_username_unique ON accounts(LOWER(username));

-- Create regular index for lookups
CREATE INDEX idx_accounts_username ON accounts(username);

PRAGMA foreign_keys = ON;
