-- Issue #411: refund idempotency keys + persistent audit trail.
--
-- Every Stripe refund attempt (cancel, reject, dispute_lost, manual ops) now
-- writes a row here BEFORE the network call so transient failures still leave
-- a paper trail. The UNIQUE constraint on idempotency_key plus the matching
-- ON CONFLICT in record_refund_attempt() turn duplicate retries into no-ops
-- that return the original row id.
--
-- No FK to contract_sign_requests: the audit must outlive a contract row that
-- might be deleted by future GDPR / lifecycle work; ops needs the trail
-- regardless.

CREATE TABLE refund_audit (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    stripe_payment_intent_id TEXT,
    stripe_charge_id TEXT,
    amount_cents BIGINT NOT NULL,
    currency TEXT NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL,
    stripe_refund_id TEXT,
    error_message TEXT,
    request_payload JSONB NOT NULL,
    response_payload JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE INDEX idx_refund_audit_contract ON refund_audit(contract_id);
