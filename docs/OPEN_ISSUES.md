# Open Issues

**Snapshot:** 2026-07-22. **Canonical source:** GitHub Issues at `decent-stuff/decent-cloud`
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
| 426 | Test: out-of-order Stripe webhook delivery (dispute.created before checkout.session.completed) |
| 425 | Audit existing Provisioning → Cancelled failure paths and migrate to ProvisioningFailed |
| 420 | ICPay: implement automated payouts when ICRC-1 transfer API ships |

## Deferred — UX

| # | Title | Filed by |
|---|-------|----------|
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
| 387 | Concurrent multi-ticket processing via multiprocessing + worktrees |
| 382 | dc-agent: remove `try_trigger_hetzner_provisioning` backward-compat alias |
| 373 | DRY refactor: `extract_contract_id()` shared across 3 provisioners |
| 344 | dc-agent: additional MOCK tests for Docker provisioner (P2) |
| 334 | Code: Add tests for database modules without dedicated test files |
| 214 | dc-agent: `verify_setup()` check for default_image existence (P2) |
| 212 | dc-agent: pre-built Docker image with openssh-server (P2) |
| 107 | Backlog: Dark/light mode toggle |

## Recently closed by this work

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
