-- Add refund tracking fields to contract_sign_requests
-- This migration adds support for tracking Stripe refunds on cancelled contracts

-- Add refund amount field
ALTER TABLE contract_sign_requests ADD COLUMN refund_amount_e9s INTEGER DEFAULT NULL;

-- Add Stripe refund ID for tracking refund status
ALTER TABLE contract_sign_requests ADD COLUMN stripe_refund_id TEXT DEFAULT NULL;

-- Add timestamp when refund was created
ALTER TABLE contract_sign_requests ADD COLUMN refund_created_at_ns INTEGER DEFAULT NULL;

-- Create index on refund_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_contract_refund_id ON contract_sign_requests(stripe_refund_id);
