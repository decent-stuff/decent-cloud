-- Fix foreign key constraints for contract_sign_replies and contract_payment_entries
-- Only fix the contract_sign_requests table to have UNIQUE constraint on contract_id
-- This resolves the foreign key mismatch error

PRAGMA foreign_keys = OFF;

-- Drop the problematic tables first
DROP TABLE IF EXISTS contract_sign_replies;
DROP TABLE IF EXISTS contract_payment_entries;
DROP TABLE IF EXISTS contract_sign_requests;

-- Recreate contract_sign_requests with UNIQUE constraint on contract_id
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
    status TEXT DEFAULT 'pending' -- pending, accepted, rejected, completed, cancelled
);

-- Recreate contract_payment_entries with proper foreign key
CREATE TABLE contract_payment_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    pricing_model TEXT NOT NULL, -- on_demand, reserved, spot
    time_period_unit TEXT NOT NULL, -- hour, day, month, year
    quantity INTEGER NOT NULL,
    amount_e9s INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE
);

-- Recreate contract_sign_replies with proper foreign key
CREATE TABLE contract_sign_replies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    provider_pubkey_hash BLOB NOT NULL,
    reply_status TEXT NOT NULL, -- accepted, rejected
    reply_memo TEXT,
    instance_details TEXT, -- connection details, IP addresses, etc.
    created_at_ns INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE
);

-- Recreate indexes
CREATE INDEX idx_contract_sign_requests_contract_id ON contract_sign_requests(contract_id);
CREATE INDEX idx_contract_sign_requests_requester_pubkey_hash ON contract_sign_requests(requester_pubkey_hash);
CREATE INDEX idx_contract_sign_requests_provider_pubkey_hash ON contract_sign_requests(provider_pubkey_hash);
CREATE INDEX idx_contract_sign_requests_status ON contract_sign_requests(status);
CREATE INDEX idx_contract_payment_entries_contract_id ON contract_payment_entries(contract_id);
CREATE INDEX idx_contract_sign_replies_contract_id ON contract_sign_replies(contract_id);

PRAGMA foreign_keys = ON;
