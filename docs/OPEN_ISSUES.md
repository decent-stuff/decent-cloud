# Open Issues

**Snapshot:** 2026-07-24. **Canonical source:** GitHub Issues at `decent-stuff/decent-cloud`
(`gh issue list --repo decent-stuff/decent-cloud --state open`). This file is a categorized
inventory for quick local reference; GitHub remains the source of truth. Re-sync with:

```bash
gh issue list --repo decent-stuff/decent-cloud --state open --json number,title,labels
```

## Scope rules (per `repo/AGENTS.md` + `repo/PROMPT.md`)

- **In scope**: labeled `launch`, `stripe`, or `decent-agents` WITHOUT `deferred-post-launch`.
- **Deferred**: labeled `deferred-post-launch`. Valid but parked until ≥20 paying customers.

## In scope (active work)

| # | Title | Labels | Notes |
|---|-------|--------|-------|
| 418 | Decent Agents: beta onboarding (invite + first-run demo) | launch | First user-facing DA flow. Large (magic-link/Google auth → Stripe → GitHub App → demo PR → invite gate). |
| 427 | Anthropic API key proxy/sidecar for per-identity isolation | decent-agents, launch | Required for multi-tenant DA isolation. Large. |
| 416 | Decent Agents: usage metering + customer-facing usage dashboard | decent-agents | Depends on #415 meters. Large. |
| 415 | Decent Agents: subscription billing with active-hour + Claude token caps | decent-agents | Meters, caps, Stripe cycle rollover. Large. |

## Deferred — Decent Agents

| # | Title |
|---|-------|
| 432 | Decent Agents: per-identity observability + incident response runbook |
| 431 | Decent Agents: GitHub App webhook secret rotation procedure + ops runbook |
| 430 | Decent Agents: CODEOWNERS / branch protection deadlock surfaced to customer at onboarding (also launch) |
| 429 | Decent Agents: Anthropic key exfiltration mitigation (read-only mounts, egress monitoring) |

## Deferred — Stripe / billing

| # | Title |
|---|-------|
| 441 | Boot gate asymmetry: `require_stripe_in_prod` exists but no `require_icpay_in_prod` |
| 426 | Test: out-of-order Stripe webhook delivery (dispute.created before checkout.session.completed) |
| 425 | Audit existing Provisioning → Cancelled failure paths and migrate to ProvisioningFailed |
| 420 | ICPay: implement automated payouts when ICRC-1 transfer API ships |

## Deferred — UX

| # | Title | Filed by |
|---|-------|----------|
| 442 | Subscription Pro/Enterprise cards advertise "14-day free trial" but CTA is only "Contact Sales" | 2026-07-24 UX audit |
| 443 | Create-offering wizard: auto-suggest monthly price from Hetzner server cost | 2026-07-24 UX-flow audit |
| 436 | Seed-phrase sign-in hidden behind extra click when no Google OAuth configured | 2026-07-20 UX audit |

> **#436 implementation note (2026-07-21):** The issue's suggested gate
> (`!import.meta.env.VITE_GOOGLE_OAUTH_CLIENT_ID`) does not apply — that Vite env
> var does not exist. Google OAuth is gated **server-side** via the Rust env var
> `GOOGLE_OAUTH_CLIENT_ID` (`api/src/main.rs:1024`, conditionally registers the
> route at `:1334`). The frontend has no signal. Three implementation paths
> documented in [issue comment 5035263075](https://github.com/decent-stuff/decent-cloud/issues/436#issuecomment-5035263075):
> (1) capability endpoint [recommended], (2) runtime HEAD probe, (3) build-time
> Vite var. Product call.

## Deferred — Tech debt / low-value

| # | Title |
|---|-------|
| 444 | Tech debt: split large source files (>2000 lines) into logical modules |
| 387 | Concurrent multi-ticket processing via multiprocessing + worktrees |
| 382 | dc-agent: remove `try_trigger_hetzner_provisioning` backward-compat alias |
| 373 | DRY refactor: `extract_contract_id()` shared across 3 provisioners |
| 344 | dc-agent: additional MOCK tests for Docker provisioner (P2) |
| 334 | Code: Add tests for database modules without dedicated test files |
| 214 | dc-agent: `verify_setup()` check for default_image existence (P2) |
| 212 | dc-agent: pre-built Docker image with openssh-server (P2) |
| 107 | Backlog: Dark/light mode toggle |

## Recently closed by this work

### 2026-07-24 session (fresh sweep: robustness + UX + coverage + create-bug)

Three read-only audits (`docs/audits/2026-07-24-{fresh-ux,code-robustness,coverage-and-ux-flow}.md`) → triaged, shipped high-confidence fixes via TDD, parked product decisions as #441-#444.

| Fix | Area | Resolution |
|-----|------|------------|
| Missing reqwest timeouts across money/identity/provisioning | Robustness | Shipped in `6cc6199c`: shared `http_client()` helper (30s timeout) in new `api/src/http_util.rs`; replaced ALL bare `Client::new()` in stripe/icpay/oauth/cloudflare/invoices/llm/chatwoot/embeddings/price-cache/vies/telegram/sms + 12 api-cli sites + `api_cli/client.rs`; `.timeout(120s)` on both dc-agent upgrade builders. |
| Silent hex-decode in receipts (refund + accept notifications) | Robustness | Shipped in `c66bd3f9`: `receipts.rs:297/418` `if let Ok`=hex::decode → `match` + `tracing::warn!` (contract id, parse error, bad value). |
| typst subprocess no timeout | Robustness | Shipped in `5ad502a5`: `invoices.rs` typst `.output()` wrapped in `tokio::time::timeout(30s)`. |
| Silent dispute-hex fallthrough | Robustness | Shipped in `707f0d97`: `webhooks.rs:779` → `match` + warn. |
| Hardcoded Stripe URL | DRY | Shipped in `f4357348`: `const STRIPE_API_BASE`. |
| Dead `network_metrics` module | Tech debt | Shipped in `4b472e73`: deleted unreferenced module (`load_ledger_metrics`). |
| Inconsistent hex::decode path boilerplate (~40 sites) | DRY | Shipped in `164bbdb4`: shared `decode_hex_path`/`decode_pubkey` in `openapi/common.rs`; unified terse→detailed error msgs. |
| Reputation "Poor" badge for zero health checks | UX | Shipped in `6df2155e`: neutral "No health checks yet" badge when `totalChecks===0`. Same class as #435. |
| Stale `© 2025` footer | UX | Shipped in `4b6659c0`: dynamic `{new Date().getFullYear()}`. |
| Breadcrumb "Dashboard" → /dashboard/rentals mismatch | UX | Shipped in `757bd79b`: relabeled "My Rentals". |
| Orphaned `/dashboard/user/[id]` route | UX | Shipped in `4292bdc9`: 307 redirect to reputation page (matches marketplace pattern). |
| Command palette had zero provider actions | UX | Shipped in `165b6720`: Create Offering/My Offerings/Agent Pools/Billing Settings gated on auth. |
| ALL native `confirm()` dialogs (6 dashboard + 5 components) | UX + e2e | Shipped across `1077dd33`,`fa82ec0e`,`41491746`,`b4ad6b61`,`d6acdf94`,`938d6c83`,`24924b51`,`d6425c10`,`d2bd52c3`,`8e348415`,`dc8ee2f3`: every native `confirm()` → inline two-step (request/confirm/cancel + pendingId). `rg "confirm\(" website/src` = 0 live calls. Unblocks headless e2e + mobile UX + consistency. |
| Create-offering 400 on every UI create (#440) | Bug (critical) | Shipped in `ebebff02`: poem-openapi ignores `#[serde(default)]`; applied `#[oai(default)]` to `Offering.pubkey` so missing field deserializes, then handler overwrites from URL path. |
| E2E coverage: 7 documented gaps closed | Coverage | add-device `@smoke` (`0730350e`), compare `@smoke` (`18b4a35b`), agent-pool (`f7b38826`), earnings (`dc84a706`), onboarding (`5f2ca8d4`), admin mutations ❌→✅ (`157ec457`), create-offering (`ebebff02`). New seed-helpers: `deleteContractsByProvider`, `deleteAgentPoolsByProvider`, `deleteProviderProfileByPubkey`, `signedApiCall`, `identityFromSeedPhrase`. |
| Stale test assertion (unified pubkey error msg) | Test | Shipped in `54c1e54d`: `provider-response-metrics.spec` asserted old terse msg; updated to `toContain('Invalid pubkey hex')` + echoed bad value. |
| Full suite baseline | — | **300 passed, 0 failed, 5.6m, 4 workers** (was 267 at session start; +33 tests from coverage closures + confirm-conversion specs). |

### 2026-07-23 session (money-safety hardening + route audit + UX review)

| Fix | Severity | Resolution |
|-----|----------|------------|
| R1: provider can drive requested→active unpaid | Critical | Shipped in `e6b5441e`: `update_contract_status` gates Provisioned/Active on `payment_status='succeeded'` OR `payment_amount_e9s=0`. Migration 048 DB CHECK. |
| R2/R3: refund+release unbounded + TOCTOU | Critical | Shipped in `45d40d82`: migration 049 CHECK `released+refund<=payment`; conditional UPDATE release path; `reject_contract`→`calculate_net_refund_e9s`. |
| R5: "refunded" with no money returned | Critical | Shipped in `6b3ad47e`: callers treat `Ok(None)` as "refund NOT performed"; `STRIPE_SECRET_KEY` required when `ENVIRONMENT=prod`. |
| R9: dispute-lost refund over-pays released funds | High | Shipped in `46edc93c`: `process_dispute_lost_refund` uses `calculate_net_refund_e9s` (subtracts `total_released_e9s`). |
| R10: `payment_status` accepts any string | High | Shipped in `220c2a82`: allow-list in code + migration 047 DB CHECK. |
| SSE auth double-prefix bug (`/api/v1/api/v1/...`) | Critical | Shipped in `d5a2e019`: SSE handlers verified against REAL request path. Was masked by env var bug. |
| Cluster A: SSE 404s (wrong env var `VITE_API_BASE_URL`) | High | Shipped in `02affbf7`: import `API_BASE_URL` from `api.ts` (2 pages). |
| B1: contract usage 401 (wrong signature path) | Medium | Shipped in `f40e35eb`: sign for correct `/contracts/{id}/usage` path. |
| B2: pending-password-reset 401 (agent-only auth) | Medium | Shipped in `e7519ee4`: `AgentAuthenticatedUser` → `ProviderOrAgentAuth`. |
| B3/B4: user activity 401 (own-only endpoint on public pages) | Medium | Shipped in `6b4d36e2`: new `GET /users/:pk/public-profile` with `PublicContractSummary` (no payment/SSH/gateway). |
| Command palette keyboard nav completely broken | High | Shipped in `09097cda`: arrows/Enter/Escape now work; visible Cmd/Ctrl+K trigger in sidebar. |
| Provider shown as raw hex pubkey on rentals | Medium | Shipped in `1921474b`: contracts query LEFT JOINs username; UI shows `@username`. |
| Test-infra: hardcoded 49-entry migration array | Tech debt | Shipped in `51131bfd`: replaced with `sqlx::migrate!()` (+37/−290 lines). |
| E2e smoke tier: only 4 tests | Coverage | Shipped in `0c033da2`+`4ead5c05`: 17 `@smoke` tests in 18s; scripts scan via `--grep`. |
| E2e: no flow catalog | Coverage | Shipped in `517b97cb`: `FLOWS.md` — 74 flows cataloged. |
| E2e: provider accept/reject uncovered | Coverage | Shipped in `dd955f4f`: 4 serial tests (see pending, accept, reject, auto-accept toggle). |
| Full suite baseline | — | **264 passed, 0 failed, 3.8m, 4 workers.** |

### 2026-07-23 session (e2e radical overhaul + issue sweep + sharding harness)

| Fix | Severity | Resolution |
|-----|----------|------------|
| `reputation-detail.spec.ts` hardcoded `uxaudit` pubkey (drifted after re-seed) | Fragile | Shipped in `c8e25e3a`: self-contained test seeds its own account, derives pubkey, asserts, cleans up. Was the 1 failure in a re-baselined 201/1 suite. |
| F1: `/dashboard` 'Get Started' CTA → `/dashboard/provider` 404s (no such route) | High | Shipped in `9dad0734`: href → `/dashboard/provider/support` (the setup wizard). The only broken link in the dashboard; 19/20 internal hrefs resolve. TDD RED→GREEN. |
| F2: onboarding modal gated on `sessionStorage` (reappears each browser session) + always said 'Complete your profile' even when complete | Medium | Shipped (F2): switched to `localStorage` (`WelcomeModal.svelte`); dynamic copy 'Your profile is ready' when username+email both set. Fixtures + existing onboarding tests updated. |
| CLI: dead `dialoguer` dependency (never used) | Tech debt | Shipped in `c29173b5`: removed from `cli/Cargo.toml` + workspace `Cargo.toml`. |
| CLI: 20 fake string-literal tests (asserted Display strings, never invoked the binary) | Tech debt | Shipped in `db5997cd`: replaced with 10 real `assert_cmd` subprocess smoke tests (`--help`/`-V`, keygen generate/import, ledger-local list, network dispatch, clap validation). Binary e2e coverage 0%→real. Net 39→29 tests. |
| saved-offerings + offering-detail-save hardcoded seed_data IDs/names | Fragile | Shipped (fragile commit): both specs now seed their own offerings under a random pubkey. |
| `account.spec.ts` seeded account with no cleanup (orphaned rows/run) | Fragile | Shipped (fragile commit): added `deleteAccountByUsername` in finally. |
| `recovery-flow.spec.ts` 2× `waitForTimeout(100)` sleeps | Fragile | Shipped (fragile commit): replaced with `waitForResponse` on the recovery API. |
| Sharding harness built + two blockers it exposed | Infra | Shipped in `297009d9`: dev CORS now allows any `localhost/127.0.0.1:*` origin (was a static list — shard ports 403'd); service worker no longer intercepts non-navigate fetches (was masking real API errors as 503); new `fixtures/api-base.ts` resolves API URL from stack port (4 specs hardcoded 59011). |
| Offering EDIT flow `/dashboard/offerings/[id]/edit` — zero coverage | Coverage | Shipped in `c97a497d`: 4 e2e tests (pre-fill, live diff panel, submit+redirect+DB persistence, validation). No source bug found. |
| Full suite baseline | — | **209 passed, 0 failed, 0 skipped, 0 networkidle, ~4.5m, 4 workers** (single warm stack). |

### 2026-07-23 session (e2e harness hardening + skip-gap closure + UX audit)

| Fix | Severity | Resolution |
|-----|----------|------------|
| `npx playwright test` defaulted to Docker port (59000), not warm stack | Test | Shipped in `3f7f9512`: `baseURL` now defaults to warm-stack 59010; Docker mode sets env explicitly. Bare `npx playwright test` Just Works. |
| 4 always-skipping e2e tests (payment-flows ×3, post-rental-welcome, marketplace-empty-state) | Test | Shipped in `6b8bafad`/`b7c05d17`/`b64effe0`/`0978c404`: new `seedRentableOffering` fixture (self_provisioned → always online). payment-flows root cause was a stale selector ("Rent Resource" button never existed; button reads "Rent"). post-rental-welcome rewritten against real `?welcome=true` banner + seeded contract (dropped first-party verify-checkout mock). marketplace-empty-state rewritten against the real default-hide path. **0 skipped now (was 4).** |
| ~19 active `networkidle` calls across 6 specs (prior "0" claim was inaccurate) | Test | Shipped in `e59e76d4`: replaced all with deterministic waits via new `clickAndRetry` helper (SSR-hydration-safe click loop) + `waitForResponse`. **Suite now genuinely 0 networkidle.** |
| payment-flows webhook helpers POST to wrong server | Test | Shipped in `b7c05d17`: `baseURL.replace('59000','59001')` was a no-op against warm stack 59010 → would POST webhooks to the web server. Now uses `PLAYWRIGHT_API_URL \|\| 59011`. |
| search-dsl `type:gpu` test flaked under parallel load | Test | Shipped in `995ac799`: `count()` immediately after `waitForResponse` hit a render gap. Gated on a GPU row rendering first. |
| Full suite baseline | — | **202 passed, 0 failed, 0 skipped, 0 networkidle, 152s, 4 workers.** |
| Live UX audit (10 pages, no mocks) | — | No actionable defects. zai-vision's 4 flags were all false positives (dark-theme contrast is 7.3:1 = WCAG AAA; truncation is intentional). Console clean apart from known dev warnings. |

### 2026-07-22 session (e2e harness overhaul + UX fixes)

| Fix | Severity | Resolution |
|-----|----------|------------|
| C1: Marketplace shows 0 offerings (demo/offline hidden by default, dead-end empty state) | Critical | Shipped in `a2ed9fd1`: split filter chain into `userFiltered` + `defaultHiddenCount`. Empty state now shows one-click 'Show N offerings' reveal button when defaults are the only cause. |
| C2: Profile page crashes ('No account username found' race) | Critical | Shipped in `67efb570`: `UserProfileEditor` takes `username` prop (no throw). Profile page guards on `currentIdentity?.account`. |
| H1: Billing spending-alerts renders raw 'not found' (endpoints missing) | High | Shipped in `6d589c5f`: removed `#[cfg(test)]` from `upsert/delete_spending_alert`, added GET/PUT/DELETE `/users/:pubkey/spending-alert` routes in `users.rs`. `api.ts` treats 404 as null. |
| H5: Login lacks discoverable registration path (Generate New hidden) | High | Shipped in `39962212`: added 'New here? Create an account' CTA on login page; `initialSeedMode` state jumps directly to generate step. |
| M1: Dashboard shows provider metrics (Trust 90, Red Flags) to non-providers | Medium | Shipped in `63d0ac4a`: TrustDashboard gated on `userRole === 'provider'` via `detectUserRole()`. |
| M2: Marketplace 'Category:' label mislabeled (holds regions/price) | Medium | Shipped in `4665ada2`: renamed to 'Quick filters:'. |
| M4: Email+seed banners clutter 19/22 pages | Medium | Shipped in `dda320cf`: email banner now has per-session dismiss button (same pattern as seed banner). |
| Invoices parallelism flake (DB state shared via testAccount pubkey) | Test | Shipped in `85cd37ec`: added `test.describe.configure({ mode: 'serial' })` to invoices.spec.ts. |
| E2E suite: 0 `networkidle` calls, 0 `registerNewAccount` in API tests | Test | 13 commits: replaced 14 networkidle + 4 registerNewAccount with deterministic waits/seedAccountDirect. Extracted 4 DRY helpers. Consolidated 12 tests→4. |
| E2E coverage: 17 GAP routes, 4 THIN flows | Test | 8 new spec files (18 tests): verify-email, agents-pricing, become-provider, reputation-detail, account-subscription, provider-pages-smoke (8 routes), account-profile-edit, provider-requests-auth. |
| UX: marketplace `/` keyboard shortcut for search focus | UX | Shipped in `dda320cf`: `/` focuses marketplace search (with visible `<kbd>` hint). Ignores when already in input/textarea. |

### 2026-07-21 session (prior)

| # | Title | Resolution |
| 437 | Marketplace: click-to-cycle visibility/stock buttons are surprising | Shipped in `e2d28e6e`: new `OfferingStatusMenu.svelte` (button + conditional panel, per-card mutual exclusion via `globalThis.__offeringStatusMenus` registry, click-outside/ESC/Enter/Space a11y, `role="menu"` + `role="menuitemradio"`). Panel auto-flips up to avoid overlapping the wrapped stock trigger. Load switched to `getMyOfferings` so owners see shared/private offerings (was filtered out by the public endpoint). E2E in `offerings-status-menus.spec.ts` (4 tests). |
| 438 | Dashboard layout: email banner preempts seed-phrase backup banner (recovery risk) | Shipped in `29efa840`: banners now render as static-block siblings inside one fixed container (each independently dismissable). `mainTopPadding` derived expr picks the right offset for both/one/none. `EmailVerificationBanner`/`SeedPhraseBackupBanner` lost their per-component positioning. E2E in `dashboard-banners.spec.ts` (4 tests). |
| 439 | Marketplace: sort UI hidden on mobile (`hidden md:flex`) | Shipped in `698329f7`: added `<select aria-label="Sort offerings">` next to the pill row. Pills unchanged on desktop; select is the mobile-only affordance + a11y alternative. Both bind to the same `sortField`/`sortDir` state and reuse `syncFiltersToUrl()`. E2E in `marketplace-sort.spec.ts` (3 tests). |
| 435 | Offering detail SLA chart renders empty gray bars when provider has no SLA data | Shipped in `ccfcb1b0`: chart now gated on `reports30d > 0`. When a provider has set an SLA target but submitted zero SLI reports, the card shows a friendly empty state ('No SLA reports in the last 30 days') instead of 30 misleading gray bars. Target stays visible in the card header. E2E in `offering-sla-empty-state.spec.ts`. |
| 433 | No UI to top up account balance — `/dashboard/transfers` only shows history | **Small-fix path** shipped in `9df37443`: balance card gained explanatory subtitle (P2P transfer units; rentals are per-transaction at checkout). E2E in `transfers.spec.ts`. Larger pre-pay deposit CTA remains out of scope. |
| 410 | Stripe: cleanup stale pending contracts (payment timeout) | Shipped in `8ca5e070`: `Pending → Expired` transition allowed, `find_stale_pending`/`expire_pending` with money-safety guard `AND payment_status != 'succeeded'`, wired into `TimeoutCleanupService` via env `PENDING_TIMEOUT_SECONDS` (default 3600). Partial index `046`. |
| 434 | Flaky test: `account-notifications.spec.ts` in parallel runs (workers>1) | False alarm — fixed in `81615b77` (P3.5 mock audit). |

## In-repo known issues (not on GitHub)

### Triaged as non-bugs (2026-07-22 UX audit re-verification)

| ID | Finding | Status |
|----|---------|--------|
| H3 | Create-offering step 2 Hetzner dead-end | **False positive** — `<a href="/dashboard/cloud/accounts">` link exists at `offerings/create/+page.svelte:570`. Next button skips to step 3. Was stale build in audit. |
| H4 | Rentals 404 on `/contract-events` | **Resolved by server rebuild** — route always existed at `main.rs:1372`. Audit's 404 was from stale binary. |
| M3 | Create-offering placeholders-as-labels | **False positive** — all inputs/selects have proper `<label for="...">` elements. Was stale build in audit. |
| H6 | All offerings unrentable (Provider Offline) | **Operational** — correct behavior (disables Rent for offline provider). Dev seed data has offline provider; bring online via `node scripts/dc-auth.js seed-ux-data` keepalive daemon. |
| L2 | Account ID reads as placeholder (`aaaa00…000001`) | **Test data artifact** — the `uxaudit` test account's ID was hand-set to `aaaa…0001` for debugging. Not a UI bug. |
| L3 | Landing stats all 0 vs populated hero card | **Dev environment** — `GET /api/v1/stats` returns zeros against empty dev DB. Populated in production. |

### Deferred product decisions

| ID | Finding | Status |
|----|---------|--------|
| H2 | Transfers page has no Send/Receive UI | **Feature gap** — P2P send needs IC canister integration (product decision, not a bug). Balance card already explains rentals are per-transaction at checkout. Related: #433 (closed, small-fix), #420 (ICPay deferred). |
| M5 | Billing VAT country EU-only | **Known limitation** — global country list needs server-side VAT rule changes. Low priority pre-launch. |
| L1 | Security page: seed-login device is 'Unnamed Device' | **Minor UX** — consider prompting device name on first login. |

### E2E harness tech debt (in-repo, surfaced 2026-07-23)

| Finding | Status |
|---------|--------|
| Full suite 192s for 205 tests; <60s goal needs multi-stack sharding | **Empirically investigated — sharding does NOT help on this box.** Built full harness (`scripts/e2e-shard.sh`, `dev-server.sh` STACK_INDEX, `fixtures/api-base.ts`). Root cause: 3 shard stacks share ONE Postgres → competing pools = worse DB contention than single-stack's single pool (3×4w=22 fails/4m30s; 3×2w=4 flakes). **Single stack 4 workers = 205/0 green ~192s = proven optimum.** For sharding to truly help, each shard needs its own Postgres instance (future CI-runner work). As a side benefit, dev CORS now correctly allows any localhost origin and the service worker no longer masks API errors. |
| `scripts/browser.js eval --seed <phrase>` throws "UtilityScript.evaluate" | **Minor tooling** — `authenticatePage` (browser.js:332-336) does an extra `goto`+`networkidle`+300ms after seed inject; a SvelteKit client-side redirect/WelcomeModal likely destroys the eval context. `snap`/`shot`/`errs`/`html`/`tour` all work with `--seed`; only `eval` is affected. For authed JS eval, use the e2e framework. |
| `scripts/browser.js --seed` greedily consumes positional args | **Minor tooling** — `--seed <phrase> <url>` fails ("Got 14 words") because the parser consumes all subsequent non-flag args as seed words. Documented usage `snap <url> --seed "$SEED"` (seed last) works. One-line fix possible but it's a test helper, not product. |
| Coverage gap: rent→pay→view→cancel happy path (UI-created contract, not DB-seeded) | **Known gap** — the primary tenant flow is only fragmented (cancel asserted on DB-seeded contracts). Payment-bound (Stripe); higher effort. Parked. |
| Coverage gap: provider agent-pool mgmt `/dashboard/provider/agents/[pool_id]` | **Known gap** — pool create + detail/edit untested. Needs a populated provider fixture. Parked. |

### Deferred product decisions (surfaced 2026-07-23 UX review)

| Finding | Status |
|---------|--------|
| No `?` keyboard-shortcut help overlay | **RESOLVED (2026-07-23; stale entry corrected 2026-07-24)** — `KeyboardHelpOverlay.svelte` exists and is covered by 3 tests in `keyboard-shortcuts.spec.ts` (`? opens help overlay listing all shortcuts` is `@smoke`). This row was stale; corrected. |
| Dashboard shows provider-monitoring stats to brand-new renters | **Design judgment** — fresh renters see "Infrastructure Uptime", "Contracts Monitored", "Red Flags Detected" cards. Non-Providers may find this confusing. Needs product input on conditional rendering. |
