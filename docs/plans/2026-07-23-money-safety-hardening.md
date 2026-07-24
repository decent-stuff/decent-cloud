# Money-Safety Hardening + Fresh Issue Sweep (2026-07-23)

**Extends:** `2026-07-23-cost-safe-billing.md` (the research/architecture doc — read its
[§2](2026-07-23-cost-safe-billing.md#2-negative-balance--over-payout--under-collection-holes)
and [§5.2](2026-07-23-cost-safe-billing.md#52-hard-invariants-db-enforced--the-non-negotiables)
for the full analysis). That doc is research-only; THIS plan implements its **Phase 1A**
build sequence against real code, then continues with a fresh functional/visual issue sweep
and e2e coverage.

**STATUS: Phase 1A+1B+1A.5 COMPLETE — Phase 1C IN PROGRESS**

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

## Phase 1A.5 — test-infra DRY + dev-cycle — COMPLETE (commit `51131bfd`)
- Replaced 49-entry hardcoded `include_str!` array + `migration_hash()` with `sqlx::migrate!()`.
- +37 / −290 lines. All migration tests + 123 contracts::tests pass (047/048/049 CHECKs confirmed).
- Dev cycle: targeted nextest runs are already 1-11s (the goal). Full suite (~2537 tests) needs
  a wall-clock budget, not per-invocation.

## Phase 1B — fresh functional/visual audit (subagents, no mocks)
Dispatch read-mostly subagents against the warm stack (web:59010, api:59011) via `scripts/browser.js`:
- **Audit-1:** every `/dashboard/*` + `/account/*` route — console errors, dead links, spinners, AI-slop/stubs, broken forms.
- **Audit-2:** marketplace + rental detail + public pages (landing, login, providers) — flows a new + returning user hits.
Findings logged to `docs/OPEN_ISSUES.md`; each fix is TDD RED→GREEN→commit.

### Phase 1B — COMPLETE (7 commits, `02affbf7`–`6b4d36e2`)
| Commit | Fix | Impact |
|--------|-----|--------|
| `02affbf7` | Cluster A: SSE env var `VITE_API_BASE_URL` → import `API_BASE_URL` from `api.ts` | 2 routes: live contract-status SSE + live password-reset SSE |
| `f40e35eb` | B1: `getContractUsage` signed for correct path | contract usage 401 → 200 |
| `d5a2e019` | SSE auth double-prefix bug (discovered) | ALL SSE handlers: `/api/v1/api/v1/...` → correct path |
| `ab460a6e` | clippy fixup for auth.rs | clean build |
| `e7519ee4` | B2: pending-password-reset `AgentAuthenticatedUser` → `ProviderOrAgentAuth` | provider self-service 401 → 200 |
| `6b4d36e2` | B3+B4: new `GET /users/:pk/public-profile` (no auth) + `PublicContractSummary` | reputation/user pages 401 → 200; no sensitive data leak |

**Result:** 43/43 routes PASS, 0 findings. `KNOWN_BROKEN` map empty. Vitest 866 passed. `npm run check` 0 errors.

Also: `7da934bc` — saved-offerings spec serial mode (parallel DB cleanup race fix).

## Phase 1C — radically improve e2e harness + UX optimization

### Goal
1. **Flow catalog** (`tests/e2e/FLOWS.md`) — single source of truth mapping ALL user flows → tests + coverage status.
2. **Expanded smoke tier** — `@smoke` tags on ~15 critical-path tests, runnable in <30s.
3. **Coverage gap closure** — provider accept/reject contracts, password reset interactions, agent management.
4. **UX optimization** — reduce clicks, keyboard shortcuts, simplify flows (with e2e tests codifying the optimized flows).
5. **Speed** — smoke tier in seconds for dev loop; full suite acceptable for CI.

### Coverage gaps identified (preliminary)
| Gap | Priority | Notes |
|-----|----------|-------|
| Provider accept/reject contract requests | HIGH | Only anonymous test exists; no authenticated accept/reject flow |
| Password reset interactions | MEDIUM | Page loads (fixed in 1B) but no interaction test |
| Provider agent pool management | LOW | Only heading smoke test |
| Full rent→accept→provision cycle | MEDIUM | Only up to `requested` (Stripe can't complete in harness) |

### Current harness state
- 56 spec files, 256 tests, 3.7 min with 4 workers.
- Well-structured fixtures: `test-account` (fast-auth via `addInitScript`), `seed-helpers` (DB-direct), `auth-helpers`, `api-base`, `stripe-mock` (external boundary only).
- Vitest (866 tests) = pure-logic unit tests, NOT UI verification — correctly placed, no migration needed.
- No TUI/desktop app exists — surfaces are Web (SvelteKit) + CLI (`cli/`).

## Method
PoC-first (repo/AGENTS.md) → RED → GREEN → keep test → commit each unit. No mocks in prod.
DRY/KISS/YAGNI. Greenfield (no backward-compat). Orchestrate via subagents to preserve context.
Commit each unit when done. Run `cargo nextest run -p api` + warm-stack e2e after each phase.

## Session commit log
_(updated as units land)_

### Phase 1A — money-safety holes closed (TDD, all GREEN)
| Commit | Item | Risk closed |
|--------|------|-------------|
| `220c2a82` | A1: R10 payment_status allow-list + CHECK migration 047 | R10 |
| `e6b5441e` | A2: R1 gate provisioning on payment_status, migration 048 | R1 |
| `45d40d82` | A3: R2/R3 refund+release integrity, migration 049 | R2/R3 |
| `6b3ad47e` | A4: R5 no silent refund marking + prod Stripe requirement | R5 |
| `41841fc8` | follow-up: stats test setups consistent with 048 CHECK | — |
| `46edc93c` | R9: dispute-lost refund subtracts released funds | R9 |

495 money-safety tests + 530 contract-touching tests PASS, 0 regressions. `.sqlx` cache
regenerated + committed. Remaining research-doc risks (R4/R6/R7/R8) are ICPay/webhook/timing
items needing product decisions — parked in `cost-safe-billing.md`.

**Out-of-Phase-1A bugs found (for later):** `timeouts.rs:414 mark_provisioning_failed` refunds
full `payment_amount_e9s` (safe today, fragile); ICPay "succeeded immediately" timing (R2.9);
test-helper `insert_contract_request` hardcodes `payment_amount_e9s=1000` + ambiguous 6th arg.

## Phase 1A.5 — test-infra DRY + dev-cycle (planned)
- `test_helpers.rs:529-726` migration list is a 49-entry hardcoded `include_str!` array → every
  new migration must be hand-added or tests SILENTLY skip it. Replace with `sqlx::migrate!`
  (auto-discovers + orders `.sql` files) — eliminates the footgun + DRYs.
- Dev cycle: targeted nextest runs are already 1-11s (the goal). Full suite (~2537 tests) needs
  a wall-clock budget, not per-invocation. Document the targeted-loop workflow.
