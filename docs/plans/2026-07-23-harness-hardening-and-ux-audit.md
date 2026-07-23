# E2E Harness Hardening + Skip-Gap Closure + UX Audit (2026-07-23)

**STATUS: IN PROGRESS**

## Goal
Finish what 2026-07-22 left half-done: the prior session claimed "0 networkidle" but
~21 active `waitForLoadState('networkidle')` calls remain across 6 spec files; 4 tests
still skip because no offering is rentable; and `npx playwright test` still defaults to
the wrong port. Close those gaps, then run a live (no-mock) UX audit and persist findings.

## Method
PoC-first (per repo/AGENTS.md mandatory workflow) → RED test → GREEN fix → commit each unit.
No mocks in prod code. DRY/KISS/YAGNI. Greenfield.

---

## Phase 1 — Config trap: `npx playwright test` must Just Work (highest leverage)

**Problem:** `playwright.config.ts:6` — `baseURL = PLAYWRIGHT_BASE_URL || (autoStartServers ? 59010 : 59000)`.
Plain `npx playwright test` sets no env → `autoStartServers=false` → baseURL=59000 (Docker).
No Docker stack → tests "did not run" / connection refused. The actual dev default is the
warm stack (59010). The Docker mode should be opt-in, not the implicit default.

**Fix:** default baseURL to the warm-stack port (59010); make Docker mode set the env explicitly.
- `baseURL = PLAYWRIGHT_BASE_URL || 'http://localhost:59010'`
- `npm run test:e2e:docker` → prepend `PLAYWRIGHT_BASE_URL=http://localhost:59000`
- Same for apiURL default (59011 warm; docker sets 59001).
**Verify (PoC):** run ONE spec via bare `npx playwright test <spec>` (no env) against warm stack → passes.

## Phase 2 — Purge remaining `networkidle` (speed + correctness; prior session missed these)

`waitForLoadState('networkidle')` "tanks parallel runs under Vite HMR" (codebase's own words).
~21 active calls remain. Files + counts:
- `registration-flow.spec.ts` (6) — :30,:111,:156,:191,:234,:274
- `notification-settings.spec.ts` (4) — :76,:86,:95,:106 (claims "load-bearing"; verify, replace if possible)
- `keyboard-shortcuts.spec.ts` (4) — :6,:21,:35,:49
- `post-rental-welcome.spec.ts` (3) — :26,:63,:82
- `login-registration-cta.spec.ts` (3) — :6,:13,:25
- `auth-protection.spec.ts` (1) — :44
- `fixtures/auth-helpers.ts:66` (1) — documented load-bearing (SSR hydration); keep only if truly needed.

**Fix per call:** replace with deterministic wait — `waitForURL`, `waitForResponse(apiCall)`,
element visibility, or click-and-retry (the pattern already used in signin-flow). Commit file-by-file.

## Phase 3 — Close the 4 skip gaps via a rentable-offering fixture

**Root cause:** all 4 skips fire because no offering has an enabled Rent button, which requires
`provider_online=true`. `compute_provider_online_status` (offerings.rs:700) sets that field:
- `offering_source='self_provisioned'` → always online (line 756). **Cleanest fixture path.**
- Otherwise needs an agent pool + online agent matching the offering's region (heavy).

**Fix:** extend `seed-helpers.ts` `seedOffering()` to accept `offeringSource` override (default
unchanged); add a `seedProviderOnline(pubkeyHex)` helper that upserts `provider_agent_status`
(online=TRUE, last_heartbeat_ns=now, valid 5min per `heartbeat_cutoff`). Then:
1. `marketplace-empty-state.spec.ts` — seed an all-demo/offline state deterministically OR assert
   the reveal path when offerings exist. Make it run (not skip).
2. `payment-flows.spec.ts` (×3 at :183,:234,:283) — seed a self_provisioned public offering before
   each test so the Rent button is enabled; tests proceed to assert payment UI.
3. `post-rental-welcome.spec.ts` — rewrite the always-skipping stub (:21) against a real seeded
   contract; purge its 3 networkidle (Phase 2). Drop the first-party `page.route` mock (:93) per
   no-mock policy OR document the Stripe-boundary exception explicitly.

**Also fix latent bug:** `payment-flows.spec.ts:26,49,96` — `baseURL.replace('59000','59001')`
is a no-op against the warm stack (59010 has no '59000' substring) → would POST webhooks to the
web server, not API. Replace with a correct apiURL derivation (strip trailing path, swap port).

## Phase 4 — Speed: measure, then target levers

**Baseline:** 198 tests / 2.6m (156s) at 4 workers. Config comment says 8 workers no help
(bottleneck = sequential page loads vs single API+PG). Measure first, then:
- Confirm networkidle purge (Phase 2) reduced wall time.
- Look for redundant navigations / serializable auth reuse.
- Do NOT over-engineer sharding against a single stack.

## Phase 5 — Live UX audit (no mocks; required)

Browse the warm stack (59010) as a real user via browser tooling. Cover routes not yet audited
and re-verify prior "non-bug" triages still hold after the 07-22 changes. Look for: console
errors, dead links, broken flows, a11y regressions, new dead-ends. File new findings to
OPEN_ISSUES.md + GitHub issues for anything real.

## Phase 6 — Documentation
- Update `repo/docs/OPEN_ISSUES.md` (new findings + resolutions table).
- Update `repo/AGENTS.md` / `repo/website/AGENTS.md` if any harness convention changed.
- Mark this plan COMPLETE with results.

## Execution order
Phase 1 (config, unblocks trust) → Phase 3 (fixture, unblocks skip gaps) → Phase 2 (networkidle
purge, file-by-file) → Phase 4 (measure) → Phase 5 (audit) → Phase 6 (docs).
Each unit: PoC/RED → GREEN → commit.
