-- Issue #410 Option B/C: periodic timeout cleanup for contracts stuck in the
-- literal `pending` state (pre-payment per the issue text).
--
-- The companion task `cleanup_stale_pending`
-- (api/src/timeout_cleanup_service.rs) scans for `pending` rows whose
-- `COALESCE(status_updated_at_ns, created_at_ns)` is older than
-- PENDING_TIMEOUT_SECONDS (default 3600s) and transitions them to `expired`.
-- The scan additionally guards `payment_status != 'succeeded'` so a
-- paid-but-pending edge row is never silently expired (no refund logic
-- exists in this path, mirroring `expire_requested`).
--
-- The status column on contract_sign_requests is free-text TEXT (no CHECK
-- constraint -- see api/migrations_pg/001_schema.sql:474) and `expired` is
-- already in use elsewhere, so no schema change is required. The
-- application-side ContractStatus enum is the source of truth.
--
-- Partial index (mirrors 045_contract_timeout_states.sql:32-34) keeps the
-- periodic scan O(stale) by indexing only the rows the cleanup loop visits.

CREATE INDEX IF NOT EXISTS idx_contract_pending_timeout
    ON contract_sign_requests (status_updated_at_ns, created_at_ns)
    WHERE status = 'pending';
