-- Fix misnamed column on contract_sign_requests (issue #422).
-- The column previously named stripe_payment_intent_id stored Stripe Checkout
-- Session IDs (cs_*), not real PaymentIntent IDs (pi_*). Rename it to clarify
-- intent and add a separate, real stripe_payment_intent_id column populated
-- from session.payment_intent at checkout completion.
ALTER TABLE contract_sign_requests
    RENAME COLUMN stripe_payment_intent_id TO stripe_checkout_session_id;

ALTER TABLE contract_sign_requests
    ADD COLUMN stripe_payment_intent_id TEXT;

DROP INDEX IF EXISTS idx_contract_sign_requests_stripe_payment_intent;

CREATE INDEX idx_contract_sign_requests_stripe_checkout_session
    ON contract_sign_requests (stripe_checkout_session_id)
    WHERE stripe_checkout_session_id IS NOT NULL;

CREATE INDEX idx_contract_sign_requests_stripe_payment_intent
    ON contract_sign_requests (stripe_payment_intent_id)
    WHERE stripe_payment_intent_id IS NOT NULL;
