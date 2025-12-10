-- Pending Stripe receipts: tracks contracts awaiting invoice before sending receipt
-- The background processor will retry until Stripe invoice is available or max attempts reached

CREATE TABLE pending_stripe_receipts (
    contract_id BLOB PRIMARY KEY,
    created_at_ns INTEGER NOT NULL,
    next_attempt_at_ns INTEGER NOT NULL,
    attempts INTEGER NOT NULL DEFAULT 0
);

-- Index for finding receipts ready to process
CREATE INDEX idx_pending_stripe_receipts_next_attempt ON pending_stripe_receipts(next_attempt_at_ns);
