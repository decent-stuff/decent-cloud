-- Fix foreign key constraint for contract_sign_replies
-- Add UNIQUE constraint to contract_id in contract_sign_requests table

-- First, let's check if there are any duplicate contract_ids
-- and handle them appropriately (the newer one should be kept)

-- Create a temporary table with unique contract_ids
CREATE TABLE contract_sign_requests_unique AS
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
GROUP BY contract_id;

-- Delete all records from the original table
DELETE FROM contract_sign_requests;

-- Re-insert the unique records with correct IDs
INSERT INTO contract_sign_requests (
    id, contract_id, requester_pubkey_hash, requester_ssh_pubkey, 
    requester_contact, provider_pubkey_hash, offering_id, region_name,
    instance_config, payment_amount_e9s, start_timestamp, request_memo,
    created_at_ns, created_at, status
)
SELECT 
    id, contract_id, requester_pubkey_hash, requester_ssh_pubkey,
    requester_contact, provider_pubkey_hash, offering_id, region_name,
    instance_config, payment_amount_e9s, start_timestamp, request_memo,
    created_at_ns, created_at, status
FROM contract_sign_requests_unique
ORDER BY id;

-- Drop the temporary table
DROP TABLE contract_sign_requests_unique;

-- Add UNIQUE constraint to contract_id
CREATE UNIQUE INDEX idx_contract_sign_requests_contract_id_unique ON contract_sign_requests(contract_id);
