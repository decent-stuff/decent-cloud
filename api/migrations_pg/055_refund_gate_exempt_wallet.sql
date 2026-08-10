-- 055_refund_gate_exempt_wallet.sql
-- The refund approval gate (051) is a backstop that blocks payment_status='refunded'
-- unless an approved refund_request exists. Its stated purpose (051 line 2) is
-- "every Stripe refund must pass through the gate" — it guards against unauthorized
-- EXTERNAL money movement (Stripe charge refunds).
--
-- Wallet refunds are instant internal balance credits: no Stripe call, no external
-- money leaves the platform. They have their own audit trail (wallet_ledger,
-- entry_type='rental_refund') and money-safety (wallet_balances CHECK balance >= 0).
-- Applying the Stripe gate to them is a category error: it blocks legitimate
-- instant credits because no Stripe payment_intent/refund_request exists.
--
-- Exempt payment_method='wallet' contracts from both guards.

CREATE OR REPLACE FUNCTION enforce_refund_approval_gate()
RETURNS TRIGGER AS $$
BEGIN
    -- Wallet refunds are instant internal credits, not Stripe refunds.
    -- The gate guards external Stripe money movement only.
    IF NEW.payment_method = 'wallet' THEN
        RETURN NEW;
    END IF;

    -- Guard 1: payment_status transitioning to 'refunded'
    IF NEW.payment_status = 'refunded' AND (OLD.payment_status IS DISTINCT FROM 'refunded') THEN
        IF NOT EXISTS (
            SELECT 1 FROM refund_requests
            WHERE contract_id = NEW.contract_id
              AND status IN ('auto_issued', 'approved')
        ) THEN
            RAISE EXCEPTION 'Refund gate violation: cannot set payment_status=refunded for contract % without an approved refund_request',
                encode(NEW.contract_id, 'hex');
        END IF;
    END IF;

    -- Guard 2: stripe_refund_id being set for the first time
    IF NEW.stripe_refund_id IS NOT NULL AND OLD.stripe_refund_id IS NULL THEN
        IF NOT EXISTS (
            SELECT 1 FROM refund_requests
            WHERE contract_id = NEW.contract_id
              AND status IN ('auto_issued', 'approved')
        ) THEN
            RAISE EXCEPTION 'Refund gate violation: cannot set stripe_refund_id for contract % without an approved refund_request',
                encode(NEW.contract_id, 'hex');
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
