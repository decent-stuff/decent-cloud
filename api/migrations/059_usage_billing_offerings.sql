-- Add usage-based billing fields to provider_offerings
-- Enables flexible pricing: flat monthly, per-minute, per-hour, etc.

-- Billing unit: how usage is measured
-- 'month' = flat monthly (default, current behavior)
-- 'day', 'hour', 'minute' = usage-based
ALTER TABLE provider_offerings ADD COLUMN billing_unit TEXT NOT NULL DEFAULT 'month';

-- Pricing model: how charges are calculated
-- NULL or 'flat' = fixed price regardless of usage (default)
-- 'usage_overage' = base price includes X units, then per-unit overage
ALTER TABLE provider_offerings ADD COLUMN pricing_model TEXT DEFAULT 'flat';

-- Price per billing unit (for usage-based pricing)
-- NULL for flat monthly (uses monthly_price field instead)
ALTER TABLE provider_offerings ADD COLUMN price_per_unit REAL;

-- Included units in base price (for overage model)
-- e.g., 100 hours included, then overage kicks in
-- NULL means unlimited (flat model)
ALTER TABLE provider_offerings ADD COLUMN included_units INTEGER;

-- Price per unit after included_units exhausted
-- Only used when pricing_model = 'usage_overage'
ALTER TABLE provider_offerings ADD COLUMN overage_price_per_unit REAL;

-- Stripe metered price ID for usage-based offerings
-- Used to report usage to Stripe for billing
ALTER TABLE provider_offerings ADD COLUMN stripe_metered_price_id TEXT;
