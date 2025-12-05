-- ICPay escrow and payment release tracking

-- Payment releases table for periodic provider payments
CREATE TABLE IF NOT EXISTS payment_releases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    release_type TEXT NOT NULL CHECK(release_type IN ('daily', 'hourly', 'final', 'cancellation')),
    period_start_ns INTEGER NOT NULL,
    period_end_ns INTEGER NOT NULL,
    amount_e9s INTEGER NOT NULL,
    provider_pubkey BLOB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'released', 'paid_out', 'refunded')),
    created_at_ns INTEGER NOT NULL,
    released_at_ns INTEGER,
    payout_id TEXT,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

CREATE INDEX IF NOT EXISTS idx_payment_releases_contract ON payment_releases(contract_id);
CREATE INDEX IF NOT EXISTS idx_payment_releases_provider ON payment_releases(provider_pubkey);
CREATE INDEX IF NOT EXISTS idx_payment_releases_status ON payment_releases(status);

-- Extend contract_sign_requests with ICPay tracking fields
ALTER TABLE contract_sign_requests ADD COLUMN icpay_payment_id TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN icpay_refund_id TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN total_released_e9s INTEGER DEFAULT 0;
ALTER TABLE contract_sign_requests ADD COLUMN last_release_at_ns INTEGER;
