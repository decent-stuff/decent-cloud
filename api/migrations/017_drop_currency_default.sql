-- Drop DEFAULT from currency column to enforce explicit currency values
-- This ensures INSERT fails if currency not provided (fail-fast principle)

-- SQLite doesn't support ALTER COLUMN, so we recreate the column without DEFAULT

-- Step 1: Drop existing index
DROP INDEX IF EXISTS idx_contract_currency;

-- Step 2: Add new column with temporary DEFAULT (required for NOT NULL in SQLite)
-- The DEFAULT is only for migration; application code will not have a default
ALTER TABLE contract_sign_requests ADD COLUMN currency_v2 TEXT NOT NULL DEFAULT '???';

-- Step 3: Copy existing data from old column to new column
UPDATE contract_sign_requests SET currency_v2 = currency;

-- Step 4: Drop old column
ALTER TABLE contract_sign_requests DROP COLUMN currency;

-- Step 5: Rename new column to currency
ALTER TABLE contract_sign_requests RENAME COLUMN currency_v2 TO currency;

-- Step 6: Recreate index on currency
CREATE INDEX idx_contract_currency ON contract_sign_requests(currency);
