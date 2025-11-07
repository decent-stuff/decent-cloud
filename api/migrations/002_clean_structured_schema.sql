-- Fix schema conflicts by dropping all tables and recreating clean structured schema
-- No backward compatibility - complete fresh start

-- Drop all existing tables
DROP TABLE IF EXISTS ledger_entries;
DROP TABLE IF EXISTS sync_state;
DROP TABLE IF EXISTS provider_registrations;
DROP TABLE IF EXISTS provider_check_ins;
DROP TABLE IF EXISTS provider_profiles;
DROP TABLE IF EXISTS provider_offerings;
DROP TABLE IF EXISTS token_transfers;
DROP TABLE IF EXISTS token_approvals;
DROP TABLE IF EXISTS user_registrations;
DROP TABLE IF EXISTS contract_sign_requests;
DROP TABLE IF EXISTS contract_sign_replies;
DROP TABLE IF EXISTS reputation_changes;
DROP TABLE IF EXISTS reputation_aging;
DROP TABLE IF EXISTS reward_distributions;
DROP TABLE IF EXISTS linked_ic_ids;

-- Provider registrations (stores public key and registration signature)
CREATE TABLE provider_registrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    pubkey_bytes BLOB NOT NULL,
    signature BLOB NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider check-ins with memo and nonce signature
CREATE TABLE provider_check_ins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    memo TEXT NOT NULL,
    nonce_signature BLOB NOT NULL,
    block_timestamp_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider profiles with JSON metadata
CREATE TABLE provider_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    profile_json TEXT NOT NULL,
    updated_at_ns INTEGER NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider offerings with JSON metadata
CREATE TABLE provider_offerings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    offering_json TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- User registrations
CREATE TABLE user_registrations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    pubkey_bytes BLOB NOT NULL,
    signature BLOB NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Token transfers with full details
CREATE TABLE token_transfers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_account TEXT NOT NULL,
    to_account TEXT NOT NULL,
    amount_e9s INTEGER NOT NULL,
    fee_e9s INTEGER NOT NULL DEFAULT 0,
    memo TEXT,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    block_hash BLOB,
    block_offset INTEGER
);

-- Token approvals
CREATE TABLE token_approvals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_account TEXT NOT NULL,
    spender_account TEXT NOT NULL,
    amount_e9s INTEGER NOT NULL,
    expires_at_ns INTEGER,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Contract sign requests
CREATE TABLE contract_sign_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    contract_json TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Contract sign replies
CREATE TABLE contract_sign_replies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id INTEGER NOT NULL,
    pubkey_hash BLOB NOT NULL,
    reply_json TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Reputation changes with details
CREATE TABLE reputation_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    change_amount INTEGER NOT NULL,
    reason TEXT NOT NULL,
    block_timestamp_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Reputation aging records
CREATE TABLE reputation_aging (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_timestamp_ns INTEGER NOT NULL,
    aging_factor_ppm INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Reward distributions
CREATE TABLE reward_distributions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_timestamp_ns INTEGER NOT NULL,
    total_amount_e9s INTEGER NOT NULL,
    providers_count INTEGER NOT NULL,
    amount_per_provider_e9s INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Linked IC identities
CREATE TABLE linked_ic_ids (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    ic_principal TEXT NOT NULL,
    linked_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Sync state tracking
CREATE TABLE sync_state (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    last_position INTEGER NOT NULL DEFAULT 0,
    last_sync_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Insert initial sync state
INSERT OR IGNORE INTO sync_state (id, last_position) VALUES (1, 0);

-- Optimized indexes for efficient querying
CREATE INDEX idx_provider_registrations_pubkey_hash ON provider_registrations(pubkey_hash);
CREATE INDEX idx_provider_check_ins_pubkey_hash ON provider_check_ins(pubkey_hash);
CREATE INDEX idx_provider_check_ins_timestamp ON provider_check_ins(block_timestamp_ns);
CREATE INDEX idx_provider_profiles_pubkey_hash ON provider_profiles(pubkey_hash);
CREATE INDEX idx_provider_offerings_pubkey_hash ON provider_offerings(pubkey_hash);
CREATE INDEX idx_token_transfers_from_account ON token_transfers(from_account);
CREATE INDEX idx_token_transfers_to_account ON token_transfers(to_account);
CREATE INDEX idx_token_transfers_timestamp ON token_transfers(created_at_ns);
CREATE INDEX idx_token_transfers_block_hash ON token_transfers(block_hash);
CREATE INDEX idx_token_approvals_owner_account ON token_approvals(owner_account);
CREATE INDEX idx_token_approvals_spender_account ON token_approvals(spender_account);
CREATE INDEX idx_reputation_changes_pubkey_hash ON reputation_changes(pubkey_hash);
CREATE INDEX idx_reputation_changes_timestamp ON reputation_changes(block_timestamp_ns);
CREATE INDEX idx_contract_sign_requests_pubkey_hash ON contract_sign_requests(pubkey_hash);
CREATE INDEX idx_linked_ic_ids_pubkey_hash ON linked_ic_ids(pubkey_hash);
CREATE INDEX idx_linked_ic_ids_principal ON linked_ic_ids(ic_principal);
