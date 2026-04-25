-- Phase 1 of Stripe dispute handling (issue #421, #408): pause-and-resume support.
-- Webhook handlers + dc-agent polling change land in Phase 2.
--
-- Adds:
--  1. contract_disputes      -- one row per Stripe dispute, idempotent on stripe_dispute_id.
--  2. contract_sign_requests.paused_at_ns / total_paused_ns / pause_reason
--                            -- pause/resume bookkeeping; total_paused_ns credits the prorated refund.
--
-- The status column on contract_sign_requests is a free-text TEXT (no CHECK constraint;
-- see api/migrations_pg/001_schema.sql:473), so the new 'paused' value needs no migration:
-- the application-side ContractStatus enum is the source of truth and is extended in the
-- same commit.

CREATE TABLE contract_disputes (
    id BIGSERIAL PRIMARY KEY,
    contract_id BYTEA REFERENCES contract_sign_requests(contract_id) ON DELETE CASCADE,
    stripe_dispute_id TEXT NOT NULL UNIQUE,
    stripe_charge_id TEXT NOT NULL,
    stripe_payment_intent_id TEXT,
    reason TEXT,
    status TEXT NOT NULL,
    amount_cents BIGINT NOT NULL,
    currency TEXT NOT NULL,
    evidence_due_by_ns BIGINT,
    funds_withdrawn_at_ns BIGINT,
    closed_at_ns BIGINT,
    raw_event JSONB NOT NULL,
    created_at_ns BIGINT NOT NULL,
    updated_at_ns BIGINT NOT NULL
);

CREATE INDEX idx_contract_disputes_contract ON contract_disputes(contract_id)
    WHERE contract_id IS NOT NULL;
CREATE INDEX idx_contract_disputes_charge ON contract_disputes(stripe_charge_id);
CREATE INDEX idx_contract_disputes_status ON contract_disputes(status);

ALTER TABLE contract_sign_requests
    ADD COLUMN paused_at_ns BIGINT,
    ADD COLUMN total_paused_ns BIGINT NOT NULL DEFAULT 0,
    ADD COLUMN pause_reason TEXT;

-- Partial index over currently-paused contracts -- supports the dc-agent polling loop
-- (Phase 2) cheaply skipping non-paused rows.
CREATE INDEX idx_contract_sign_requests_paused
    ON contract_sign_requests (paused_at_ns)
    WHERE paused_at_ns IS NOT NULL;
