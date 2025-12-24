-- Subscription plans table for storing tier definitions
-- Plans are defined in the database to allow easy updates without code changes

CREATE TABLE subscription_plans (
    id TEXT PRIMARY KEY,                    -- e.g., 'free', 'pro', 'enterprise'
    name TEXT NOT NULL,                     -- Display name
    description TEXT,
    stripe_price_id TEXT,                   -- Stripe Price ID (null for free tier)
    monthly_price_cents INTEGER NOT NULL DEFAULT 0,
    trial_days INTEGER NOT NULL DEFAULT 0,
    features TEXT,                          -- JSON array of feature keys
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now') * 1000000000)
);

-- Insert default plans
INSERT INTO subscription_plans (id, name, description, monthly_price_cents, trial_days, features) VALUES
    ('free', 'Free', 'Basic marketplace access', 0, 0, '["marketplace_browse","one_rental"]'),
    ('pro', 'Pro', 'Full platform access', 2900, 14, '["marketplace_browse","unlimited_rentals","priority_support","api_access"]'),
    ('enterprise', 'Enterprise', 'Enterprise features', 9900, 14, '["marketplace_browse","unlimited_rentals","priority_support","api_access","dedicated_support","sla_guarantee"]');

CREATE INDEX idx_subscription_plans_stripe_price ON subscription_plans(stripe_price_id);
