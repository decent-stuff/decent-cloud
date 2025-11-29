-- Payment Methods Migration
-- Adds payment method tracking and Stripe integration support to contracts

-- Add payment method field (defaults to 'dct' for existing contracts)
ALTER TABLE contract_sign_requests ADD COLUMN payment_method TEXT NOT NULL DEFAULT 'dct';

-- Add Stripe payment tracking fields (nullable for DCT payments)
ALTER TABLE contract_sign_requests ADD COLUMN stripe_payment_intent_id TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN stripe_customer_id TEXT;

-- Create index on payment_method for efficient filtering
CREATE INDEX idx_contract_sign_requests_payment_method ON contract_sign_requests(payment_method);

-- Create index on stripe_payment_intent_id for payment verification lookups
CREATE INDEX idx_contract_sign_requests_stripe_payment_intent ON contract_sign_requests(stripe_payment_intent_id) WHERE stripe_payment_intent_id IS NOT NULL;
