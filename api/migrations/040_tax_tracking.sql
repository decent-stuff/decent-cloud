-- Add tax tracking columns to contract_sign_requests for Stripe Tax integration
ALTER TABLE contract_sign_requests ADD COLUMN tax_amount_e9s INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN tax_rate_percent REAL;
ALTER TABLE contract_sign_requests ADD COLUMN tax_type TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN tax_jurisdiction TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN customer_tax_id TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN reverse_charge INTEGER DEFAULT 0;
