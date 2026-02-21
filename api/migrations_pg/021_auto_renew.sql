-- Add auto-renewal opt-in to contracts
ALTER TABLE contract_sign_requests ADD COLUMN IF NOT EXISTS auto_renew BOOLEAN NOT NULL DEFAULT FALSE;
