-- Migrate all ICP-denominated offerings and contracts to USD, and normalise
-- the seed data's stale payment_methods.
--
-- Background: ICPay (the ICP cryptocurrency payment rail) was fully retired on
-- 2026-07-24 — Stripe is now the sole payment rail. ICP is not a Stripe-
-- supported currency, so any offering still priced in ICP cannot actually
-- settle through checkout.
--
-- Migration 002 (already applied everywhere) seeded all 10 demo offerings with
-- currency='ICP' and payment_methods referencing 'ICP,ckBTC[,ckETH]'. Rather
-- than mutate the applied 002 (which would break sqlx's checksum verification
-- on every existing deployment), this migration is the single source of repair
-- for both fresh installs (where it runs immediately after 002) and existing
-- databases (where it converts whatever ICP rows are present).
--
-- Scope:
--   * provider_offerings.currency: 'ICP' -> 'USD'.
--   * contract_sign_requests.currency: 'ICP' -> 'USD' (any historical rows).
--   * provider_offerings.payment_methods: any value referencing a retired
--     token ('ICP', 'ckBTC', 'ckETH') -> 'Stripe' (the sole remaining rail).
--
-- The create/update boundary (api/src/database/offerings.rs +
-- api/src/openapi/providers.rs) now rejects non-Stripe currencies, so no new
-- ICP rows can appear after this runs. Idempotent.

UPDATE provider_offerings
SET currency = 'USD'
WHERE lower(currency) = 'icp';

UPDATE contract_sign_requests
SET currency = 'USD'
WHERE lower(currency) = 'icp';

-- Stripe is the sole remaining payment rail; replace any payment_methods value
-- that referenced a retired token with 'Stripe'.
UPDATE provider_offerings
SET payment_methods = 'Stripe'
WHERE payment_methods IS NULL
   OR payment_methods ~* 'icp|ckbtc|cketh';
