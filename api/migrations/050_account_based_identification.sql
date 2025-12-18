-- Migration: Account-Based User Identification
-- This migration links providers, offerings, and contracts to accounts (not just pubkeys).
-- This enables:
--   1. Username-based URLs instead of pubkey-based
--   2. Multi-device support (all keys under same account see same data)
--   3. Proper account-centric data model

-- Step 1: Add account_id to provider_profiles
ALTER TABLE provider_profiles ADD COLUMN account_id BLOB REFERENCES accounts(id);
CREATE INDEX idx_provider_profiles_account ON provider_profiles(account_id);

-- Step 2: Add account_id to provider_offerings
ALTER TABLE provider_offerings ADD COLUMN account_id BLOB REFERENCES accounts(id);
CREATE INDEX idx_provider_offerings_account ON provider_offerings(account_id);

-- Step 3: Add account_id columns to contracts
ALTER TABLE contract_sign_requests ADD COLUMN requester_account_id BLOB REFERENCES accounts(id);
ALTER TABLE contract_sign_requests ADD COLUMN provider_account_id BLOB REFERENCES accounts(id);
CREATE INDEX idx_contracts_requester_account ON contract_sign_requests(requester_account_id);
CREATE INDEX idx_contracts_provider_account ON contract_sign_requests(provider_account_id);

-- Note: Data backfill will be handled by the application on startup.
-- The migration script runs in "offline" mode during sqlx prepare, so we can't
-- execute complex UPDATE queries that depend on runtime data.
-- See Database::backfill_account_ids() for the backfill logic.
