-- Add subscription fields to accounts table
-- Links accounts to their Stripe subscription

ALTER TABLE accounts ADD COLUMN stripe_customer_id TEXT;
ALTER TABLE accounts ADD COLUMN subscription_plan_id TEXT DEFAULT 'free';
ALTER TABLE accounts ADD COLUMN subscription_status TEXT DEFAULT 'active';  -- active, past_due, canceled, trialing
ALTER TABLE accounts ADD COLUMN subscription_stripe_id TEXT;                 -- Stripe subscription ID (sub_xxx)
ALTER TABLE accounts ADD COLUMN subscription_current_period_end INTEGER;     -- Unix timestamp (nanoseconds)
ALTER TABLE accounts ADD COLUMN subscription_cancel_at_period_end INTEGER DEFAULT 0;

CREATE INDEX idx_accounts_stripe_customer ON accounts(stripe_customer_id);
CREATE INDEX idx_accounts_subscription_status ON accounts(subscription_status);
CREATE INDEX idx_accounts_subscription_plan ON accounts(subscription_plan_id);
