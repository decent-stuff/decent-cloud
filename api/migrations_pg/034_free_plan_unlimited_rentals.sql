-- Migration 034: Give free plan unlimited rentals
-- The one_rental limit was wrong: users pay per-rental to providers,
-- so the platform should not impose an arbitrary rental cap.
-- Email verification (added as enforcement in contracts.rs) is the
-- Sybil-resistance mechanism instead.
UPDATE subscription_plans
SET features = '["marketplace_browse","unlimited_rentals"]',
    updated_at = (EXTRACT(EPOCH FROM NOW()) * 1000000000)::BIGINT
WHERE id = 'free';
