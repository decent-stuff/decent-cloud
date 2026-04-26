-- Issues #409 + #410: periodic timeout cleanup for stuck contracts.
--
-- Two periodic background tasks (api/src/timeout_cleanup_service.rs) close
-- the highest-probability Stripe-account-freeze gaps:
--
--   * #410: contracts in `requested` (Stripe checkout never completed) for
--     longer than REQUESTED_TIMEOUT_SECONDS transition to `expired`. No
--     refund is issued because no payment ever succeeded.
--
--   * #409: contracts in `accepted` or `provisioning` (provider/system
--     failed to bring the VM up) for longer than PROVISIONING_TIMEOUT_SECONDS
--     transition to `provisioningfailed` and a full auto-refund is triggered
--     via issue_audited_refund (idempotency key shape:
--     `provisioning_failed:{contract_id_hex}:{provisioning_failed_at_ns}`).
--
-- The status column on contract_sign_requests is free-text TEXT (no CHECK
-- constraint -- see api/migrations_pg/001_schema.sql:474), so the new values
-- `expired` (already used elsewhere) and `provisioningfailed` need no schema
-- change. The application-side ContractStatus enum is the source of truth.
--
-- Tracking columns + indexes below let the cleanup loop find stale rows
-- cheaply and let ops/audit reconstruct exactly when each timeout fired
-- and which reason was recorded.

ALTER TABLE contract_sign_requests
    ADD COLUMN requested_expired_at_ns BIGINT,
    ADD COLUMN provisioning_failed_at_ns BIGINT,
    ADD COLUMN provisioning_failure_reason TEXT;

-- Partial indexes keep the periodic scan cheap by indexing only the rows
-- the cleanup loop needs to visit.
CREATE INDEX idx_contract_requested_pending_timeout
    ON contract_sign_requests (status_updated_at_ns, created_at_ns)
    WHERE status = 'requested';

CREATE INDEX idx_contract_provisioning_pending_timeout
    ON contract_sign_requests (status_updated_at_ns)
    WHERE status IN ('accepted', 'provisioning');
