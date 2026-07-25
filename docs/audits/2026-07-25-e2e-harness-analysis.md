# E2E Harness Analysis — 2026-07-25

**Scope:** `website/tests/e2e/` (299 passing tests, ~6.6m, 2 workers, single warm Postgres).
**Mode:** READ-ONLY analysis. No code changed, no commits.
**Baseline:** full suite 299/0 green ~6.6m; smoke 23 tests ~29s (per `OPEN_ISSUES.md` 2026-07-24).
**Constraint respected:** sharding is NOT re-proposed (proven unhelpful on one Postgres — `OPEN_ISSUES.md` e2e tech-debt).

All items below are **NET-NEW** (not in prior audits `2026-07-23-e2e-coverage-speed.md` / `2026-07-24-coverage-and-ux-flow.md`, which are re-read and intentionally not duplicated).

---

## Prioritized table (value ÷ effort, descending)

| # | Cat | Title | Effort | Conf | Est. saving / gain |
|---|-----|-------|--------|------|--------------------|
| 1 | coverage | Verify-email SUCCESS path via DB-seeded token | S | 9 | closes ⚠️→✅; +1 test |
| 2 | reliability | search-dsl: stop depending on ambient demo seed data | S | 9 | removes a whole-spec flake-on-reset risk (8 tests) |
| 3 | dry+speed | Consolidate 7 inline-confirm delete specs → 1 parametrized | M | 9 | -6 tests, -~150 LOC dup, shares serial setup |
| 4 | dry | Promote `accountIdHex` (4 copies) → seed-helpers.ts | S | 10 | -~30 LOC dup |
| 5 | dry | Promote `email_verified=true` UPDATE (2 copies) + `assertNoNativeDialog` (5 copies) → helpers | S | 9 | -~40 LOC dup |
| 6 | coverage | Stripe `checkout.session.completed` webhook → contract activation | M | 8 | closes the money-path ⚠️; needs STRIPE_WEBHOOK_SECRET in warm stack |
| 7 | speed | route-audit: drop the blanket 700ms settle (≈31s of sleep) | S | 8 | -~15-25s wall (gated settle) |
| 8 | speed | Promote 5 fast/low-seed tests into `@smoke` | S | 8 | smoke covers more critical paths; ~0 net time |
| 9 | coverage | Cloud-accounts populated state via DB-seeded `cloud_accounts` row | S | 8 | closes ⚠️→✅; +1-2 tests |
| 10 | speed | offering-edit: share one seeded offering across 4 tests (beforeAll) | S | 7 | -3 seed/cleanup cycles |
| 11 | dry | Extract `confirmInlineAction(row, {arm, confirm})` helper (9+ uses) | S | 8 | -~80 LOC dup |
| 12 | reliability | Replace fragile `:has-text` button/heading selectors (registration:27, recovery:25, signin:18, anonymous:18, account-page:21) | L | 6 | copy-change resilience |
| 13 | coverage | Agent-pool detail-page edit (rename / provisioner change) | S | 7 | extends ✅ on `/dashboard/provider/agents/[id]` |
| 14 | coverage | Provider password-resets / ssh-key-rotations POPULATED state | M | 6 | extends ⚠️→✅ (needs backing-table ID) |
| 15 | ux-flow | Sign-in flow short-circuit when no Google OAuth (#436, deferred — capability endpoint path documented) | M | 7 | -2 clicks; e2e test + UX fix |
| 16 | ux-flow | Become-provider wizard: deep-link / auto-advance to step 3 | S | 7 | -2 clicks in onboarding |
| 17 | ux-flow | Offering edit as drawer/modal vs full navigation | L | 5 | faster edit loop |

---

## 1. Coverage gaps closeable WITHOUT external services

### [coverage] Verify-email SUCCESS path via DB-seeded token
**where:** `verify-email.spec.ts` (currently 2 tests, error branches only); backing logic `api/src/database/accounts.rs:242` (`verify_email_token`), table `email_verification_tokens(token, account_id, email, created_at, expires_at, used_at)`.
**problem:** FLOWS.md marks Verify email ⚠️ with "success-verify needs a real token". The token is NOT minted by an external email service at verification time — it is a plain DB row that the email-send flow writes. The verify handler just reads/marks-used that row. So the success path is fully exercisable by seeding the row DB-side.
**proposal:** New test in `verify-email.spec.ts`:
```
const token = randomHex(32);  // 32-byte token; verify_email_token takes &[u8]
// resolve the testAccount's account id (bytea) + email, then:
await sql(`INSERT INTO email_verification_tokens (token, account_id, email, created_at, expires_at)
           VALUES (decode('${token}','hex'), '<accountHex>', '<email>', ${nowSec}, ${nowSec + 86400})`);
await page.goto(`/verify-email?token=${token}`);
await expect(page.getByRole('heading', { name: 'Email Verified' })).toBeVisible();
// assert account row now has email_verified = true
```
Needs a small helper `seedVerificationToken(accountHex, email)` in seed-helpers.ts (returns the hex token). No external service, no mock.
**effort:** S  **confidence:** 9

### [coverage] Stripe `checkout.session.completed` webhook → contract `payment_status=succeeded`
**where:** `payment-flows.spec.ts` (defines `simulateStripeWebhook` but never calls it in a real test); backend `api/src/openapi/webhooks.rs:220` (`checkout.session.completed` handler).
**problem:** FLOWS.md marks Payment flows ⚠️ ("real checkout cannot complete in-harness"). But the BACKEND half of the payment path — webhook signature verification → `update_checkout_session_payment` → `payment_status` flip — needs no Stripe. Two issues with the existing scaffolding:
  1. `simulateStripeWebhook` (payment-flows.spec.ts:41) sends `payment_intent.succeeded`, which the backend **explicitly ignores** (`webhooks.rs:725`: "payment_intent.succeeded and payment_intent.payment_failed webhooks are NOT used. We use checkout.session.completed"). So even if it were called, it would exercise nothing.
  2. The webhook verifies the signature against `STRIPE_WEBHOOK_SECRET` (`webhooks.rs:195`), which is commented-out in `cf/.env.dev.example` (`#STRIPE_WEBHOOK_SECRET=whsec_test_secret`). Without it, the endpoint 500s before reaching the handler.
**proposal:**
  - Config: add `STRIPE_WEBHOOK_SECRET=whsec_test_secret` to the warm-stack env (either `cf/.env.dev` or `scripts/dev-server.sh` `--e2e` block — it already injects `RATE_LIMIT_ENABLED=false`). This is a dev-only secret; production stays real.
  - Rewrite `simulateStripeWebhook` to emit `checkout.session.completed` with `{ id: "cs_test_<ts>", metadata: { contract_id: "<hex>" } }`.
  - New test: seed a contract at `requested`/`pending` with `payment_status='pending'` (reuse `seedContract`) → POST the signed webhook → assert `payment_status='succeeded'` via `sql(...)` and that `stripe_checkout_session_id`/`stripe_payment_intent_id` were recorded.
  Closes the money-path ⚠️ without ever touching Stripe Checkout.
**effort:** M (config + rewrite helper + 1 test)  **confidence:** 8

### [coverage] Cloud-accounts POPULATED state via DB-seeded row
**where:** `cloud.spec.ts` (5 tests, all empty-state); tables `cloud_accounts`, `cloud_resources` (already written to by `seedRentableWithResource` in seed-helpers.ts:563).
**problem:** FLOWS.md marks Cloud accounts ⚠️ ("real cloud connect not asserted"). The `Add Account` modal options are tested, but a connected account never renders in the list. Connecting real Hetzner/Proxmox is out-of-scope, BUT `cloud_accounts` is just a DB row — `seedRentableWithResource` already inserts one (`backend_type='hetzner'`, `name`, `credentials_encrypted`). We can reuse that pattern to assert the populated list + the delete/disconnect flow.
**proposal:** New tests in `cloud.spec.ts`:
```
const { providerAccountIdHex } = await seedRentableWithResource({ name: 'E2E Cloud Acct', resourceCount: 0 });
// navigate to /dashboard/cloud/accounts, assert the row renders with the seeded name
// assert the Delete/Disconnect inline-confirm removes it (mirrors the 7 delete specs)
```
Needs a `resourceCount: 0` option on `seedRentableWithResource` (currently defaults to 4; the loop is trivial to skip). Cleanup via the existing `cleanupRentableWithResource`.
**effort:** S  **confidence:** 8

### [coverage] Agent-pool DETAIL edit (rename / provisioner change)
**where:** `agent-pool-revoke.spec.ts` (covers `/dashboard/provider/agents/[pool_id]` revoke only); the detail page also exposes pool edit.
**problem:** The pool detail page is now partially covered (revoke), but the EDIT affordances (rename, change provisioner/location) are untested. FLOWS.md "Agent pools" is ✅ but only for create.
**proposal:** Extend `agent-pool-revoke.spec.ts` (or a sibling) with a test that seeds a pool (the `seedPool` helper already exists), renames it via the signed PUT, and asserts the new name persists in the table + DB. Reuses `signedApiCall` from seed-helpers.
**effort:** S  **confidence:** 7

### [coverage] Provider password-resets / ssh-key-rotations POPULATED state
**where:** `provider-pages-smoke.spec.ts` (asserts empty-state copy for both).
**problem:** FLOWS.md marks Password resets ⚠️ ("empty-state render only"). Both pages render "No pending … requests." for a fresh provider. The populated branch (a request row with action buttons) is untested.
**proposal:** Identify the backing table/API for each page (the handler in `api/src/openapi/providers.rs`), add a `seed<KeyRotation|PasswordReset>Request(providerPubkey)` helper, and assert the row renders with its action button. Needs a short investigation of the data source first.
**effort:** M  **confidence:** 6 (blocked on confirming the backing table/API shape)

---

## 2. Slow tests / wall-clock reduction

### [speed] Consolidate 7 inline-confirm delete specs → 1 parametrized spec
**where:** `contact-delete`, `device-remove`, `external-key-delete`, `offerings-editor-replace`, `offering-delete`, `reseller-delete`, `social-delete`, `agent-pool-revoke` (8 specs, ~16 tests).
**problem:** These are structurally identical: seed a row → navigate → click Delete/Remove → assert inline Confirm/Cancel appear → (test 2) click Cancel → assert row kept. Each is its own file with its own `test.describe.configure({ mode: 'serial' })`, its own seed/cleanup helpers, and 4 of them duplicate `accountIdHex` verbatim.
**proposal:** One `inline-confirm-delete.spec.ts` with a table of entities:
```
const ENTITIES = [
  { name: 'contact',  route: '/dashboard/account/profile',     seed: seedContact,  rowSel: '...', arm: 'Delete',  ... },
  { name: 'device',   route: '/dashboard/account/security',    seed: seedDevice,   rowSel: '...', arm: 'Remove',  ... },
  { name: 'ext-key',  route: '/dashboard/account/profile',     seed: seedKey,      ... },
  { name: 'social',   route: '/dashboard/account/profile',     seed: seedSocial,   ... },
  { name: 'offering', route: '/dashboard/offerings',           seed: seedOffering, ... },
  { name: 'reseller', route: '/dashboard/provider/reseller',   seed: seedRelationship, ... },
];
for (const e of ENTITIES) {
  test(`${e.name}: arm reveals inline confirm; Cancel keeps row`, ...);
  test(`${e.name}: Confirm deletes row`, ...);  // skip for entities where Confirm is covered elsewhere
}
```
Net: 8 files → 1, ~16 tests → ~10 (drop redundant Cancel coverage for entities where the pattern is already proven), removes 4× `accountIdHex` + 5× `page.on('dialog')`. Keeps `offerings-editor-replace` and `agent-pool-revoke` separate (different UX: replace-guard / delegation).
**effort:** M  **confidence:** 9  **saving:** ~6 tests of overhead, simpler mental model.

### [speed] route-audit: drop the blanket 700ms post-content settle
**where:** `route-audit.spec.ts:242` (`await page.waitForTimeout(opts.settleMs ?? 700)`), runs once per route.
**problem:** The audit visits ~45 routes (8 public + 31 static authed + 6 dynamic). The 700ms settle fires on EVERY one = **~31.5s of pure sleep**, split across 2 workers ≈ 16s of critical-path idle. The settle exists to let client-side fetch + hydration land, but most routes are SSR-complete at `domcontentloaded` and the defect checks (leakage/slop/error-page) already work on SSR text.
**proposal:** Replace the blanket timeout with a content-gated settle: only sleep if a client-fetch indicator (e.g. the page issues a `/api/v1/` request that hasn't resolved) is in flight, else 0. Concretely: capture `page.on('request')` for `/api/v1/` during goto, and if any is pending, `waitForResponse` on it (bounded 2s); otherwise skip the settle entirely. The `checkStuckLoading` 3s grace already catches the "never resolved" tail.
**effort:** S  **confidence:** 8  **saving:** ~15-25s off the route-audit worker.

### [speed] Promote 5 fast/low-seed tests into `@smoke`
**where:** `FLOWS.md` smoke membership (23 tests, <30s). Selection rules: <5s, reliable, low-seed.
**problem:** Several eligible tests are not in smoke, leaving the fast dev loop thinner than it could be.
**proposal:** Add `@smoke` to (all verified <5s + no complex seeding):
  - `invoices.spec.ts` › `empty state: fresh user sees FAQ and marketplace CTA` (pure render, 0 seed)
  - `transfers.spec.ts` › `empty state: fresh user sees 0 balance and empty transfer list` (pure render, 0 seed)
  - `verify-email.spec.ts` › `shows a missing-token error…` (pure render, 0 seed, anonymous)
  - `error-page.spec.ts` › `404 renders branded error page…` (pure render, anonymous)
  - `rentals.spec.ts` › `filter tab: Cancelled tab shows empty-state message…` (1 seed, fast) — borderline; prefer the empty-state ones first.
  Re-run `npm run test:e2e:fast:smoke` to confirm still <30s (should be ~+4-6s).
**effort:** S  **confidence:** 8

### [speed] offering-edit: share one seeded offering across 4 tests
**where:** `offering-edit.spec.ts` (4 tests, each does `seedOffering` + `deleteOfferingsByProvider` in try/finally).
**problem:** Tests 1, 2, 4 are read-only on the offering (test 3 renames + submits). Each pays 1 INSERT + 1 DELETE + the psql round-trips (~100-200ms × 3 wasted). Serial mode already guarantees order.
**proposal:** Move seed to `beforeAll`, cleanup to `afterAll`. Order tests so the mutating "submit persists" runs LAST (or give it its own offering). Saves 3 seed/cleanup cycles.
**effort:** S  **confidence:** 7

---

## 3. DRY / consolidation

### [dry] Promote `accountIdHex` to seed-helpers.ts (4 verbatim copies)
**where:** `contact-delete.spec.ts:19`, `device-remove.spec.ts:24`, `external-key-delete.spec.ts:19`, `social-delete.spec.ts:19` — byte-identical 8-line function.
**proposal:** Add `export async function accountIdHex(username: string): Promise<string>` to `seed-helpers.ts` (next to `pubkeyHexFromSeed`); delete the 4 copies.
**effort:** S  **confidence:** 10

### [dry] Promote `email_verified=true` UPDATE + `assertNoNativeDialog` to helpers
**where:**
  - `UPDATE accounts SET email_verified = true WHERE id = (SELECT account_id FROM account_public_keys WHERE public_key = decode(...))` appears in `route-audit.spec.ts:615` and `rent-flow.spec.ts:70`.
  - `page.on('dialog', (d) => expect(d.type(), 'native dialog must not fire').toBe('never'))` appears in 5 specs (`contact-delete`, `device-remove`, `external-key-delete`, `offerings-editor-replace`, `social-delete`).
**proposal:**
  - `seed-helpers.ts`: `export async function verifyAccountEmail(pubkeyHex: string): Promise<void>` (or an option `{ verifyEmail: true }` on `seedAccountDirect` — preferred, since most specs want it true; only `dashboard-banners` + `account.spec.ts` want it false).
  - `auth-helpers.ts`: `export function assertNoNativeDialog(page: Page): void { page.on('dialog', d => expect(d.type(), ...).toBe('never')); }`.
**effort:** S  **confidence:** 9

### [dry] Extract `confirmInlineAction(row, { arm, confirm })` helper
**where:** The "click arm → assert Confirm/Cancel visible → click Confirm → wait for response" sequence appears in 9+ specs (the 7 delete specs + `rent-flow` cancel + `rentals` cancel + `rental-detail-cancel` + `provider-accept-reject`).
**proposal:** In `auth-helpers.ts`:
```
export async function confirmInlineAction(
  page: Page, row: Locator,
  opts: { arm: string; confirm?: string; waitForResponse?: string },
): Promise<void> { ... }
```
Collapses ~8 lines per call site to 1. Pairs with the consolidation in §2.
**effort:** S  **confidence:** 8

---

## 4. Reliability risks

### [reliability] search-dsl depends on AMBIENT demo seed data
**where:** `search-dsl.spec.ts:17` (`MARKETPLACE_URL = '/dashboard/marketplace?demo=1&offline=1'`), all 8 tests.
**problem:** The spec relies on the dev DB shipping demo offerings (comment line 13: "The dev DB ships only offline demo offerings"). The `seed-e2e-test-data.sh` script exists but is a MANUAL step — the spec never calls it. If the DB is reset (as happened 2026-07-24 for the ICPay migration) or demo offerings are missing, ALL 8 tests fail with no obvious cause. This is the single biggest ambient-data risk in the suite.
**proposal:** Self-seed at the top of the describe via the existing `seedRentableOffering` helper (which creates an always-online, non-example offering under a random pubkey). Add GPU + compute + storage variants so the type-filter and price-filter tests have known data. Drop the `?demo=1&offline=1` reliance. The DSL `type:gpu` test then matches the self-seeded GPU offering deterministically.
**effort:** S  **confidence:** 9

### [reliability] Fragile `:has-text` selectors on buttons/headings
**where:** Heaviest in `registration-flow.spec.ts` (27), `recovery-flow.spec.ts` (25), `signin-flow.spec.ts` (18), `anonymous-browsing.spec.ts` (18), `account-page.spec.ts` (21), `admin-dashboard.spec.ts` (15), `billing-settings.spec.ts` (16).
**problem:** `:has-text` matches any element containing the substring and is Playwright-engine (not W3C) CSS — a copy change ("Sign In" → "Log in") silently breaks the test. The repo already standardizes on `getByRole`/`getByLabel` elsewhere (e.g. the delete specs use `getByRole('button', { name: 'Confirm' })`).
**proposal:** Sweep the highest-count specs first; convert button/heading/link selectors to `getByRole({ name })`. Leave `:has-text` only where it scopes a row by its seeded data text (legitimate). This is the largest single effort but the highest copy-resilience payoff.
**effort:** L  **confidence:** 6

### [reliability] `offerings-editor-replace` replace-bar selector is class-based
**where:** `offerings-editor-replace.spec.ts:31` (`page.locator('div.flex.items-center.gap-3.flex-wrap').filter({ hasText: 'Replace existing data' })`).
**problem:** Tailwind class chains are an implementation detail; any restyle breaks the test. The bar has no `data-testid`.
**proposal:** Add a `data-testid="replace-confirm-bar"` to the replace bar in `OfferingsEditor.svelte` and select on that. (One-line product change + one-line test change.)
**effort:** S  **confidence:** 8

---

## 5. UX-flow optimization opportunities (UX fix + e2e test)

### [ux-flow] Sign-in short-circuit when Google OAuth is unconfigured (#436, deferred)
**where:** `src/routes/login/+page.svelte`; issue #436 already documents three paths.
**problem:** Sign-in is 6 clicks/steps: goto `/login` → "Sign in with seed phrase instead" → "Import Existing" → paste → "Continue" → "Go to Dashboard". When `GOOGLE_OAUTH_CLIENT_ID` is unset (server-side, `main.rs:1024`), the seed-phrase path is the ONLY path, yet it's hidden behind a toggle. This also slows every UI-signin e2e test.
**proposal:** Implement path (1) from the issue comment: a `GET /api/v1/capabilities` endpoint returning `{ google_oauth: bool, ... }`; the login page shows the seed-phrase form directly when `google_oauth=false`. The e2e (`signin-flow.spec.ts`) drops the `revealSeedPhraseOptions` step → faster + more honest UX. Issue is labeled `deferred-post-launch` but the e2e-speed + UX win is concrete.
**effort:** M  **confidence:** 7

### [ux-flow] Become-provider wizard: deep-link / auto-advance to the relevant step
**where:** `src/routes/dashboard/provider/support/+page.svelte`; `provider-onboarding-submit.spec.ts:44` clicks "Save & Continue" twice before the Help Center form appears.
**problem:** A returning provider (step persisted in localStorage) lands back on step 3 — good. But a FIRST-TIME provider must click through steps 1 and 2 even if they only want to fill the Help Center profile. The wizard has no `?step=3` deep-link.
**proposal:** Support `?step=N` on the wizard route (reads + validates against the persisted max step) so a CTA can deep-link. Add an e2e that navigates `?step=3` directly and asserts the form renders (drops 2 clicks from the test).
**effort:** S  **confidence:** 7

### [ux-flow] Offering edit as inline drawer/modal vs full page
**where:** `src/routes/dashboard/offerings/[id]/edit/+page.svelte`; covered by `offering-edit.spec.ts`.
**problem:** Editing an offering navigates away from the offerings list, losing scroll position + list context. The edit form already has an inline "Changes Since Last Save" diff panel, so the UX is half-inline already. A drawer would keep the list visible.
**proposal:** (Product judgment, larger) Convert the edit page to a slide-over drawer on the offerings list route. The e2e would then assert the list stays mounted behind the drawer. Lower confidence — this is a design call, not a clear defect.
**effort:** L  **confidence:** 5

---

## Non-findings (checked, deliberately NOT proposed)

- **Sharding** — proven unhelpful on one Postgres (`OPEN_ISSUES.md`); not re-proposed.
- **`waitForTimeout` outside fixtures** — grep confirms ZERO stray calls in specs (only `clickAndRetry` 100ms and `route-audit` settle/stuck-loading, both justified). Clean.
- **Ambient `uxaudit`/`compute-001`/seed_data references** — grep finds only historical mentions in comments; `reputation-detail` was already fixed to self-seed. Clean.
- **`networkidle`** — suite is genuinely 0 (per `OPEN_ISSUES.md` 2026-07-23). Clean.
- **Rent→pay→view→cancel UI-created-contract gap** (`OPEN_ISSUES.md` e2e gap note) — STALE; `rent-flow.spec.ts` now covers it end-to-end (dialog → real POST → list → detail → cancel). The only remaining gap is the Stripe-Checkout completion, addressed in item #6.
