-- Change currency default from 'usd' to '???' for fail-fast behavior
-- SQLite doesn't support ALTER COLUMN, so we recreate the column with the new default

-- Step 1: Drop existing index (will be recreated at the end)
DROP INDEX IF EXISTS idx_contract_currency;

-- Step 2: Add new column with correct default
ALTER TABLE contract_sign_requests ADD COLUMN currency_new TEXT NOT NULL DEFAULT '???';

-- Step 3: Copy existing data from old column to new column
UPDATE contract_sign_requests SET currency_new = currency;

-- Step 4: Drop old column
ALTER TABLE contract_sign_requests DROP COLUMN currency;

-- Step 5: Rename new column to currency
ALTER TABLE contract_sign_requests RENAME COLUMN currency_new TO currency;

-- Step 6: Recreate index on currency for efficient filtering
CREATE INDEX idx_contract_currency ON contract_sign_requests(currency);
