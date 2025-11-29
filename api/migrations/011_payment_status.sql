-- Payment Status Tracking Migration
-- Adds payment_status field to track the lifecycle of contract payments

-- Add payment_status field with default 'pending'
ALTER TABLE contract_sign_requests ADD COLUMN payment_status TEXT NOT NULL DEFAULT 'pending';

-- Update existing Stripe contracts to 'pending' (they need webhook confirmation)
UPDATE contract_sign_requests SET payment_status = 'pending' WHERE payment_method = 'stripe';

-- Update existing DCT contracts to 'succeeded' (they are pre-paid via DCT)
UPDATE contract_sign_requests SET payment_status = 'succeeded' WHERE payment_method = 'dct';

-- Create index on payment_status for efficient queries
CREATE INDEX idx_contract_sign_requests_payment_status ON contract_sign_requests(payment_status);

-- Create composite index for filtering by payment method and status
CREATE INDEX idx_contract_sign_requests_payment_method_status ON contract_sign_requests(payment_method, payment_status);
