-- 051_refund_approval_gate.sql
-- Refund approval gate: every Stripe refund must pass through the gate.
-- Auto-issued only when refund_e9s <= user's latest Stripe payment.
-- Larger refunds are held for admin approval.
-- The trigger is the unbypassable backstop: even if application code is
-- bypassed, the DB refuses to record a refund without a matching refund_requests row.

CREATE TABLE IF NOT EXISTS refund_requests (
    id                       BIGSERIAL PRIMARY KEY,
    contract_id              BYTEA NOT NULL,
    requester_pubkey         BYTEA NOT NULL,
    refund_amount_e9s        BIGINT NOT NULL,
    reason                   TEXT NOT NULL CHECK (reason IN ('cancel', 'reject', 'dispute_lost', 'provisioning_failed', 'ops_manual')),
    status                   TEXT NOT NULL DEFAULT 'pending'
                             CHECK (status IN ('pending', 'auto_issued', 'approved', 'declined')),
    user_latest_payment_e9s  BIGINT NOT NULL,
    cap_exceeded             BOOLEAN NOT NULL,
    -- Self-contained: must survive contract row deletion (no FK, same as refund_audit)
    payment_intent_id        TEXT NOT NULL,
    currency                 TEXT NOT NULL,
    stripe_dispute_id        TEXT,
    stripe_refund_id         TEXT,
    idempotency_key          TEXT NOT NULL,
    created_at_ns            BIGINT NOT NULL,
    reviewed_at_ns           BIGINT,
    reviewed_by              BYTEA,
    review_note              TEXT,
    UNIQUE(contract_id, reason)
);

-- Indexes for common query patterns
CREATE INDEX idx_refund_requests_pending ON refund_requests (created_at_ns) WHERE status = 'pending';
CREATE INDEX idx_refund_requests_contract ON refund_requests (contract_id);
CREATE INDEX idx_refund_requests_requester ON refund_requests (requester_pubkey);
CREATE INDEX idx_refund_requests_idempotency ON refund_requests (idempotency_key);

-- Unbypassable backstop: block payment_status='refunded' or stripe_refund_id
-- being set unless an approved/auto_issued refund_request exists for the contract.
-- This trigger fires on BOTH transitions so that cancel/reject (which set
-- payment_status='refunded') and dispute_lost (which sets stripe_refund_id only)
-- are equally protected.
CREATE OR REPLACE FUNCTION enforce_refund_approval_gate()
RETURNS TRIGGER AS $$
BEGIN
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

DROP TRIGGER IF EXISTS trigger_enforce_refund_approval_gate ON contract_sign_requests;
CREATE TRIGGER trigger_enforce_refund_approval_gate
    BEFORE UPDATE OF payment_status, stripe_refund_id ON contract_sign_requests
    FOR EACH ROW
    EXECUTE FUNCTION enforce_refund_approval_gate();
