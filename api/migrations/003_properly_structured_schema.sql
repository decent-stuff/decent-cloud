-- Properly structured schema with no JSON blobs
-- This replaces all JSON storage with proper relational tables

-- Drop all existing tables for clean start
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
DROP TABLE IF EXISTS sync_state;

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

-- Provider profiles with structured fields
CREATE TABLE provider_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    name TEXT,
    description TEXT,
    website_url TEXT,
    contact_email TEXT,
    location TEXT,
    capabilities_json TEXT, -- Store capabilities as JSON since it's complex nested data
    updated_at_ns INTEGER NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Provider offerings with structured fields
CREATE TABLE provider_offerings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    offering_id TEXT NOT NULL,
    instance_type TEXT NOT NULL,
    region TEXT,
    pricing_model TEXT NOT NULL,
    price_per_hour_e9s INTEGER,
    price_per_day_e9s INTEGER,
    min_contract_hours INTEGER,
    max_contract_hours INTEGER,
    availability_json TEXT, -- Store availability schedule as JSON
    tags TEXT, -- Comma-separated tags
    description TEXT,
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

-- Contract sign requests with structured fields
CREATE TABLE contract_sign_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL UNIQUE,
    requester_pubkey_hash BLOB NOT NULL,
    requester_ssh_pubkey TEXT NOT NULL,
    requester_contact TEXT NOT NULL,
    provider_pubkey_hash BLOB NOT NULL,
    offering_id TEXT NOT NULL,
    region_name TEXT,
    instance_config TEXT,
    payment_amount_e9s INTEGER NOT NULL,
    start_timestamp INTEGER,
    request_memo TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    status TEXT DEFAULT 'pending' -- pending, accepted, rejected, completed
);

-- Contract payment entries (separate table for proper normalization)
CREATE TABLE contract_payment_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    pricing_model TEXT NOT NULL,
    time_period_unit TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    amount_e9s INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

-- Contract sign replies with structured fields
CREATE TABLE contract_sign_replies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    provider_pubkey_hash BLOB NOT NULL,
    reply_status TEXT NOT NULL, -- accepted, rejected
    reply_memo TEXT,
    instance_details TEXT, -- JSON for instance connection details
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
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
CREATE INDEX idx_provider_offerings_offering_id ON provider_offerings(offering_id);
CREATE INDEX idx_provider_offerings_region ON provider_offerings(region);
CREATE INDEX idx_token_transfers_from_account ON token_transfers(from_account);
CREATE INDEX idx_token_transfers_to_account ON token_transfers(to_account);
CREATE INDEX idx_token_transfers_timestamp ON token_transfers(created_at_ns);
CREATE INDEX idx_token_transfers_block_hash ON token_transfers(block_hash);
CREATE INDEX idx_token_approvals_owner_account ON token_approvals(owner_account);
CREATE INDEX idx_token_approvals_spender_account ON token_approvals(spender_account);
CREATE INDEX idx_contract_sign_requests_contract_id ON contract_sign_requests(contract_id);
CREATE INDEX idx_contract_sign_requests_requester ON contract_sign_requests(requester_pubkey_hash);
CREATE INDEX idx_contract_sign_requests_provider ON contract_sign_requests(provider_pubkey_hash);
CREATE INDEX idx_contract_sign_requests_status ON contract_sign_requests(status);
CREATE INDEX idx_contract_payment_entries_contract_id ON contract_payment_entries(contract_id);
CREATE INDEX idx_contract_sign_replies_contract_id ON contract_sign_replies(contract_id);
CREATE INDEX idx_reputation_changes_pubkey_hash ON reputation_changes(pubkey_hash);
CREATE INDEX idx_reputation_changes_timestamp ON reputation_changes(block_timestamp_ns);
CREATE INDEX idx_linked_ic_ids_pubkey_hash ON linked_ic_ids(pubkey_hash);
CREATE INDEX idx_linked_ic_ids_principal ON linked_ic_ids(ic_principal);
