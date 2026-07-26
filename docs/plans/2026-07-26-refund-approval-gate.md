# Refund Approval Gate

**Created:** 2026-07-26
**Status:** Implementation
**Related:** `docs/plans/2026-07-23-cost-safe-billing.md` (Phase 1A complete)

## Goal

Every Stripe refund is auto-issued only if it does not exceed the user's latest
single Stripe payment. Refunds above that cap are held for explicit admin
approval via the admin panel. The restriction is enforced at the database level
and cannot be bypassed through code changes alone.

## Policy (user-confirmed)

1. **Auto-refund** when `refund_e9s <= user_latest_stripe_payment_e9s`
2. **Hold for approval** when `refund_e9s > user_latest_stripe_payment_e9s`
3. **Telegram alert** on every refund event (auto-issued + pending)
4. **Admin panel** to approve/decline pending requests
5. **Unbypassable**: DB trigger blocks `payment_status='refunded'` and
   `stripe_refund_id` updates unless a `refund_requests` row with
   `status IN ('auto_issued','approved')` exists

## Architecture

### Migration `051_refund_approval_gate.sql`

- `refund_requests` table (NO FK to contracts — must outlive contract rows,
  same as `refund_audit`). Columns: `id`, `contract_id BYTEA`, `requester_pubkey
  BYTEA`, `refund_amount_e9s BIGINT`, `reason TEXT CHECK`, `status TEXT CHECK
  (pending/auto_issued/approved/declined)`, `user_latest_payment_e9s BIGINT`,
  `cap_exceeded BOOL`, `stripe_refund_id TEXT`, `idempotency_key TEXT`,
  `created_at_ns BIGINT`, `reviewed_at_ns BIGINT`, `reviewed_by BYTEA`,
  `review_note TEXT`, `UNIQUE(contract_id, reason)`.
- DB trigger `enforce_refund_approval_gate` on `contract_sign_requests`:
  blocks `payment_status='refunded'` OR `stripe_refund_id` first-set unless a
  matching `refund_requests` row exists with approved status.

### DB layer `database/refund_requests.rs`

- `RefundRequest` struct (`FromRow`)
- `get_user_latest_stripe_payment(pubkey) -> Option<i64>`
- `create_refund_request(input) -> RefundRequest`
- `list_pending_refund_requests(limit, offset) -> Vec<RefundRequest>`
- `list_all_refund_requests(limit, offset) -> Vec<RefundRequest>`
- `approve_refund_request(id, admin_pubkey, note) -> RefundRequest`
- `decline_refund_request(id, admin_pubkey, note) -> RefundRequest`
- `mark_refund_request_issued(id, stripe_refund_id) -> Result<()>`

### Gate logic `process_gated_refund`

Centralized `Database` method replacing direct `issue_audited_refund` calls in
ALL 4 refund paths (cancel, reject, dispute_lost, provisioning_failed):

1. Compute prorated refund (existing `calculate_net_refund_e9s`)
2. Query user's latest Stripe payment
3. Create `refund_requests` row (unbypassable audit FIRST)
4. If cap passes → `status='auto_issued'` → `issue_audited_refund` → Telegram
5. If cap fails → `status='pending'` → Telegram alert → no Stripe call

Returns `RefundGateOutcome` enum: `AutoIssued { amount, refund_id }` |
`PendingApproval { amount, cap }` | `NoRefund`.

### Admin API (extend `admin.rs`)

- `GET /api/v1/admin/refund-requests?status=pending` — list
- `POST /api/v1/admin/refund-requests/:id/approve` — approve + issue
- `POST /api/v1/admin/refund-requests/:id/decline` — decline

### Admin UI (extend `+page.svelte`)

- Refund requests section: pending list + approve/decline buttons
- Shows: contract hex, amount, reason, user latest payment, cap exceeded

### Tests

- DB layer: unit tests in `refund_requests.rs`
- Gate logic: tests for auto-issue (cap passes) + hold (cap fails)
- Admin API: endpoint tests
- E2E: seed contract with cap-exceeded refund → cancel → verify pending in
  admin panel → approve → verify issued

## Implementation Order

1. Migration + trigger
2. DB layer + tests
3. Gate logic (replace 4 call sites) + tests
4. Admin API + tests
5. Admin UI
6. E2E test
7. FLOWS.md + OPEN_ISSUES.md

Commit each unit.
