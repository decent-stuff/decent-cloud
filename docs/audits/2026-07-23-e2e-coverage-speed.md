# E2E Coverage & Speed Audit — 2026-07-23

**Scope:** `website/` Playwright E2E harness (55 spec files, **202 tests**) against the warm dev
stack (web `http://localhost:59010`, api `http://localhost:59011`).
**Mode:** Research-only — no source changes. All timings are wall-clock, warm stack, single shared
backend, `PLAYWRIGHT_BASE_URL=http://localhost:59010`.
**Box:** Intel Xeon D-1541, **16 cores**, **62 GiB RAM**, no swap.

---

## TL;DR

- **Coverage:** 44 / 48 user-reachable routes are visited by ≥1 spec (**92%**). The 4 gaps are
  `/offline`, `/dashboard/offerings/[id]/edit`, `/dashboard/reputation/[identifier]/trust`, and
  `/dashboard/provider/agents/[pool_id]`. A larger *depth* gap exists: the core
  **rent → pay → view-rental → cancel** loop is only **fragmented**, never run as one connected
  journey, and 7 provider sub-pages are **smoke-only** (empty-state, non-provider fixture).
- **Speed verdict:** **The prior "8 workers = no gain" claim is mostly right in conclusion but wrong
  in mechanism.** Measured `4w=151.4s · 8w=148.5s · 12w=151.8s · 16w=168.0s` — workers past ~6 give
  **negative** returns. The bottleneck is **NOT CPU** (box is idle) and **NOT browser count**; it is
  the **single shared API+Postgres stack saturating** under concurrent test load, surfacing as
  `Failed to fetch` / `Cannot connect to API server` / 401s. Parallel efficiency collapses
  92% → 47% → 31% → 21%.
- **Highest-value recommendation:** **Shard the suite across 2-3 warm stacks** (each its own
  API+Postgres on its own port pair) at 4-5 workers/shard via Playwright `--shard`. This is the
  only path that reliably reaches **<60s** without the contention-flakiness that single-stack 8w+
  introduces. (Confidence 8/10.)

---

## Methodology

- Routes enumerated from `website/src/routes/**/+page.svelte` (48 user-reachable URLs, incl. dynamic
  `[param]` routes normalized to `<param>`).
- Per-spec coverage from grepping every `page.goto(...)` and dynamic `${...}` navigation across all
  55 `*.spec.ts` files, cross-checked against the `test` / `testLoggedOut` fast-auth fixtures
  (`fixtures/test-account.ts`) which auto-navigate to `/dashboard`.
- Speed matrix: full suite (`npm run test:e2e:fast`) at `E2E_WORKERS = 4, 8, 12, 16`, each from a
  warm stack, wall-clock timed. Per-test durations from a JUnit XML export of the 4-worker run.
- Flakiness characterization: the 2 tests that failed at 8w (`search-dsl` price/combine) were
  re-run **in isolation** at 4w and 8w — both pass clean, proving the failures are global-stack
  contention, not test bugs.

---

## Task A — COVERAGE

### A.1 Route → spec coverage matrix (all 48 routes)

| # | Route | Covered? | Spec file(s) | Notes |
|---|-------|:------:|--------------|-------|
| 1 | `/` (landing) | ✅ | auth-protection, admin-dashboard, provider-batch-actions, signin-flow, registration-flow, keyboard-shortcuts | Multiple anonymous + CTA tests |
| 2 | `/login` | ✅ | signin-flow (8), registration-flow (6), recovery-flow, login-registration-cta (3) | Full UI sign-in path canonical here |
| 3 | `/recover` | ✅ | recovery-flow (9) | Token/invalid-token/expired branches; email delivery mocked |
| 4 | `/verify-email` | ✅ | verify-email (2) | valid + invalid token |
| 5 | `/offline` | ❌ | — | **GAP**: offline/error-boundary page never visited |
| 6 | `/agents` | ✅ | agents (4) | |
| 7 | `/agents/pricing` | ✅ | agents-pricing (1) | |
| 8 | `/checkout/cancel` | ✅ | checkout (3) | Visited directly in isolation; not via real cancel redirect |
| 9 | `/checkout/success` | ✅ | checkout (3) | Visited directly; **success-redirect from payment NOT covered** (mock-policy; unit-tested only) |
| 10 | `/dashboard` (overview) | ✅ | dashboard-overview (2), anonymous-browsing, dashboard-banners (4), auth-protection, first-login-onboarding (2), keyboard-shortcuts, notification-bell (3), dashboard-role-gating | Combined `/provider/dashboard` call asserted |
| 11 | `/dashboard/account` | ✅ | account-page (8), account, billing-settings, keyboard-shortcuts | |
| 12 | `/dashboard/account/billing` | ✅ | billing-settings (7) | Alerts, persistence, no-raw-error |
| 13 | `/dashboard/account/notifications` | ✅ | account-notifications (2) | |
| 14 | `/dashboard/account/profile` | ✅ | account-profile-edit, profile-page (2) | Avatar URL + edit persist |
| 15 | `/dashboard/account/security` | ✅ | account-page (security tests) | 5 security sub-tests |
| 16 | `/dashboard/account/subscription` | ✅ | account-subscription | |
| 17 | `/dashboard/admin` | ✅ | admin-dashboard (4) | Requires admin fixture; banners, marketplace link |
| 18 | `/dashboard/cloud/accounts` | ✅ | cloud (5) | |
| 19 | `/dashboard/cloud/resources` | ✅ | cloud (5) | |
| 20 | `/dashboard/invoices` | ✅ | invoices (6, **serial**) | Provider-column→reputation link, states |
| 21 | `/dashboard/marketplace` | ✅ | anonymous-browsing (9), marketplace-sort (3), marketplace-empty-state, search-dsl (8), keyboard-shortcuts, rentable-offering-fixture, payment-flows (3), offline-provider-warning (3), dashboard-banners, auth-protection | Heaviest-covered route |
| 22 | `/dashboard/marketplace/<id>` (detail) | ✅ | offering-detail-save, offering-sla-empty-state, offline-provider-warning | Save/bookmark, SLA empty-state (#435), offline-warning |
| 23 | `/dashboard/marketplace/compare` | ✅ | compare-share (@smoke) | URL canonicalization + clipboard only |
| 24 | `/dashboard/offerings` (provider list) | ✅ | offerings-status-menus (4), offerings-template (2), anonymous-browsing | Visibility/stock menus persist via signed PUT |
| 25 | `/dashboard/offerings/create` (wizard) | 🟡 | become-provider (1) | **Partial**: step 1→2 only; **step 3 submit never clicked** (would create real offering) |
| 26 | `/dashboard/offerings/<id>/edit` | ❌ | — | **GAP**: offering edit form — zero coverage |
| 27 | `/dashboard/provider/agents` | 🟡 | provider-pages-smoke | **Smoke only** (heading + "New Pool" button) |
| 28 | `/dashboard/provider/agents/<pool_id>` | ❌ | — | **GAP**: pool detail/edit — zero coverage |
| 29 | `/dashboard/provider/analytics` | 🟡 | provider-pages-smoke | **Smoke only** (empty-state) |
| 30 | `/dashboard/provider/earnings` | 🟡 | provider-pages-smoke | **Smoke only** (revenue panel heading) |
| 31 | `/dashboard/provider/feedback` | 🟡 | provider-pages-smoke | **Smoke only** (empty-state) |
| 32 | `/dashboard/provider/password-resets` | 🟡 | provider-pages-smoke | **Smoke only** (empty-state) |
| 33 | `/dashboard/provider/requests` | 🟡 | provider-requests-auth, provider-batch-actions | **Anonymous/structure only**; no authenticated populated batch accept/reject |
| 34 | `/dashboard/provider/reseller` | 🟡 | provider-pages-smoke | **Smoke only** (empty-state) |
| 35 | `/dashboard/provider/sla` | 🟡 | provider-pages-smoke | **Smoke only** (empty-state) |
| 36 | `/dashboard/provider/ssh-key-rotations` | 🟡 | provider-pages-smoke | **Smoke only** (empty-state) |
| 37 | `/dashboard/provider/support` | ✅ | notification-settings (5) | Covered as provider-setup-banner context |
| 38 | `/dashboard/providers/<identifier>` | ✅ | providers (2) | Unknown-id 404 path |
| 39 | `/dashboard/rentals` | ✅ | rentals (9, serial), auth-protection | Empty-state, status badges, cancel CTA, failed-contract CTA |
| 40 | `/dashboard/rentals/<contract_id>` | ✅ | rentals, post-rental-welcome (3, serial) | Seeded contracts; welcome banner |
| 41 | `/dashboard/reputation` | ✅ | reputation (3) | |
| 42 | `/dashboard/reputation/<identifier>` | ✅ | reputation-detail | Unknown + real pubkey/username |
| 43 | `/dashboard/reputation/<identifier>/trust` | ❌ | — | **GAP**: trust detail sub-page — zero coverage |
| 44 | `/dashboard/saved` | ✅ | saved-offerings (5), offering-detail-save | |
| 45 | `/dashboard/transfers` | ✅ | transfers (6) | |
| 46 | `/dashboard/user/<identifier>` | ✅ | user (2) | Unknown-id 404 path |
| 47 | `/dashboard/validators` | ✅ | validators (3), anonymous-browsing | |
| 48 | `/dashboard/marketplace` extras | — | (covered under #21) | |

**Legend:** ✅ = real interaction/assertion · 🟡 = smoke-only or partial flow · ❌ = zero coverage.

**Totals:** 44 covered, 4 uncovered. Of covered, **8 routes are smoke-only / partial** (🟡).

### A.2 User-flow coverage

| Flow | End-to-end tested? | What's missing |
|------|:------------------:|----------------|
| **Acquisition**: register → verify-email → first-login-onboard → dashboard | 🟡 Mostly | `registration-flow` (UI) + `verify-email` + `first-login-onboarding`; verify-email token link is token-mocked, not a real email round-trip |
| **Browse → detail → save/compare** | ✅ Yes | anonymous-browsing, marketplace-sort, search-dsl, offering-detail-save, saved-offerings, compare-share |
| **Rent a compute offering** (core product loop): browse → detail → rent dialog → pick payment → pay → contract created → view rental → status transitions → cancel | ❌ **Fragmented only** | `payment-flows` exercises rent dialog + ICPay/Stripe UI + **simulated** webhook (no real provisioning); `rentals` tests the list with **DB-seeded** contracts; cancel is asserted on seeded data, **not** on a UI-created contract. No single connected happy-path. |
| **Become provider → publish offering → manage**: onboarding banner → `/offerings/create` wizard → publish → list → edit → status menus | 🟡 Partial | Wizard stops at step 2 (step 3 submit never clicked); `/offerings/[id]/edit` untested; status menus covered |
| **Provider agent pools**: `/provider/agents` → create pool → `/provider/agents/[pool_id]` | ❌ Smoke only / gap | Pool creation + pool-detail/edit untested |
| **Account / billing / subscription** | ✅ Yes | account-page, billing-settings, account-subscription; Stripe at SDK-boundary mock |
| **Admin moderation** | ✅ Yes | admin-dashboard |
| **Cloud accounts / resources** | ✅ Yes | cloud |
| **Invoices / transfers / reputation / saved / validators** | ✅ Yes | dedicated specs each |

### A.3 Prioritized coverage gaps (by user impact)

1. **[HIGH] Offering EDIT flow — `/dashboard/offerings/[id]/edit`.** Providers editing live
   offerings is a primary provider action and has **zero** tests. (Create wizard is only half
   covered too.)
2. **[HIGH] Connected rent → pay → view-rental → cancel happy path.** The core marketplace loop is
   never exercised end-to-end through the UI; cancel is only verified on DB-seeded contracts. This
   is the highest-value *flow* gap.
3. **[MED] Provider agent-pool management — `/dashboard/provider/agents/[pool_id]`.** Pool
   creation + detail/edit untested; only a smoke render of the list.
4. **[MED] Provider sub-pages are smoke-only.** analytics, earnings, feedback, ssh-key-rotations,
   password-resets, reseller, SLA — all assert only the empty-state for a zero-offering,
   non-provider fixture. No populated-state / interaction coverage.
5. **[MED] Provider batch accept/reject (`/provider/requests`) — authenticated populated state.**
   Only anonymous-gating + page-structure are tested; the actual batch action behavior is deferred to
   integration tests.
6. **[LOW] `/dashboard/reputation/<identifier>/trust`** — trust detail sub-page untested.
7. **[LOW] `/offline`** — offline/error-boundary page never visited.
8. **[LOW] Checkout success redirect from payment** — explicitly deferred (mock-policy); covered
   by unit test only.

---

## Task B — SPEED ANALYSIS

### B.1 Worker-scaling matrix (warm stack, 202 tests, 0 retries)

| Workers | Result | Wall time | Pass / Fail / Not-run | Parallel efficiency¹ |
|:------:|--------|:---------:|:---------------------:|:--------------------:|
| 4 (default) | ✅ clean | **151.4 s** (2.5m) | 202 / 0 / 0 | **92%** |
| 8 | ⚠️ 2 fail | **148.5 s** (2.4m) | 200 / 2 / 0 | 47% |
| 12 | ❌ regress | **151.8 s** (2.5m) | 181 / 9 / 12 | 31% |
| 16 | ❌ worse | **168.0 s** (2.8m) | 166 / – / 12 | 21% |

¹ Efficiency = (Σ test durations / workers) ÷ wall time. Σ durations = **559 s** across 202 tests
(avg **2.77 s/test**), measured from the 4-worker JUnit export. Ideal wall = 559 / workers.

**Reading:** the curve is flat-to-negative past ~6 workers. Going 4→8 buys ~2% wall time but
introduces 2 failures; 12 and 16 are strictly worse on *both* axes. This is the signature of a
**shared serialized resource**, not a CPU-bound or browser-bound workload.

### B.2 Bottleneck diagnosis — it is the backend, not the box

The 8w/12w/16w logs are full of backend contention errors, not timeouts on test logic:

```
[Browser ERROR] auth.ts:190: Failed to load account for identity:
  Error: Cannot connect to API server at http://localhost:59011
[Browser ERROR] auth.ts:190: Failed to load account for identity:
  Error: Failed to search account by pubkey:
[Browser ERROR] marketplace/+page.svelte:932: Failed to load recommended offerings: TypeError: Failed to fetch
[Browser DEBUG]  No usage data for contract: Error: Failed to fetch contract usage: 401 Unauthorized
```

- **`nproc` = 16, `free -h` = 62 GiB / 43 GiB available.** The box has enormous headroom —
  consistent with the prior session's "64% idle at 8 workers". The CPU being idle *is the evidence*
  that workers are blocked on the single API+Postgres stack (connection-pool / Tokio / PG
  connection ceiling), not on local compute.
- **Flakiness is contention-induced, not test-specific.** The 2 failing tests at 8w
  (`search-dsl › price query`, `search-dsl › combine type+DSL`) both passed clean when re-run in
  isolation at **both 4w and 8w** (`8 passed` each). Their full-suite failures were the API
  returning empty/late result sets under global load → `expect(count).toBeGreaterThan(0)` got 0.
- **The "did not run" cascade at 12w/16w** is the fast-auth `testAccount` fixture failing under
  saturation: its `page.goto('/dashboard')` + `waitFor(Logout, 15s)` times out when the API can't
  answer the auth call, which marks every fixture-dependent test in that worker as not-run.

> The config comment (`playwright.config.ts:32-36`) hypothesized "sequential page loads against a
> single API+Postgres stack" and concluded 8w gives no gain. The **conclusion is correct**; the
> **mechanism is more precise**: it is backend *connection/request saturation*, and 12w/16w are
> actively harmful. Note the contradictory fixture comment (`test-account.ts:57-58`) which claims
> "16 workers = 1-minute suite" — that claim is **not reproducible** here (16w = 168s, worst).

### B.3 Slowest 10 tests (4-worker run, from JUnit)

All tests are modest (≤7.5 s); there is **no single pathological test** dragging the suite. Slowness
comes from multi-navigation + fast-auth setup, not heavy logic.

| Dur | Test |
|----:|------|
| 7.5 s | Auth Protection › redirect to /login with returnUrl |
| 6.3 s | Offering detail save flow › bookmark toggle on offering detail |
| 5.9 s | Billing Settings Page › spending alerts: no raw error text |
| 5.9 s | Account Notification Settings › notification channels render |
| 5.7 s | Billing Settings Page › settings persist on reload |
| 5.5 s | Payment Flows › ICPay payment UI |
| 5.4 s | /dashboard/invoices › provider column links to reputation |
| 5.3 s | keyboard-shortcuts › email-verification banner dismiss |
| 5.2 s | account/profile — avatar URL field › error |
| 5.1 s | Offering detail SLA card — empty state (#435) |

**Heavy setup/teardown flags:** specs using **serial mode + DB seeding** via `psql` subprocess
spawns (`post-rental-welcome`, `invoices`, `rentals`, `offerings-status-menus`,
`offering-sla-empty-state`) pay extra per-test cost: each `sql()` call forks a `psql` process and
opens a fresh PG connection. Under parallelism this multiplies fork + connection churn.

### B.4 Test-fold candidates

The **per-test floor is the fast-auth fixture**: `page.goto('/dashboard')` + `waitFor(Logout)` ≈
**1.5–2.0 s/test**. With 202 tests that is **~350–400 s of fixed overhead inside the 559 s sum
(~70%)**. This is the biggest fold lever.

| Candidate spec | Tests | Same page? | Fold action |
|----------------|:-----:|:----------:|-------------|
| `billing-settings` | 7 | yes (`/account/billing` ×7) | Convert to serial shared-session: one `goto` in `beforeAll`, reuse `page`; assert all 7 in fewer sessions |
| `account-page` | 8 | mostly (`/account`, `/account/security`) | Same — group the 5 security tests behind one nav |
| `invoices` (serial) | 6 | yes (`/invoices` ×6) | Already serial; could share one seeded dataset + one nav |
| `transfers` | 6 | yes (`/transfers` ×6) | Shared-session fold |
| `saved-offerings` | 5 | yes (`/saved` ×5) | Shared-session fold |
| `notification-settings` | 5 | yes (`/provider/support` ×5) | Shared-session fold |
| `offerings-status-menus` | 4 | yes (`/offerings` ×4) | Each re-seeds+renavs; fold behind one offering seed |

**Trade-off:** shared sessions lose per-test isolation (a state-mutating test leaks into the next).
Safe for read/visual/smoke tests; risky for tests that mutate DB rows. Apply selectively.
**Estimated win:** cutting the ~2 s/test auth floor for ~40 foldable tests ≈ **70–80 s off the sum**
→ ~480 s sum → ~80 s at 4w. Not enough alone for <60s, but it stacks with sharding.

**Thin/overlap folds:** `checkout` (3 isolated page-render tests) and several single-assertion
`provider-pages-smoke` entries are candidates to merge, but they are cheap (≤3 s) — low priority.

### B.5 Sharding feasibility

Playwright supports `--shard=x/N` natively, splitting the test files across N runners. The
constraint here is that **every shard must point at its own warm stack** so the per-stack load stays
at the ~4-6-worker sweet spot.

| Shards (stacks) | Workers/shard | Effective parallelism | Est. wall (shard max) | <60s? |
|:---------------:|:-------------:|:---------------------:|:---------------------:|:----:|
| 1 (current) | 4 | 4 | 151 s | ❌ |
| 1 | 8 | 8 | 148 s (+flakes) | ❌ |
| 2 | 4 each | 8 (no cross-stack contention) | ~75 s | ❌ (close) |
| **3** | **4 each** | **12** | **~50 s** | **✅** |
| 3 | 5 each | 15 | ~45 s | ✅ |

**Infra needed:**
1. `scripts/dev-server.sh` generalized to bring up **N** stacks on N port pairs, e.g.
   `59010/59011`, `59020/59021`, `59030/59031`, each with its own `RATE_LIMIT_ENABLED=false`.
   (Postgres can stay shared — it's not the saturated layer; or give each a DB for isolation.)
2. A shard runner (small wrapper) that maps `--shard=i/N` → the i-th stack's
   `PLAYWRIGHT_BASE_URL`/`PLAYWRIGHT_API_URL` and fans out the N invocations in parallel.
3. Merge the N JUnit/HTML reports (Playwright `merge-reports`).

Postgres sharing is the one risk: if all shards hammer one DB, the PG connection ceiling could
re-introduce the saturation. Safer to give each shard its own DB (`createdb e2e_shard_i`); the seed
helpers already key off `DATABASE_URL`.

---

## Recommendations → reach <60s (with confidence)

| # | Recommendation | Est. wall impact | Conf. |
|---|----------------|:----------------:|:-----:|
| 1 | **Shard across 3 warm stacks** (`--shard=i/3`, 4 workers/shard, per-shard API+DB). | 151s → **~50s** | **8/10** |
| 2 | **Revert/avoid ≥8 workers on a single stack.** 4-5 is the reliability sweet spot; ≥12 is harmful. | prevents flakes | **9/10** |
| 3 | **Test-fold** same-page specs (billing, account-page, invoices, transfers, saved, notifications) to shared authenticated sessions (serial `beforeAll` nav) to kill the ~2s/test auth floor. | 559s sum → ~480s | **6/10** |
| 4 | **Investigate the backend concurrency ceiling**: the saturation is `Cannot connect`/401 — profile API DB-pool size, Tokio worker threads, and PG `max_connections` under the test's shared-IP bucket. Raising headroom could make 8w reliably help (→~80s single-stack). | enables 8w | **5/10** |
| 5 | **Replace `psql`-spawn seeding with a persistent node-pg client** in `seed-helpers.ts` to cut fork/connection churn under parallelism. | minor (5-10s) | **4/10** |
| 6 | **Fix the 2 contention-flaky `search-dsl` assertions** (`toBeGreaterThan(0)` on API counts) with a `waitForResponse` + retry so they tolerate transient empty results. | removes 8w flakes | **7/10** |

**Combining #1 + #2 + #3** is the realistic <60s recipe: shard 3 ways at 4w/shard, with folded tests
→ ~35-45s, reliably green.

---

## Appendix

- **Route count:** 48 `+page.svelte` user-reachable URLs under `src/routes/`.
- **Test count:** 202 (Playwright-reported); 189 top-level `test(` + nested.
- **Fixture model:** `testAccount` is **worker-scoped** + DB-direct `seedAccountDirect()` (skips the
  ~10-15s UI registration); per-test `page` fixture injects `localStorage.seed_phrases` via
  `addInitScript` and lands on `/dashboard`. `testLoggedOut` provides credentials without auto-signin.
- **Seeding:** `seed-helpers.ts` writes directly to Postgres via `psql` subprocess (contracts,
  transfers, offerings, SLA targets). Specs sharing the testAccount pubkey use **serial mode** to
  avoid parallel cleanup nukes.
- **Mock policy:** only Stripe SDK + outbound external HTTP; first-party API never mocked (hence the
  checkout-success redirect and full rent provisioning are out of e2e scope).
- **Raw run logs:** `/tmp/e2e-{4,8,12,16}w-out.log`, JUnit `/tmp/junit-4w.xml`.
