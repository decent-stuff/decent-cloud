-- Add subscription billing support to provider offerings
-- Allows providers to offer recurring billing (monthly/yearly) for their services

-- is_subscription: 0 = one-time rental, 1 = recurring subscription
ALTER TABLE provider_offerings ADD COLUMN is_subscription INTEGER DEFAULT 0;

-- subscription_interval_days: billing cycle length (30 = monthly, 365 = yearly)
ALTER TABLE provider_offerings ADD COLUMN subscription_interval_days INTEGER;

-- Index for filtering subscription vs one-time offerings
CREATE INDEX idx_provider_offerings_is_subscription ON provider_offerings(is_subscription);
