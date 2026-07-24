-- R10 / A1: DB-enforced allow-list for contract_sign_requests.payment_status.
--
-- `payment_status` was free-text with zero CHECK constraints; a compromised or
-- replayed webhook (or any caller) could write an arbitrary string and confuse
-- every downstream guard that compares `== 'succeeded'` (provisioning gate,
-- refund gating, dispute handling). The code-level validation is the loud,
-- caller-visible check; this CHECK constraint is the un-bypassable backstop
-- that holds even via direct SQL.
--
-- The allow-list mirrors `dcc_common::payment_status::ALL` exactly:
--   pending, succeeded, refunded, failed, disputed
ALTER TABLE contract_sign_requests
    ADD CONSTRAINT payment_status_valid
    CHECK (payment_status IN ('pending', 'succeeded', 'refunded', 'failed', 'disputed'));
