-- Add subscription tracking to contracts
-- Enables recurring billing for rental contracts

-- stripe_subscription_id: Stripe subscription ID (sub_xxx) for recurring billing
ALTER TABLE contract_sign_requests ADD COLUMN stripe_subscription_id TEXT;

-- subscription_status: 'active', 'past_due', 'cancelled', 'trialing'
-- NULL for one-time contracts
ALTER TABLE contract_sign_requests ADD COLUMN subscription_status TEXT;

-- current_period_end_ns: when current billing period ends (nanoseconds)
ALTER TABLE contract_sign_requests ADD COLUMN current_period_end_ns INTEGER;

-- cancel_at_period_end: 1 = will cancel at period end, 0 = will auto-renew
ALTER TABLE contract_sign_requests ADD COLUMN cancel_at_period_end INTEGER DEFAULT 0;

-- Index for finding active subscriptions
CREATE INDEX idx_contract_subscriptions ON contract_sign_requests(stripe_subscription_id) WHERE stripe_subscription_id IS NOT NULL;

-- Index for finding subscriptions by status
CREATE INDEX idx_contract_subscription_status ON contract_sign_requests(subscription_status) WHERE subscription_status IS NOT NULL;
