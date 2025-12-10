-- Add Stripe invoice ID for invoice PDF retrieval
ALTER TABLE contract_sign_requests ADD COLUMN stripe_invoice_id TEXT;

-- Index for invoice lookups
CREATE INDEX idx_contract_sign_requests_stripe_invoice ON contract_sign_requests(stripe_invoice_id) WHERE stripe_invoice_id IS NOT NULL;
