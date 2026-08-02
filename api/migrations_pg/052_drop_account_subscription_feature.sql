-- Drop the unused account SaaS subscription feature (Free/Pro/Enterprise plans).
DROP TABLE IF EXISTS subscription_events;
DROP TABLE IF EXISTS subscription_plans;
DROP INDEX IF EXISTS idx_accounts_subscription_plan;
DROP INDEX IF EXISTS idx_accounts_subscription_status;
DROP INDEX IF EXISTS idx_accounts_stripe_customer;
ALTER TABLE accounts
  DROP COLUMN IF EXISTS subscription_cancel_at_period_end,
  DROP COLUMN IF EXISTS subscription_current_period_end,
  DROP COLUMN IF EXISTS subscription_stripe_id,
  DROP COLUMN IF EXISTS subscription_status,
  DROP COLUMN IF EXISTS subscription_plan_id,
  DROP COLUMN IF EXISTS stripe_customer_id;
