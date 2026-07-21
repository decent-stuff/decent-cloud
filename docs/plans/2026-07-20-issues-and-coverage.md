# 2026-07-20 — Open Issues, Coverage Gaps, and New Visual/UX Audit

**Author:** opencode autonomous session (continuation of `2026-07-20-app-health-and-harness.md`)
**Scope:** `repo/` (submodule). Outer workspace is wrapper only.
**Predecessor:** All 7 phases of `2026-07-20-app-health-and-harness.md` are complete; baseline = 4/4 smoke in ~14s, 131/4-skip full in 2.5m against a warm stack.
**Confidence:** 9/10 that the plan is correct; per-phase confidence noted inline.

## Goal

1. Close out **all in-scope open issues** that are tractable in one session (`#433`, `#410`, plus the in-repo `product_type:gpu` bug).
2. Fill **remaining E2E coverage gaps** for info/admin routes.
3. Run a fresh **UX + visual audit** to discover new issues; fix the high-value ones, file the rest.
4. Plan and start the multi-session Decent Agents features (`#418`, `#415`, `#416`, `#427`) — break into smaller land-able pieces; commit what's shippable.
5. Keep dev cycle in seconds; commit each unit.

## Operating posture (unchanged)

- Autonomous. Subagents for high-level decisions; swarm for parallel plan/build/verify.
- TDD: RED → GREEN → keep test. No silent errors (`match`/`?`, never `let _ = ...` for Results).
- DRY/KISS/YAGNI. No backward-compat. Mocks only at smallest external-dep boundary.
- Confidence 1-10 shown for changes. Verify alignment (mechanical + human).

## Phases

### Phase A — Tractable in-scope fixes (HIGH, ~2 h)

Confidence 9/10. Parallelizable.

- [ ] **A1** In-repo known issue: marketplace empty-state hint suggests `product_type:gpu` field syntax but the API rejects it (`Unknown field: product_type`). Fix the hint text in `website/src/routes/dashboard/marketplace/+page.svelte:1307` to use a field the API actually accepts (or remove the misleading example). Add E2E test that asserts the hint text matches an accepted field name.
- [ ] **A2** #433 Top-up balance UI on `/dashboard/transfers`. Decision required (UX): pre-pay path (Stripe/ICPay deposit to balance) vs clarify empty-state copy that rentals are pay-per-transaction. Default: **clarify copy** (smaller, safer; matches the actual product model today — balance is refunds/affiliate, not pre-paid). If a deposit flow already exists in `RentalRequestDialog`, factor it into a reusable component used by both. RED test first.
- [ ] **A3** #410 Stripe stale pending contracts cleanup. Add a periodic worker that finds `pending` contracts older than a configurable window (default 60 min), transitions them to `expired`, releases held inventory/identity slot, emits metric. Time-controlled unit test. Negative test: does NOT expire recent pending contracts.

### Phase B — E2E coverage gaps + fresh audit (HIGH, ~2 h)

Confidence 9/10.

- [ ] **B1** Add E2E specs for the remaining info/admin routes that only have visibility checks today (or none): `/dashboard/cloud`, `/dashboard/providers`, `/dashboard/user`, `/dashboard/reputation`, `/dashboard/validators`, top-level `/agents`, `/checkout`. Each spec asserts: route loads, primary content visible, empty state copy is helpful, no console errors. One spec per route family for DRY.
- [ ] **B2** Fresh UX/visual audit via `scripts/browser.js shot` (no mocks). Catalog any new issues; fix high-value; file low-value.
- [ ] **B3** Codify new UX optimizations as E2E tests.

### Phase C — Decent Agents features (LARGE, multi-session)

Confidence 7/10 (scope, not correctness). Break into smaller pieces; commit what's shippable today.

- [ ] **C1** #418 Decent Agents beta onboarding — design doc + smallest landable piece (invite-code gate table + middleware). Bulk of feature is multi-session.
- [ ] **C2** #415 Subscription billing caps — schema migration + meter tables. Enforcement logic in subsequent sessions.
- [ ] **C3** #416 Usage dashboard — UI shell with placeholder data, backend wired in subsequent session.
- [ ] **C4** #427 Anthropic key proxy — design doc + smallest landable piece (config + side-by-side feature flag). Bulk of feature is multi-session.

### Phase D — Knowledge base sync (LOW, always last)

- [ ] **D1** Update `docs/OPEN_ISSUES.md` with closed/new issues.
- [ ] **D2** Update `repo/AGENTS.md` if any new conventions or commands emerged.
- [ ] **D3** Rewrite `repo/PROMPT.md` for next session.

## Risk / Out-of-Scope Today

- Full Decent Agents feature builds (#418, #415, #416, #427 end-to-end) — multi-session.
- All `deferred-post-launch` issues — explicitly parked.
