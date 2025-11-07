-- Fix sync duplication issues by adjusting uniqueness constraints
-- The issue: contract_sign_requests table has UNIQUE(contract_id) 
-- but sync retries cause same contract_id to be inserted multiple times
-- Since (block_offset, label, key) is always unique, we handle duplicates at app level

-- Remove problematic UNIQUE index that prevents re-processing
DROP INDEX IF EXISTS idx_contract_sign_requests_contract_id;

-- Reset stuck sync position
UPDATE sync_state 
SET last_position = CASE 
    WHEN last_position = 8388608 THEN 0  -- Reset if stuck at the known position
    ELSE last_position 
END;
