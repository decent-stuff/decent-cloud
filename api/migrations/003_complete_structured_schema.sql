-- Complete structured schema replacing generic key-value storage
-- This migration creates proper relational tables for all ledger entry types

-- Drop existing generic tables (no backward compatibility needed)
DROP TABLE IF EXISTS ledger_entries;
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
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Provider profiles with JSON metadata
CREATE TABLE provider_profiles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL UNIQUE,
    profile_json TEXT NOT NULL,
    updated_at_ns INTEGER NOT NULL,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Provider offerings with JSON metadata
CREATE TABLE provider_offerings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    offering_json TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
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
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Contract sign replies
CREATE TABLE contract_sign_replies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_id INTEGER NOT NULL,
    pubkey_hash BLOB NOT NULL,
    reply_json TEXT NOT NULL,
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (request_id) REFERENCES contract_sign_requests(id),
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
);

-- Reputation changes with details
CREATE TABLE reputation_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pubkey_hash BLOB NOT NULL,
    change_amount INTEGER NOT NULL,
    reason TEXT NOT NULL,
    block_timestamp_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (pubkey_hash) REFERENCES provider_registrations(pubkey_hash)
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

-- Views for common queries
CREATE VIEW provider_summary AS
SELECT 
    pr.pubkey_hash,
    pr.created_at_ns as registration_time,
    pp.updated_at_ns as last_profile_update,
    pp.profile_json,
    COUNT(pci.id) as check_in_count,
    MAX(pci.block_timestamp_ns) as last_check_in_time,
    COUNT(po.id) as offering_count
FROM provider_registrations pr
LEFT JOIN provider_profiles pp ON pr.pubkey_hash = pp.pubkey_hash
LEFT JOIN provider_check_ins pci ON pr.pubkey_hash = pci.pubkey_hash
LEFT JOIN provider_offerings po ON pr.pubkey_hash = po.pubkey_hash
GROUP BY pr.pubkey_hash, pr.created_at_ns, pp.updated_at_ns, pp.profile_json;

CREATE VIEW token_transfers_summary AS
SELECT 
    DATE(created_at, 'unixepoch') as transfer_date,
    COUNT(*) as transfer_count,
    SUM(amount_e9s) as total_transferred,
    SUM(fee_e9s) as total_fees
FROM token_transfers
GROUP BY DATE(created_at, 'unixepoch')
ORDER BY transfer_date DESC;

CREATE VIEW reputation_summary AS
SELECT 
    rc.pubkey_hash,
    SUM(rc.change_amount) as net_reputation,
    COUNT(*) as change_count,
    MAX(rc.block_timestamp_ns) as last_change_time
FROM reputation_changes rc
GROUP BY rc.pubkey_hash
ORDER BY net_reputation DESC;
