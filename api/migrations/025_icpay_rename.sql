-- ICPay Rename Migration
-- Renames DCT payment method to ICPay and adds ICPay transaction tracking

-- Rename existing 'dct' payment method to 'icpay'
UPDATE contract_sign_requests SET payment_method = 'icpay' WHERE payment_method = 'dct';

-- Add ICPay transaction ID field for tracking ICPay-specific transactions
ALTER TABLE contract_sign_requests ADD COLUMN icpay_transaction_id TEXT;

-- Create index on icpay_transaction_id for efficient lookups
CREATE INDEX idx_contract_sign_requests_icpay_transaction ON contract_sign_requests(icpay_transaction_id) WHERE icpay_transaction_id IS NOT NULL;
