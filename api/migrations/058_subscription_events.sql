-- Subscription events audit trail
-- Records all subscription changes for debugging and analytics

CREATE TABLE subscription_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id BLOB NOT NULL REFERENCES accounts(id),
    event_type TEXT NOT NULL,               -- created, updated, deleted, payment_failed, payment_succeeded
    stripe_event_id TEXT UNIQUE,            -- For idempotency
    old_plan_id TEXT,
    new_plan_id TEXT,
    stripe_subscription_id TEXT,
    stripe_invoice_id TEXT,
    amount_cents INTEGER,
    metadata TEXT,                          -- JSON for additional data
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

CREATE INDEX idx_subscription_events_account ON subscription_events(account_id);
CREATE INDEX idx_subscription_events_stripe_event ON subscription_events(stripe_event_id);
CREATE INDEX idx_subscription_events_created ON subscription_events(created_at);
