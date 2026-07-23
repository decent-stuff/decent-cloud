# E2E Harness Hardening + Skip-Gap Closure + UX Audit (2026-07-23)

**STATUS: COMPLETE** — 7 commits, 202 passed / 0 failed / 0 skipped, 0 `networkidle`, 152s.

## Goal
Finish what 2026-07-22 left half-done: the prior session claimed "0 networkidle" but
~21 active `waitForLoadState('networkidle')` calls remained across 6 spec files; 4 tests
still skipped because no offering was rentable; and `npx playwright test` still defaulted to
the wrong port. Closed those gaps, fixed a config trap and a latent bug, then ran a live
(no-mock) UX audit and persisted findings.

## Method
PoC-first (per repo/AGENTS.md mandatory workflow) → RED test → GREEN fix → commit each unit.
No mocks in prod code. DRY/KISS/YAGNI. Greenfield.

---

## Phase 1 — Config trap: `npx playwright test` now Just Works ✓ (3f7f9512)

**Problem:** `playwright.config.ts:6` — `baseURL = PLAYWRIGHT_BASE_URL || (autoStartServers ? 59010 : 59000)`.
Plain `npx playwright test` sets no env → `autoStartServers=false` → baseURL=59000 (Docker).
No Docker stack → tests "did not run" / connection refused. The actual dev default is the
warm stack (59010). The Docker mode was the implicit default; it should be opt-in.

**Fix:** default baseURL to the warm-stack port (59010); make Docker mode set the env explicitly.
- `baseURL = PLAYWRIGHT_BASE_URL || 'http://localhost:59010'`; `apiURL` default → 59011.
- `npm run test:e2e:docker` → prepends `PLAYWRIGHT_BASE_URL=http://localhost:59000`.
**Verify:** bare `npx playwright test anonymous-browsing.spec.ts` (no env) against warm stack → 9 passed 6.1s.

## Phase 2 — Purged ALL `networkidle` (19 calls across 6 files) ✓ (e59e76d4)

The prior session's "0 networkidle" claim in `website/AGENTS.md` was inaccurate (off-by-date);
fixed the date to 2026-07-23 and pointed the anti-pattern note at the new helper. Verified: 5
affected specs pass 22/22 in 34s against the warm stack. **The suite now genuinely has zero networkidle.**

**New shared helper `clickAndRetry(page, target, success)`** (`fixtures/auth-helpers.ts`): retries
`target.click({timeout:5000}).catch(()=>{})` up to 20× (100ms apart) until `success` (Locator or
predicate) is satisfied. Canonical fix for SSR'd SvelteKit buttons whose onclick binds only on
hydration (pre-hydration click = silent no-op). `revealSeedPhraseOptions()` refactored to delegate.

Per-file: registration-flow (6→`revealSeedPhraseOptions`), login-registration-cta (3), notification-settings
(4→`clickAndRetry`), keyboard-shortcuts (4→`waitForResponse('/api/v1/offerings')` before goto), auth-protection (1).

## Phase 3 — Closed all 4 skip gaps via rentable-offering fixture ✓

**Root cause:** all 4 skips fired because no offering had an enabled Rent button, which requires
`provider_online=true`. `compute_provider_online_status` (offerings.rs:700) sets that field;
`offering_source='self_provisioned'` → always online (line 756) — the cleanest fixture path.

**Fixture (6b8bafad):** `seedRentableOffering(overrides?)` in `seed-helpers.ts` — generates a random
non-example provider pubkey, seeds a self_provisioned public offering, returns the offering handle.
Plus `deleteOfferingsByProvider(pubkeyHex)` cleanup, `currency?: string` override (default ICP).

- **payment-flows.spec.ts ×3 (b7c05d17):** root cause was a stale selector — tests probed
  `button:has-text("Rent Resource")` which never existed (the marketplace action button reads **"Rent"**;
  "Rent Resource" is the dialog title). Rewrote with `seedRentableOffering` + `openRentalDialog` helper
  (scopes the Rent button to the offering's `<tr>` via `tr.filter({hasText})`). Seeded 2 offerings
  (ICP + USD) so the ICPay wallet-guard (needs `VITE_ICPAY_PUBLISHABLE_KEY`, injected into dev-server.sh)
  and the Stripe Checkout redirect path (USD-only) are both reachable. Also fixed the latent
  `baseURL.replace('59000','59001')` URL bug (no-op against warm stack 59010) in the webhook helpers.
- **post-rental-welcome.spec.ts (b64effe0):** rewrote against real UI (`data-testid="welcome-banner"`
  gated on `?welcome=true` + seeded contract). Old spec used nonexistent selectors + mocked first-party
  `/contracts/verify-checkout`. 3 tests; removed the mock-requiring checkout-success test (redirect
  logic covered by `src/routes/checkout/success/page.test.ts`).
- **marketplace-empty-state.spec.ts (0978c404):** rewrote against the real default-hide path
  (all 10 seed offerings are `is_example` → hidden by default → empty state + "Show N offerings"
  reveal). No seeding needed. 1 test.

## Phase 4 — Speed baseline measured + flake fixed ✓ (995ac799)

**202 passed, 0 failed, 0 skipped, 0 networkidle, 152s (2.5m) wall, 4 workers.** The networkidle
purge did not meaningfully move wall time (the suite was already efficient); its value is
determinism/correctness.

**Flake found & fixed:** `search-dsl.spec.ts:156` (`type:gpu`) read `tbody tr.count()` immediately
after `await waitForResponse` (HTTP receipt) — a render-gap race under parallel load. Fixed by gating
on a GPU row rendering before counting. Re-ran full suite → 202 passed 152s, green.

**Speed assessment (<60s target):** `playwright.config.ts:32-36` already documents 8 workers = no gain
(box 64% idle, not CPU/Vite/DB-pool-bound). Bottleneck = sequential browser-driven page loads against a
single API+Postgres stack. 152s/202 tests = 0.75s/test wall (3s/test worker-time) is already efficient.
The <60s goal realistically needs multi-stack sharding (Playwright `--shard` across N warm stacks) — an
infra investment beyond harness-hardening scope. Deferred as tech debt (see OPEN_ISSUES.md).

## Phase 5 — Live UX audit (no mocks) ✓ — UI is CLEAN

Audited the warm stack (web:59010, api:59011) via `scripts/browser.js` (Playwright Chromium; no system
Chrome binary present) + `zai-vision` screenshot analysis. Seeded stable audit account `uxaudit`.
Pages checked: landing, login, marketplace, dashboard home, dashboard marketplace, account/profile,
account/billing, cloud/resources, invoices, provider/agents.

**Console errors:** landing/login clean (0); dashboard pages show only known dev-only warnings
(Lit dev mode, Stripe.js-over-HTTP). The `net::ERR_ABORTED` on `.svelte-kit/generated/*` and
`@vite/client` are Vite HMR module-cancellation noise (fresh-browser-per-call artifact); do not
appear in e2e runs.

**zai-vision flagged 4 items — ALL false positives (verified via CSS):**
1. Landing "Decentralized Cloud" badge low-contrast → FALSE (7.3:1, exceeds WCAG AAA).
2. Profile "Current Identity" labels low-contrast → FALSE (7.3:1; zai-vision misread dark-theme cards).
3. Marketplace trending card name truncated → intentional `truncate` class (full name on detail page).
4. Billing "placeholder text" → normal empty-textarea placeholder, correct UX.

**Conclusion:** No actionable product defects. Dark-theme contrast is AAA-compliant.

**Tooling note (dev-script, not product):** `browser.js eval --seed <phrase>` throws
"UtilityScript.evaluate" (the extra `goto`+`networkidle` in `authenticatePage` likely destroys the
eval context via SvelteKit client redirect). `snap`/`shot`/`errs`/`html`/`tour` all work with `--seed`;
only `eval` is affected. For authed JS eval, use the e2e framework instead. Tracked in OPEN_ISSUES.md.

## Phase 6 — Documentation ✓
- Updated `repo/docs/OPEN_ISSUES.md` (2026-07-23 session results + new tech-debt items).
- Updated `repo/website/AGENTS.md` networkidle anti-pattern note (date + helper pointer).
- This plan marked COMPLETE with results.

## Commits this session
| Commit | Phase | Summary |
|--------|-------|---------|
| 3f7f9512 | 1 | fix(e2e): default baseURL to warm stack so bare 'npx playwright test' works |
| 6b8bafad | 3 | test(e2e): add seedRentableOffering fixture for skip-gap closure |
| b7c05d17 | 3 | test(e2e): close payment-flows skip gaps via rentable-offering fixture |
| b64effe0 | 3 | test(e2e): rewrite post-rental welcome banner against real UI |
| 0978c404 | 3 | test(e2e): rewrite marketplace empty-state against real default-hide path |
| e59e76d4 | 2 | test(e2e): purge all remaining networkidle waits for deterministic hydration |
| 995ac799 | 4 | fix(e2e): gate type:gpu count on render to close parallel flake |
