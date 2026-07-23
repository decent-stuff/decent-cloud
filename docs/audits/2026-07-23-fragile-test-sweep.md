# Fragile-Test Anti-Pattern Sweep — Playwright E2E Suite

**Date:** 2026-07-23
**Scope:** all 55 spec files in `website/tests/e2e/*.spec.ts` + fixtures under `website/tests/e2e/fixtures/`
**Mode:** research only — no source changes
**Trigger:** `reputation-detail.spec.ts` hardcoded a `uxaudit` pubkey that drifted when that externally-seeded account was re-seeded. That test is now fixed (self-seeds via `seedAccountDirect()` + derives the pubkey + cleans up in `finally`); this sweep looks for the same anti-pattern class across the rest of the suite.

---

## Headline verdict

**The suite is SAFE to run on a FRESH database** (schema + migrations applied, including `api/migrations_pg/002_seed_data.sql`).

- No spec depends on the externally-runtime-seeded `uxaudit` / `dc-auth.js seed-ux-data` accounts any more. Those are manual bootstrap steps, not required by any test.
- Every spec that needs a user account self-seeds it via `seedAccountDirect()` (worker-scoped, unique per run).
- Demo-offering-dependent specs rely on `002_seed_data.sql`, which is a **migration** — it always runs on a fresh DB, so the example provider + 10 demo offerings (numeric ids 1–10, all `is_example`, all offline) are guaranteed present.
- The 3 specs that mutate the shared `testAccount` pubkey (`invoices`, `rentals`, `post-rental-welcome`) correctly use `test.describe.configure({ mode: 'serial' })`.

So: no High-severity "fails on fresh DB" findings remain. The findings below are robustness/maintenance hazards, not boot-blockers.

---

## Severity summary

| Severity | Count | Meaning |
|----------|-------|---------|
| **High** (fails on fresh DB / after re-seed) | **0** | — |
| **Medium** (flaky or fragile to maintenance) | **3** | hardcoded offering IDs/names from seed_data (×2 specs); one missing cleanup |
| **Low** (cosmetic / timing / documented coupling) | **3 clusters** | magic sleeps; demo-offering coupling (6 specs); bounded retry `.catch` |

---

## Findings

### MEDIUM-1 — Hardcoded offering IDs + names from `seed_data.sql`

**Files:**
- `website/tests/e2e/saved-offerings.spec.ts:50-51,61-62,77-78,105-107,131-132`
- `website/tests/e2e/offering-detail-save.spec.ts:19,22,29,35`

**Anti-pattern:** The specs insert `saved_offerings` rows referencing **hardcoded numeric offering IDs `1`, `2`, `3`** and assert on the literal offering names `Basic VPS` / `Performance VPS` and the hrefs `/dashboard/marketplace/1` and `/dashboard/marketplace/2`. Those numeric IDs and names come from `002_seed_data.sql` (compute-001 = id 1 "Basic VPS", compute-002 = id 2 "Performance VPS", gpu-001 = id 3).

**Why it's fragile:** This is the *same anti-pattern class* as the `uxaudit` incident — a hardcoded identifier pinned to externally-seeded state. It happens to work today because `002_seed_data.sql` is the first insert on a fresh DB, so BIGSERIAL assigns 1–10 in file order. It breaks if anyone:
- reorders/adds an offering above `compute-001` in `seed_data.sql`, or
- the DB is seeded with offerings before the migration runs (unusual, but possible in fixture-laden dev DBs), or
- an offering name in `seed_data.sql` is edited.

The numeric ID is an implementation detail of seed ordering; the test should not depend on it.

**Fix (using existing seed-helpers):** Seed the offerings the test saves, and derive IDs/names from the seeded rows. Use `seedRentableOffering()` (returns `{ offeringNumericId, offeringId, offeringName }`) and pass `offeringNumericId` to the saved-offerings INSERT and to the href assertions; pass `offeringName` to the name assertions. Example shape for `saved-offerings`:
```ts
const o1 = await seedRentableOffering({ name: 'E2E Saved A' });
const o2 = await seedRentableOffering({ name: 'E2E Saved B' });
await seedSavedOffering(pubkey, Number(o1.offeringNumericId));
await seedSavedOffering(pubkey, Number(o2.offeringNumericId));
// assert on o1.offeringName and href `/dashboard/marketplace/${o1.offeringNumericId}`
// cleanup: deleteSavedOfferingsForUser(pubkey) + deleteOfferingsByProvider(o1.providerPubkeyHex) + deleteOfferingsByProvider(o2.providerPubkeyHex)
```
`offering-detail-save` should `goto('/dashboard/marketplace/' + o.offeringNumericId)` and assert on `o.offeringName` instead of the hardcoded `/1` / `Basic VPS`. Note: `seedRentableOffering` uses `offering_source='self_provisioned'` so the offering is reachable on the detail route without an agent pool.

---

### MEDIUM-2 — `seedAccountDirect()` with no cleanup

**File:** `website/tests/e2e/account.spec.ts:37`

**Anti-pattern:** `const credentials = await seedAccountDirect();` … `await context.close();` — the account (`accounts` + `account_public_keys` rows) is created but **never deleted**. No `deleteAccountByUsername(credentials.username)`.

**Why it's fragile:** Data accumulation across runs — each invocation orphans one `accounts` row + one `account_public_keys` row (and any cascade children). It does NOT break the test itself (each run uses a fresh random username that is never re-queried), so this is a slow bloat / hygiene issue, not a functional breakage. Compare: the `testAccount` fixture (`fixtures/test-account.ts:37-44`) and `test-admin-account.ts:40` both delete in teardown; this spec is the outlier.

**Fix:** Wrap the body in `try { … } finally { await deleteAccountByUsername(credentials.username); }`, matching the fixture teardown pattern.

---

### LOW-1 — Magic sleeps (`waitForTimeout`) in a hydration-retry loop

**File:** `website/tests/e2e/recovery-flow.spec.ts:72,113`

**Anti-pattern:** `await page.waitForTimeout(100);` inside a `for (attempt < 20)` loop that re-`fill()`s an input and re-`click()`s submit, breaking when a success heading becomes visible. This is a fixed-duration sleep rather than a deterministic wait.

**Why it's fragile:** Bounded (max 20 × 100 ms = 2 s) and mitigated by the `isVisible()` break, so it rarely flakes — but it is the only `waitForTimeout` usage in the whole suite and contradicts the project's "no magic sleeps" rule (`website/AGENTS.md`). Under load the 100 ms gap can be too short, wasting attempts; under fast hydration it's wasted time.

**Fix:** Replace the retry-with-sleep with `page.waitForResponse` on the recovery POST (or `waitForSelector` on the success heading after a single `fill`+`click`), mirroring the `waitForResponse` pattern already used in `anonymous-browsing.spec.ts:93` and `invoices.spec.ts:136`. If a retry is genuinely needed for hydration races, prefer `clickAndRetry` from `fixtures/auth-helpers.ts` (already used elsewhere) over `waitForTimeout`.

---

### LOW-2 — Demo-offering cluster coupled to `seed_data.sql`

**Files (all READ-ONLY, all load `/dashboard/marketplace?demo=1&offline=1`):**
- `website/tests/e2e/search-dsl.spec.ts:17,35`
- `website/tests/e2e/marketplace-sort.spec.ts:16,28`
- `website/tests/e2e/offering-status-badge.spec.ts:18,24`
- `website/tests/e2e/marketplace-empty-state.spec.ts:14,22,34`
- `website/tests/e2e/anonymous-browsing.spec.ts:124,129` (asserts `demoCount > 0`)
- `website/tests/e2e/dashboard-banners.spec.ts:69,72`

**Anti-pattern:** These specs assume the dev DB "ships only offline demo offerings" (quoted from their own comments) and assert at least one offering row / badge renders. They depend on `002_seed_data.sql`'s example provider + 10 offerings.

**Why it's only Low:** `002_seed_data.sql` is a **migration** — guaranteed on every fresh DB — so these pass on a clean database and on re-seed (re-running migrations reproduces identical ids). They are READ-ONLY, so no parallel-cleanup collision. The coupling is explicit and documented in each spec's header comment. Risk is limited to: someone deleting/altering `seed_data.sql`, or making demo offerings online by default (which would change the empty-state assertions).

**Fix (optional, if decoupling is desired):** Each `beforeEach` could `seedRentableOffering()` a known offering and navigate to its detail/filter state instead of relying on the global demo set — but given the migration guarantee this is lower priority than MEDIUM-1/2.

---

### LOW-3 — Bounded retry `.catch(() => {})` (not a silent failure)

**File:** `website/tests/e2e/anonymous-browsing.spec.ts:169-170` (and the same pattern is the basis of the recovery-flow loop above)

**Anti-pattern:** `await bannerSignIn.click({ timeout: 5000 }).catch(() => {});` inside a loop that re-checks `page.url().includes('/login')` and breaks on success.

**Why it's only Low:** This is NOT a silent error swallow — the loop has a deterministic success condition and a bounded attempt count (10), and the final `expect(page).toHaveURL(...)` (line 174) is the real assertion that will fail loudly if the flow never completed. The `.catch` only suppresses per-attempt click failures during a hydration race. It is the idiomatic Playwright click-until-hydrated pattern. No empty `catch {}` / `catch (e) {}` blocks exist anywhere in the suite (verified by ripgrep).

**Fix:** None required. If tightened, prefer `clickAndRetry(page, target, success)` from `fixtures/auth-helpers.ts`.

---

## Patterns that were checked and are CLEAN

- **Serial mode for shared-pubkey mutation:** exactly the 3 specs that mutate the worker-shared `testAccount` pubkey use `mode: 'serial'` — `invoices.spec.ts:25`, `rentals.spec.ts:28`, `post-rental-welcome.spec.ts:31`. The other DB-mutating specs (`transfers`, `offerings-status-menus`, `saved-offerings`, `notification-bell`, `admin-dashboard`) seed under either the worker-scoped pubkey (isolated per worker) or fresh random counterpart pubkeys, so they are parallel-safe without serial mode.
- **Cross-test READ coupling:** `transfers.spec.ts` "All Recent" view reads all platform transfers but asserts on a per-run `uniqueMemo` (`randomHex(4)`), not on counts — correctly isolated from parallel workers.
- **Time-dependent ordering:** exact-count assertions in `rentals` (`All.*3`, tab counts) and `invoices` are safe because each test seeds its own contracts for an isolated worker pubkey and cleans up in `finally`; `admin-dashboard` uses a `Date.now()`-suffixed token + seeded `created_at` for its own rows, not global ordering. `provider-response-metrics.spec.ts:9` uses a sentinel `0×64` pubkey and asserts only response *shape*, not values.
- **Cleanup coverage:** every DB-seeding spec cleans up in `finally`/`afterAll` EXCEPT `account.spec.ts:37` (MEDIUM-2). `seedAccountDirect` is always paired with `deleteAccountByUsername` in both fixtures (`test-account.ts`, `test-admin-account.ts`) and in `reputation-detail.spec.ts`.
- **Empty `catch`/silent swallow:** none found (ripgrep for `catch {}` / `catch (e) {}` returns nothing across specs + fixtures).
- **Hardcoded 64-char hex pubkeys:** only one literal — `offering-sla-empty-state.spec.ts:7` `'6578616d...'` — which is the ASCII placeholder `"example-offering-provider-identifier"` from `seed_data.sql`, used deliberately to match the example provider. Not a fragile real-pubkey dependency.
- **`waitForTimeout`:** only the 2 instances in `recovery-flow.spec.ts` (LOW-1). Zero `networkidle` calls (matches the `website/AGENTS.md` rule).

---

## Recommended fix order

1. **MEDIUM-2** (`account.spec.ts` cleanup) — trivial 3-line `try/finally`, do first.
2. **MEDIUM-1** (`saved-offerings` + `offering-detail-save` hardcoded IDs/names) — the only remaining instances of the exact anti-pattern that caused the original incident; switch to `seedRentableOffering()` + derived IDs.
3. **LOW-1** (`recovery-flow` magic sleeps) — convert to `waitForResponse`/`clickAndRetry` when touching that spec.
4. **LOW-2/LOW-3** — optional; no action needed for fresh-DB safety.

## Risks / notes

- **Defining "fresh DB":** this verdict assumes migrations `001`+`002_seed_data.sql` are applied. A DB with schema only and NO seed migration would fail the demo-offering cluster (LOW-2) and the hardcoded-id specs (MEDIUM-1). The warm stack and `E2E_AUTO_SERVER=1` both run migrations, so this is the normal state.
- **`seed-ux-data` / `seed-contracts` (`scripts/dc-auth.js`):** these are **manual** bootstrap helpers, NOT part of migrations or the test setup. No spec depends on them post-fix. If a future spec needs a "known provider with offerings + heartbeats", it must self-seed (e.g. `seedRentableOffering` with `offering_source:'self_provisioned'`) rather than reaching for `seed-ux-data`.
- **Test bloat from MEDIUM-2:** orphaned accounts are never re-read (random usernames), so they cause DB growth but not test failures; still worth fixing for hygiene and to keep `accounts` FK cleanup paths exercised.
