-- Profile Architecture Fix
-- Migrates from pubkey-based user_profiles to account-based profiles
-- See docs/PROFILE_ARCHITECTURE_FIX.md for full specification

-- Step 1: Add profile columns to accounts table (1:1 relationship - YAGNI)
ALTER TABLE accounts ADD COLUMN display_name TEXT;
ALTER TABLE accounts ADD COLUMN bio TEXT;
ALTER TABLE accounts ADD COLUMN avatar_url TEXT;
ALTER TABLE accounts ADD COLUMN profile_updated_at INTEGER;

-- Step 2: Migrate existing data from user_profiles to accounts
-- Strategy: For each user_profiles entry, find the account that owns that pubkey
UPDATE accounts
SET
    display_name = (
        SELECT up.display_name
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    ),
    bio = (
        SELECT up.bio
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    ),
    avatar_url = (
        SELECT up.avatar_url
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    ),
    profile_updated_at = (
        SELECT up.updated_at_ns
        FROM user_profiles up
        INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
        WHERE apk.account_id = accounts.id
        LIMIT 1
    )
WHERE EXISTS (
    SELECT 1
    FROM user_profiles up
    INNER JOIN account_public_keys apk ON up.pubkey = apk.public_key
    WHERE apk.account_id = accounts.id
);

-- Step 3: Create account_contacts table (1:many relationship)
CREATE TABLE IF NOT EXISTS account_contacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    contact_type TEXT NOT NULL,
    contact_value TEXT NOT NULL,
    verified BOOLEAN DEFAULT FALSE,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

CREATE INDEX IF NOT EXISTS idx_account_contacts_account_id ON account_contacts(account_id);

-- Step 4: Create account_socials table (1:many relationship)
CREATE TABLE IF NOT EXISTS account_socials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    username TEXT NOT NULL,
    profile_url TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

CREATE INDEX IF NOT EXISTS idx_account_socials_account_id ON account_socials(account_id);

-- Step 5: Create account_external_keys table (1:many relationship)
-- External public keys (SSH/GPG), not to be confused with account_public_keys (auth keys)
CREATE TABLE IF NOT EXISTS account_external_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id BLOB NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    key_type TEXT NOT NULL,
    key_data TEXT NOT NULL,
    key_fingerprint TEXT,
    label TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

CREATE INDEX IF NOT EXISTS idx_account_external_keys_account_id ON account_external_keys(account_id);
