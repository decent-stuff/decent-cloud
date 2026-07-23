# Money-Safety Hardening + Fresh Issue Sweep (2026-07-23)

**Extends:** `2026-07-23-cost-safe-billing.md` (the research/architecture doc — read its
[§2](2026-07-23-cost-safe-billing.md#2-negative-balance--over-payout--under-collection-holes)
and [§5.2](2026-07-23-cost-safe-billing.md#52-hard-invariants-db-enforced--the-non-negotiables)
for the full analysis). That doc is research-only; THIS plan implements its **Phase 1A**
build sequence against real code, then continues with a fresh functional/visual issue sweep
and e2e coverage.

**STATUS: IN PROGRESS**

## Why this exists
The user brief: "If there are documented (pre-existing) issues, fix them ALL first." The
billing research doc documents concrete money-safety holes R1–R10 with `file:line` citations.
These ARE the documented pre-existing issues. We close Phase 1A (no business-model change,
just DB-enforced invariants + defensive code guards) before any new feature/UX work.

## Verified facts (this session, against `repo/`)
- `payment_status` distinct values in code: **`pending, succeeded, refunded, failed, disputed`**
  (grep of all assignment/comparison sites). Allow-list = these 5.
- `update_contract_status` (`api/src/database/contracts/rental.rs:273-360`) validates auth +
  state transitions but **never reads `payment_status`** → R1 real.
- `update_icpay_payment_status` (`payment.rs:229-243`) binds `new_status` straight into the
  column, no allow-list → R10 real.
- schema `contract_sign_requests` (`migrations_pg/001_schema.sql`): `payment_status TEXT`,
  `total_released_e9s BIGINT DEFAULT 0`, `refund_amount_e9s BIGINT` — **zero CHECKs** → R2 real.
- `calculate_net_refund_e9s` (`payment.rs:174`) reads `total_released` then refund path writes
  in **separate txns** → R3 real. `reject_contract` (`rental.rs:400`) uses raw
  `payment_amount_e9s` not the net calc → R3-variant real.
- Stripe optional at boot (`main.rs:939` `check_env!("STRIPE_SECRET_KEY", optional, ...)`) → R5 real.
- `ENVIRONMENT` read at `main.rs:1278`; `== "prod"` gates CORS. Use same var to gate Stripe requirement.
- Latest migration: **046** → new ones start at **047**.
- Warm stack healthy (api:59011 `environment: dev`, web:59010). E2e smoke green (4/4, 12.8s).
- Postgres sidecar: `dc-agent-1-postgres-1` (container); `DATABASE_URL=postgres://test:test@postgres:5432/test`.

## Phase 1A — close the money-safety holes (TDD: RED → GREEN → commit each)

Each item: write a failing test that reproduces the hole, then the fix, then GREEN. No mocks
of first-party code. Verify with `cargo nextest run -p api` + the warm-stack e2e suite.

| # | Risk | Hole | Fix (DB-enforced where possible) | Confidence |
|---|------|------|----------------------------------|------------|
| **A1** | R10 | `update_icpay_payment_status` accepts any string | Allow-list the 5 values in code; add `CHECK (payment_status IN (...))` migration 047 | 9/10 |
| **A2** | R1 | provider `update_contract_status` ignores payment | Gate provider transitions to provisioned/active on `payment_status='succeeded'` OR `payment_amount_e9s=0` (free/self-rental) in the same UPDATE; add DB CHECK mirroring it | 8/10 |
| **A3** | R2/R3 | `total_released`/`refund` unbounded + TOCTOU | Migration: `CHECK (total_released_e9s + COALESCE(refund_amount_e9s,0) <= payment_amount_e9s)`. Make the release loop use a conditional UPDATE (`... WHERE new_total <= payment_amount_e9s RETURNING`) so release+refund are race-free. `reject_contract` → `calculate_net_refund_e9s`. | 8/10 |
| **A4** | R5 | Stripe optional; "refunded" with no refund | Require `STRIPE_SECRET_KEY`+`STRIPE_WEBHOOK_SECRET` at boot when `ENVIRONMENT=prod`; `issue_audited_refund` returns a loud `Err` (not `None`) when no client → refund path fails visibly | 9/10 |

### A1 — payment_status allow-list + CHECK
- **RED:** `update_icpay_payment_status(..., "bogus")` must `Err`; direct SQL `UPDATE ... SET payment_status='bogus'` must violate CHECK.
- **GREEN:** allow-list in `update_icpay_payment_status`; migration `047_payment_status_check.sql`
  `ALTER TABLE contract_sign_requests ADD CONSTRAINT payment_status_chk CHECK (payment_status IN ('pending','succeeded','refunded','failed','disputed'))`.

### A2 — gate provider accept on payment
- **RED:** a `requested`+`payment_status='pending'` contract (non-zero amount) → `update_contract_status(...,'accepted')` must `Err`; provisioning must refuse.
- **GREEN:** in `update_contract_status`, when target ∈ {accepted, provisioning, provisioned, active}
  and `payment_amount_e9s > 0`, require `payment_status='succeeded'` inside the txn. Migration
  `048_provision_requires_payment.sql`: `CHECK (status NOT IN ('provisioned','active') OR payment_status='succeeded' OR payment_amount_e9s=0)`.
- Audit provisioning entry points (recipe/self-provisioned/cloud) so none bypasses this (cf. billing §2.1).

### A3 — no over-release / over-refund
- **RED:** (1) release loop that would push `total_released > payment_amount` must refuse (0 rows / Err). (2) reject on a contract that already released funds must refund net, not gross.
- **GREEN:** migration `049_no_negative_money.sql`:
  `ALTER TABLE ... ADD CONSTRAINT money_balance_chk CHECK (total_released_e9s + COALESCE(refund_amount_e9s,0) <= payment_amount_e9s)`.
  Release path (`payment_release_service.rs`) → conditional `UPDATE ... SET total_released_e9s = total_released_e9s + $1 WHERE contract_id=$2 AND total_released_e9s + $1 <= payment_amount_e9s RETURNING ...` (0 rows = refused, logged loud). `reject_contract` → `calculate_net_refund_e9s`.

### A4 — Stripe required in prod; refunds fail loud
- **RED:** boot with `ENVIRONMENT=prod` + no `STRIPE_SECRET_KEY` must refuse to start; refund with no Stripe client must `Err`.
- **GREEN:** `serve_command()` startup validation: if `ENVIRONMENT=prod` and either Stripe secret unset → `Err` (fail-fast). `issue_audited_refund`: no client → `Err("cannot refund: STRIPE_SECRET_KEY not configured")` instead of `Ok(None)`.

## Phase 1B — fresh functional/visual audit (subagents, no mocks)
Dispatch read-mostly subagents against the warm stack (web:59010, api:59011) via `scripts/browser.js`:
- **Audit-1:** every `/dashboard/*` + `/account/*` route — console errors, dead links, spinners, AI-slop/stubs, broken forms.
- **Audit-2:** marketplace + rental detail + public pages (landing, login, providers) — flows a new + returning user hits.
Findings logged to `docs/OPEN_ISSUES.md`; each fix is TDD RED→GREEN→commit.

## Phase 1C — e2e coverage for money-safety + new flows
- e2e/integration tests proving the new guards (A1–A4) hold through the real API.
- Close any high-value coverage gap found by the audits.

## Method
PoC-first (repo/AGENTS.md) → RED → GREEN → keep test → commit each unit. No mocks in prod.
DRY/KISS/YAGNI. Greenfield (no backward-compat). Orchestrate via subagents to preserve context.
Commit each unit when done. Run `cargo nextest run -p api` + warm-stack e2e after each phase.

## Session commit log
_(updated as units land)_
