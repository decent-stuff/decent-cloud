-- Fix existing contracts with wrong currency
-- This migration updates contracts that have incorrect currency (usd or ???)
-- by looking up the correct currency from their associated offering

-- Update contracts to use the currency from their offering
UPDATE contract_sign_requests
SET currency = (
    SELECT po.currency
    FROM provider_offerings po
    WHERE po.offering_id = contract_sign_requests.offering_id
    LIMIT 1
)
WHERE currency IN ('usd', '???')
AND EXISTS (
    SELECT 1
    FROM provider_offerings po
    WHERE po.offering_id = contract_sign_requests.offering_id
);
