-- Receipt tracking for payment receipts
-- Sequential numbering for tax compliance

-- Create receipt_sequence table (single row, holds next receipt number)
CREATE TABLE receipt_sequence (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    next_number INTEGER NOT NULL DEFAULT 1
);

-- Initialize with first receipt number
INSERT INTO receipt_sequence (id, next_number) VALUES (1, 1);

-- Add receipt columns to contract_sign_requests
ALTER TABLE contract_sign_requests ADD COLUMN receipt_number INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN receipt_sent_at_ns INTEGER;

-- Create index on receipt_number for lookups
CREATE INDEX idx_contract_sign_requests_receipt_number ON contract_sign_requests(receipt_number) WHERE receipt_number IS NOT NULL;
