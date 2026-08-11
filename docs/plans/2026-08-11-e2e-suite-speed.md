# E2E-1: E2E Suite Speed Investigation & Improvement Plan

**Date:** 2026-08-11
**Author:** Planner (investigation only — no code changes)
**Branch:** written against `main` (`dd07bd5c`)
**Ticket:** E2E-1 — full Playwright suite takes ~11 min, not "seconds"

---

## 1. Executive summary

Profiling the full suite against the warm stack (`localhost:59010` web /
`59011` api) on 2 workers:

| Metric | Value |
|---|---|
| Tests / files | **335** / 84 |
| Wall-clock (2 workers, local) | **500.5 s (~8.3 min)** |
| Sum of all test durations | 926.9 s |
| Achieved parallelism | **1.85×** (ceiling 2.0× for 2 workers) |
| Smoke suite | 40.5 s / 34 tests |
| Result | **330 pass, 4 FAIL, 1 skipped** |

**Why CI reports ~11 min, not 8.3.** CI sets `retries: 2` and CI hosts run
~1.3× slower than this dev box. The 4 failing tests each retry 2× on CI
(8 extra ~2-3 s runs ≈ +25 s) plus host slowdown → ~11 min. **The suite is
currently RED on `main`** (4 real product defects in `route-audit.spec.ts`).

### Three headline findings

1. **The `serial` constraint is NOT the real bottleneck.** `fullyParallel:
   true` already distributes serial *files* across workers — each worker
   spins its own `testAccount` (worker-scoped fixture, random mnemonic per
   worker). Serial mode only orders tests *within* a file. Cross-file
   parallelism works today; the ceiling is the **worker count (2)**, not
   the serial specs.
2. **`route-audit.spec.ts` is the single biggest cost AND the only failure
   source.** 99.8 s / 41 tests = ~20% of total test-time, concentrated in
   one file, and it carries all 4 failures.
3. **The dominant lever is worker count.** We achieved 1.85× of a 2.0×
   ceiling. Lifting to 4-6 workers approaches the theoretical floor
   (~230-270 s) IF the CPU-contention flakiness noted in
   `playwright.config.ts` can be hardened away. That flakiness is
   "intermittent auth-settle timeouts" — fixable by hardening auth waits,
   not by reducing workers.

---

## 2. Top 20 slowest specs (by summed test duration, 2 workers)

| # | File | Tests | Sum dur | Failed? | Serial? |
|---|------|------:|--------:|:-------:|:-------:|
| 1 | `route-audit.spec.ts` | 41 | **99.8 s** | **4** | no |
| 2 | `signin-flow.spec.ts` | 8 | 54.3 s | 0 | no |
| 3 | `registration-flow.spec.ts` | 6 | 38.8 s | 0 | no |
| 4 | `rentals.spec.ts` | 12 | 34.7 s | 0 | **yes** |
| 5 | `inline-confirm-delete.spec.ts` | 10 | 33.2 s | 0 | **yes** |
| 6 | `rent-flow.spec.ts` | 4 | 25.7 s | 0 | **yes** |
| 7 | `invoices.spec.ts` | 6 | 22.2 s | 0 | **yes** |
| 8 | `billing-settings.spec.ts` | 7 | 21.4 s | 0 | no |
| 9 | `keyboard-shortcuts.spec.ts` | 6 | 21.2 s | 0 | no |
| 10 | `offering-edit.spec.ts` | 4 | 20.7 s | 0 | **yes** |
| 11 | `admin-dashboard.spec.ts` | 8 | 20.5 s | 0 | no |
| 12 | `recovery-flow.spec.ts` | 10 | 18.2 s | 0 | no |
| 13 | `rent-dialog-keyboard.spec.ts` | 4 | 17.9 s | 0 | **yes** |
| 14 | `notification-settings.spec.ts` | 5 | 16.9 s | 0 | no |
| 15 | `account-page.spec.ts` | 6 | 16.6 s | 0 | no |
| 16 | `provider-pages-smoke.spec.ts` | 8 | 16.1 s | 0 | no |
| 17 | `offerings-status-menus.spec.ts` | 4 | 15.5 s | 0 | no |
| 18 | `search-dsl.spec.ts` | 8 | 15.4 s | 0 | **yes** |
| 19 | `cloud.spec.ts` | 7 | 15.3 s | 0 | **yes** |
| 20 | `payment-flows.spec.ts` | 3 | 14.9 s | 0 | **yes** |

Aggregate: **serial files (29) = 353.7 s of 927 s total test-time (38%)**;
non-serial = 573.3 s. The largest single serial file is only 34.7 s —
serial files are individually small and already spread across workers.

### Top 12 slowest individual tests

| dur | file › test |
|---:|---|
| 9.8 s | `rent-flow` › cancel a rental directly from the rentals list |
| 9.6 s | `provider-onboarding` › fresh provider steps through onboarding hub |
| 9.3 s | `signin-flow` › should maintain session after page refresh |
| 9.0 s | `registration-flow` › should handle network errors gracefully |
| 8.5 s | `rent-flow` › rent an offering → contract on rentals list w/ Cancel |
| 8.0 s | `account-profile-edit` › avatar URL edit persists after save+reload |
| 8.0 s | `signin-flow` › should auto-detect account from seed phrase |
| 7.7 s | `signin-flow` › should redirect to returnUrl when accessing protected page |
| 7.7 s | `agent-pool-edit` › rename persists in DB, list, and detail header |
| 7.5 s | `auth-protection` › should redirect to /login with returnUrl |
| 7.4 s | `registration-flow` › should complete full registration flow |
| 7.4 s | `signin-flow` › `@smoke` should sign in successfully |

The slow tests cluster in **signin-flow + registration-flow** (full-UI auth
flows, ~7-9 s each — inherent to driving the real auth UI) and **rent-flow**
(real signed POST + wallet debit + cancel round-trip).

---

## 3. Analysis: the serial constraint & per-worker identities

### 3.1 What `serial` actually constrains

The `testAccount` fixture (`tests/e2e/fixtures/test-account.ts`) is declared
with **`scope: 'worker'`** and calls `seedAccountDirect()`, which generates a
fresh random 12-word mnemonic per worker. So:

- 2 workers → **2 distinct accounts / pubkeys**. No cross-worker collision.
- Within ONE worker, every file that uses `testAccount` shares that worker's
  single account.
- `mode: 'serial'` on a file only guarantees **in-file test ordering** (and
  that the file's tests land on one worker). It does **not** prevent the file
  from running in parallel with other files.

Confirmed empirically: route-audit (non-serial) split its 41 tests across
worker indices `{0:12, 1:6, 2:2, 3:9, 4:4, 5:3, 6:4, 7:1}` — fullyParallel
works. The 29 serial files run *whole* on one worker each but are distributed
across the 2-worker pool like any other file.

**Conclusion: the "30 serial specs sharing one pubkey" framing overstates the
problem.** Cross-file parallelism already exists. The real ceiling is the
**worker count**, not the serial markers.

### 3.2 Why each spec is serial (read from each file's own comment)

| Reason | Example specs | Genuinely needs ordering? |
|---|---|---|
| Dependent multi-step flow on one account (setup → action → assert → teardown via `beforeAll`/`afterAll`) | `rent-flow`, `rent-provisioning-real`, `rent-wallet-auto-accept`, `payment-flows`, `provider-accept-reject` | **Yes** — these are real journeys |
| Each test seeds+cleans its OWN entity but was marked serial defensively ("shared pubkey") | `inline-confirm-delete` (per-entity `seed()` + `cleanup()` in `finally`), `search-dsl`, `saved-offerings`, `offerings-status-menus` | **No** — tests are independent |
| Mutates wallet balance / contract rows that another test asserts empty | `wallet-ui`, `rentals`, `invoices` | Partially — could be de-serialized with per-test balance reset |

### 3.3 Feasibility of per-worker / per-spec identities

**Per-worker identities already exist** (worker-scoped fixture). Two further
options, with trade-offs:

1. **Per-test-scoped identity** (create a fresh account per test): removes
   ALL serial constraints and the shared-pubkey hazard. Cost: `seedAccountDirect`
   is 2 DB INSERTs (~5-15 ms) — cheap. But it *fragments* the realistic
   "one user across many surfaces" story that some specs want, and it does
   NOT raise the worker ceiling (the real bottleneck). **Low ROI in isolation.**

2. **De-serialize files whose tests are already independent** (option 1 above's
   real value): for `inline-confirm-delete`, `search-dsl`, `saved-offerings`,
   etc., drop `serial` and let their tests split across workers. This *does*
   help once workers > 2, because it lets a single file's tests use multiple
   workers in parallel. Best paired with raising `workers`.

**Recommendation:** the identity model is fine as-is. Spend the effort on
**raising the worker ceiling** (§4.1) and **de-serializing the independent
specs** (§4.3) so the higher worker count actually pays off.

---

## 4. Ranked improvements

Estimates assume the current **500 s / 2-worker** baseline. "Wall-clock
saving" is the projected reduction on the local 2-worker run; CI savings are
~1.3× larger (slower hosts + retries).

### 🥇 #1 — Raise worker count 2 → 4 (after hardening auth waits)  · **Effort: M** · **Saving: ~200-235 s (-45%)**

- Config comment (`playwright.config.ts:25-30`) says 4 workers "contend and
  produce intermittent auth-settle timeouts (~1 flake/run, all green in
  isolation)". Root cause: under CPU contention, signed-API-gated page renders
  exceed the 5 s `expect` timeout — hence the bumped 10 s `expect` timeout.
- **Fix the flake at the source**, then raise workers:
  - Replace ad-hoc "goto then assert visible" with explicit `waitForResponse`
    on the signed `/api/v1/...` fetch each page fires (deterministic, no
    polling). Several specs already do this; standardize.
  - Gate page assertions on the auth-dependent element (the `Logout` button
    via the existing `waitForAuthReady`) rather than on content text.
- Theoretical floor at 4 workers ≈ **265 s**; at 6 ≈ **230 s**.

### 🥈 #2 — Triage the 4 `route-audit` failures (suite is RED on `main`)  · **Effort: S** · **Saving: ~25-30 s on CI**

All 4 failures are real defects the audit caught (and `KNOWN_BROKEN` map has
not been updated for them, so they fail loudly):

| Route | Defect |
|---|---|
| `/agents/pricing` | console 404 ("Failed to load resource") |
| `/dashboard/admin` | console 404 |
| `/dashboard/cloud/resources` | console 404 |
| `/dashboard/providers/[identifier]` | **data-leakage: literal `NaN` in user-visible text** |

Two paths: **(a)** fix the underlying product bugs (the `NaN` leakage is a
real UX defect worth fixing regardless), or **(b)** add them to `KNOWN_BROKEN`
in `route-audit.spec.ts` to make the suite green immediately. On CI
(`retries: 2`) each failure burns 2 retries × ~3 s = ~24 s of pure waste, and
a red suite blocks merges. **Either path removes ~25-30 s of CI retry waste.**

### 🥉 #3 — Split `route-audit.spec.ts` into N files by route category  · **Effort: S** · **Saving: ~30-50 s at 4 workers**

One file = one serial unit of scheduling. 99.8 s in a single file can only
occupy one worker-slot's worth of cross-worker overlap at a time. Split into
e.g. `route-audit-public`, `-dashboard`, `-provider`, `-marketplace`,
`-rentals` (sharing a single seed helper). With 4 workers, the 41 tests spread
4-way instead of effectively 2-way. The shared `beforeAll` seeding
(`seedOffering` + `seedRentableOffering` + `seedContract`) becomes a tiny
per-file cost (ms). **Best done together with #1.**

### #4 — De-serialize the independent specs  · **Effort: S** · **Saving: ~15-25 s at 4 workers**

Drop `mode: 'serial'` from files whose tests already self-contain
(seed + cleanup in `finally`), so their tests split across workers:

- `inline-confirm-delete.spec.ts` (10 tests, 33.2 s) — per-entity `seed()`+`cleanup()`, confirmed independent.
- `search-dsl.spec.ts` (8 tests, 15.4 s).
- `saved-offerings.spec.ts` (5 tests, 14.5 s).

Only pays off once `workers > 2` (#1). Audit each for hidden cross-test state
before flipping.

### #5 — Collapse redundant full-UI auth in `signin-flow` / `registration-flow`  · **Effort: M** · **Saving: ~20-30 s**

93 s combined, ~7-9 s per test, all driving the real ~7 s auth UI. Several
sub-tests perform a full `signIn()` just to assert a minor variant:

- `signin-flow › maintain session after page refresh` — full sign-in THEN tests
  refresh. The signed-in `context` is already established by the fixture for
  most tests; this one could reuse a pre-authed context and only test the refresh.
- `registration-flow › handle network errors gracefully` — re-drives
  registration to test error UX; could throttle one fetch instead of the full flow.
- Consider a shared `beforeAll` sign-in for the non-smoke variants (smoke
  stays canonical).

Keep `@smoke should sign in successfully` and `should complete full
registration flow` as the canonical end-to-end paths (one each); refactor the
rest to start from an authed context.

### #6 — Trim the `route-audit` `checkStuckLoading` 3 s grace  · **Effort: S** · **Saving: ~5-15 s**

`route-audit.spec.ts:478` does `waitForTimeout(3000)` inside
`checkStuckLoading`, re-checked only when a spinner is present. If healthy
routes never show a spinner, the cost is near-zero today — but verify the
grace isn't firing on slow workers. Could be reduced to 1.5 s or converted to
a polling predicate. Minor; pursue only after #2/#3.

### #7 — Make `addInitScript` auth robust under contention (supports #1)  · **Effort: M** · **Saving: enabler for #1**

The fixture injects the seed phrase via `context.addInitScript`
(localStorage). Under 4-worker CPU contention, hydration + the subsequent
signed-API call can lag. Standardizing on `waitForResponse` for the first
`/api/v1/` call after each `goto` (instead of content-text waits) removes the
load-sensitivity that caps workers at 2 today.

---

## 5. Recommended order (by ROI)

1. **#2 — Triage route-audit failures** (S, immediate, makes suite green, -25-30 s CI).
2. **#1 + #7 — Harden auth waits, then raise workers 2 → 4** (M, the big lever, -200+ s).
3. **#3 — Split route-audit** (S, multiplies the #1 win, -30-50 s).
4. **#4 — De-serialize independent specs** (S, lets #1's workers saturate).
5. **#5 — Collapse redundant auth** (M, -20-30 s, also speeds smoke slightly).
6. **#6 — checkStuckLoading grace** (S, cleanup, last).

Projected end state: **~230-270 s local (≈ 4-5 min CI), green**, with 4-6
workers and a green route-audit. Smoke stays ~30-40 s.

---

## 6. Risks & caveats

- **Raising workers may resurface flakes** if auth waits aren't hardened first
  (#7 before #1). The config comment documents a specific "1 flake/run" failure
  mode at 4 workers — respect it or fix the root cause.
- **De-serializing (#4) needs a per-file audit** for hidden shared state
  (e.g. a test that leaves wallet rows another asserts empty). The
  `inline-confirm-delete` finally-cleanup pattern is the safe template; specs
  that rely on `beforeAll`-seeded shared rows must stay serial.
- **`route-audit` splitting (#3)** duplicates the `beforeAll` seeding across
  files (cheap, ms) — acceptable. Keep ONE `KNOWN_BROKEN` source of truth.
- **The 4 failures are real product defects**, not test bugs. Adding them to
  `KNOWN_BROKEN` hides them; fixing them is better. The `NaN` leakage on
  `/dashboard/providers/[identifier]` is user-visible and should be fixed
  regardless of this effort.
- Measurements are from ONE 2-worker local run on the warm stack. CI timings
  differ (~1.3× slower + retries). Re-profile after #1 lands to confirm the
  worker-count win scales as predicted.

---

## Appendix: methodology

```bash
cd repo/website
# warm stack: web localhost:59010, api localhost:59011, postgres sidecar
timeout 1500 npx playwright test --workers 2 --reporter=json > results.json
# parsed results.json: per-test duration, status, workerIndex, errors
```

Anti-pattern sweep: **0 actual `networkidle` waits remain** (all 18 grep hits
are in comments documenting its removal). Only **2 real `waitForTimeout`
calls** (`route-audit` 3 s spinner grace + two 250 ms click-retry loops).
Seeding volume is high but DB-direct-cheap (`seedContract` 50×, `seedOffering`
32× — ms each, not a bottleneck).
