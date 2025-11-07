-- Structured tables for different ledger entry types
-- This replaces the generic key-value storage with proper relational structure

-- Provider registrations
CREATE TABLE IF NOT EXISTS provider_registrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    pubkey_bytes BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    created_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider check-ins
CREATE TABLE IF NOT EXISTS provider_check_ins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    memo TEXT,
    nonce_signature BLOB NOT NULL,
    block_timestamp INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Provider profiles
CREATE TABLE IF NOT EXISTS provider_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    profile_data BLOB NOT NULL,
    updated_at INTEGER NOT NULL,
    updated_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Provider offerings
CREATE TABLE IF NOT EXISTS provider_offerings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    offering_data BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    created_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Token transfers
CREATE TABLE IF NOT EXISTS token_transfers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_pubkey_hash BLOB,
    to_pubkey_hash BLOB,
    from_account TEXT NOT NULL,
    to_account TEXT NOT NULL,
    amount_e9s INTEGER NOT NULL,
    fee_e9s INTEGER NOT NULL DEFAULT 0,
    memo TEXT,
    created_at INTEGER NOT NULL,
    created_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Token approvals
CREATE TABLE IF NOT EXISTS token_approvals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_pubkey_hash BLOB,
    spender_account TEXT NOT NULL,
    amount_e9s INTEGER NOT NULL,
    expires_at INTEGER,
    created_at INTEGER NOT NULL,
    created_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Reward distributions
CREATE TABLE IF NOT EXISTS reward_distributions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_timestamp INTEGER NOT NULL,
    total_amount_e9s INTEGER NOT NULL,
    providers_count INTEGER NOT NULL,
    amount_per_provider_e9s INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- User registrations
CREATE TABLE IF NOT EXISTS user_registrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    pubkey_bytes BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    created_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Contract sign requests
CREATE TABLE IF NOT EXISTS contract_sign_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    contract_data BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    created_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Contract sign replies
CREATE TABLE IF NOT EXISTS contract_sign_replies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id INTEGER NOT NULL,
    pubkey_hash BLOB NOT NULL,
    reply_data BLOB NOT NULL,
    created_at INTEGER NOT NULL,
    created_at_datetime DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (request_id) REFERENCES contract_sign_requests(id),
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Reputation changes
CREATE TABLE IF NOT EXISTS reputation_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    change_amount INTEGER NOT NULL,
    reason TEXT,
    block_timestamp INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Reputation aging records
CREATE TABLE IF NOT EXISTS reputation_aging (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_timestamp INTEGER NOT NULL,
    aging_factor INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for efficient querying
CREATE INDEX idx_provider_check_ins_pubkey_hash ON provider_check_ins(pubkey_hash);
CREATE INDEX idx_provider_check_ins_timestamp ON provider_check_ins(block_timestamp);
CREATE INDEX idx_token_transfers_from_account ON token_transfers(from_account);
CREATE INDEX idx_token_transfers_to_account ON token_transfers(to_account);
CREATE INDEX idx_token_transfers_timestamp ON token_transfers(created_at);
CREATE INDEX idx_reputation_changes_pubkey_hash ON reputation_changes(pubkey_hash);
CREATE INDEX idx_reputation_changes_timestamp ON reputation_changes(block_timestamp);
CREATE INDEX idx_contract_sign_requests_pubkey_hash ON contract_sign_requests(pubkey_hash);
CREATE INDEX idx_provider_offerings_pubkey_hash ON provider_offerings(pubkey_hash);

-- Keep the original ledger_entries table as a fallback/archive
-- but mark entries as migrated once they're moved to structured tables
ALTER TABLE ledger_entries ADD COLUMN migrated INTEGER DEFAULT 0;
CREATE INDEX idx_ledger_entries_migrated ON ledger_entries(migrated);
