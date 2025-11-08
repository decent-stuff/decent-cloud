-- Fix foreign key constraints for contract_sign_replies and contract_payment_entries
-- SQLite doesn't support adding UNIQUE constraints to columns referenced by foreign keys
-- We need to recreate the contract_sign_requests table with the proper constraints

-- Disable foreign key constraints temporarily
PRAGMA foreign_keys = OFF;

-- Create the new contract_sign_requests table with UNIQUE constraint on contract_id
CREATE TABLE contract_sign_requests_new (
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

-- Copy data from old table, keeping only the latest record for each contract_id
INSERT INTO contract_sign_requests_new (
    id, contract_id, requester_pubkey_hash, requester_ssh_pubkey, 
    requester_contact, provider_pubkey_hash, offering_id, region_name,
    instance_config, payment_amount_e9s, start_timestamp, request_memo,
    created_at_ns, created_at, status
)
SELECT 
    MAX(id) as id,
    contract_id,
    requester_pubkey_hash,
    requester_ssh_pubkey,
    requester_contact,
    provider_pubkey_hash,
    offering_id,
    region_name,
    instance_config,
    payment_amount_e9s,
    start_timestamp,
    request_memo,
    created_at_ns,
    created_at,
    status
FROM contract_sign_requests
GROUP BY contract_id
ORDER BY MAX(id);

-- Drop the old table
DROP TABLE contract_sign_requests;

-- Rename the new table to the original name
ALTER TABLE contract_sign_requests_new RENAME TO contract_sign_requests;

-- Re-enable foreign key constraints
PRAGMA foreign_keys = ON;
