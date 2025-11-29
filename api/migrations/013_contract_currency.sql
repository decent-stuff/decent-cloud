-- Add currency field to contract_sign_requests
-- This migration adds the currency field to track what currency the payment_amount_e9s is denominated in

-- Add currency field (default to '???' to make missing currency obvious - fail fast principle)
ALTER TABLE contract_sign_requests ADD COLUMN currency TEXT NOT NULL DEFAULT '???';

-- Create index on currency for efficient filtering
CREATE INDEX IF NOT EXISTS idx_contract_currency ON contract_sign_requests(currency);
