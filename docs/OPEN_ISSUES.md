# Open Issues

**Snapshot:** 2026-08-02. **Canonical source:** GitHub Issues at `decent-stuff/decent-cloud`
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
| 427 | Anthropic API key proxy/sidecar for per-identity isolation | decent-agents, launch | **Architecture decided + core shipped** (host-side reverse proxy). New `anthropic-proxy` crate: injects key per-request, meters usage per identity, streams responses, redacts key everywhere. PoC proven against z.ai; 33 tests green. **Acceptance #3/#4 BLOCKED on #413 Rust impl** (container config doesn't exist yet). Not closed. |
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
| 447 | Replay dispute lifecycle (pause/refund) for orphans re-linked after late checkout completion (filed 2026-07-25 from #426 scope decision) |
| 425 | Audit existing Provisioning → Cancelled failure paths and migrate to ProvisioningFailed |

> **#426 (RESOLVED 2026-07-25, `8ab75838`):** investigated real behavior — orphan disputes (delivered
> before `checkout.session.completed`) stayed orphaned permanently. Shipped a minimal money-safe
> reconciliation (`relink_orphan_disputes_for_payment_intent`, idempotent, no money-column writes).
> The scoped-out retroactive pause/refund replay filed as **#447**.

> **#447 (PARTIALLY SHIPPED 2026-07-26, `71732957`):** the missed effect was `pause_contract` (the
> normal `handle_dispute_created` handler pauses the matched contract; an orphan missed that). PoC
> confirmed the gap. Shipped `Database::replay_orphan_dispute_pause` — wired best-effort after the
> relink in `checkout.session.completed`, it replays the missed non-money pause for re-linked OPEN
> disputes (idempotent via `pause_contract`'s same-reason no-op), detects closed-`lost` orphans and
> pages ops (no auto-refund), and fails gracefully when the contract is not yet pausable (the
> realistic `requested` state). Money-safe: pause touches status/audit rows only, NEVER refund
> columns; 3 DB-backed tests. **Deferred (money-path, needs operator sign-off):** (1) auto-replay of
> terminate+refund for orphans that closed `lost` while orphaned — currently detected + ops-paged,
> recommend an operator-triggered replay endpoint; (2) pause-on-activation for contracts that become
> active after the replay ran (pre-existing gap, needs state-machine + dc-agent coordination). #447
> stays **open** until the money-path follow-up lands.

> **#443** (boot-gate asymmetry: no `require_icpay_in_prod`) and **#420** (ICPay automated payouts)
> closed **2026-07-24 — moot**: the ICPay rail was fully retired (Stripe is the sole rail). See
> "Recently closed" below.

## Deferred — UX

| # | Title | Filed by |
|---|-------|----------|
| _none currently open_ | — | — |

> **#442 (RESOLVED 2026-07-25, `c14cb939`):** create-offering price auto-suggest shipped —
> pre-fill `#monthly-price` with `cost × 1.15` (15% markup, the product decision from comment
> `5078165010`) when Hetzner server cost is known; provider-overridable via a `monthlyPriceTouched`
> flag (never clobbers a typed value). Pure `suggestMonthlyPrice(cost)` helper + `DEFAULT_MARKUP`
> const in `offering-wizard.ts` (10 unit tests); 2 e2e (pre-fill + override-reaches-API). Issue
> **closed**. (Previously listed here as deferred+actionable; the resolution was recorded in the
> 2026-07-25 GH issue sweep but this section was not updated — corrected 2026-08-01.)

> **#441 (RESOLVED 2026-07-25, `b1158bff`):** trial/CTA mismatch fixed — copy now honestly reflects
> the CTA via `shouldShowTrialCopy(plan)` = `trialDays>0 && stripePriceId`; contact-sales-only plans
> (Pro/Enterprise) no longer advertise a trial. Test in `account-subscription.spec.ts` (`@smoke`).
>
> **#436 (RESOLVED 2026-07-25, `3fa993a4` + `ea29b0a3`):** seed-phrase sign-in default fixed via the
> recommended capability-endpoint path. New public `GET /api/v1/auth/capabilities` →
> `{google_oauth: bool}`; the frontend defaults to the credential (seed-phrase) form when OAuth is
> off (no extra click). Server env (`GOOGLE_OAUTH_CLIENT_ID`) is the single source of truth. The
> success-screen auto-redirect bonus was **deferred** (filed as **#445**) — **RESOLVED** later the
> same day (`3b501c62`, see "Recently closed" above).

## Deferred — Tech debt / low-value

| # | Title |
|---|-------|
| 444 | Tech debt: split large source files (>2000 lines) into logical modules |
| 387 | Concurrent multi-ticket processing via multiprocessing + worktrees |
| 334 | Code: Add tests for database modules without dedicated test files |

> **#444 progress (updated 2026-08-02):** 6 providers.rs splits shipped (`PoolsApi` `74fb9248`,
> `NotificationsApi` `b4259194`, `SlaApi` `ae97cd8f`, `AllowlistApi` `290a218f`, `OfferingCsvApi`
> `d94d29af`, `ProviderStatsApi` `b5aa9acb`) + the `api-cli.rs` → dir-bin split (`c7dbf962`);
> providers.rs 6739→**4280** (−2459), each verified byte-identical OpenAPI. Decomposition roadmap at
> `docs/plans/2026-07-25-large-file-splits-444.md`. Current largest **source** files (>2000 lines,
> `wc -l` 2026-08-02, excluding `target/`/`third_party/` and `*_tests.rs`/`tests.rs`):
> `api/src/openapi/providers.rs` **4280**, `dc-agent/src/main.rs` **3674**, `api/src/database/offerings.rs`
> **2865**, `api/src/openapi/webhooks.rs` **2504**, `api/src/database/cloud_resources.rs` **2444**,
> `api/src/openapi/contracts.rs` **2251**, `api/src/openapi/accounts.rs` **2230** (7 files).
> (The largest **test** files — `database/contracts/tests.rs` 5952, `database/offerings/tests.rs` 5368,
> `database/stats/tests.rs` 3141 — are out of #444's "source files" scope.) `accounts.rs`'s three clean
> `#[OpenApi]` clusters (TOTP/recovery/email-verification) were split in Waves 9/10/11 using free slots
> in the 2nd inner tuple (now 14/16 used) — further accounts.rs shrinkage needs a design pass (remaining
> handlers share `ApiAuthenticatedUser`-gated core). GH #444 stays **open** (partial; ongoing).

> **#387 status (verified 2026-08-02):** still **open** — no implementation found.
> `rg "multiprocessing|worktree|concurrent\.futures|ProcessPool"` = 0 hits across dc-agent/api/cli.
> The dc-agent ticket loop is single-threaded async (`poll_and_provision` driven by one tokio
> `interval` in `dc-agent/src/main.rs:1456`); tickets are processed serially per poll tick. No git
> worktrees. Parked — would need a deliberate design (per-ticket worktree + process pool) before work.

> **#334 status (verified 2026-08-02):** largely addressed, kept **open**. Audited
> `api/src/database/*.rs`: nearly every logic module now has in-file `#[cfg(test)]` coverage
> (`acme_dns`, `agent_*`, `api_tokens`, `bandwidth`, `chatwoot`, `cloud_accounts`, `cloud_resources`,
> `handlers`, `notification_config`, `offering_sla`, `offerings`, `recovery`, `refund_audit`,
> `reputation`, `reseller`, `rewards`, `spending_alerts`, `stats`, `telegram_tracking`, `tokens`,
> `totp`, `user_notifications`, `users`, `visibility_allowlist`) or a dedicated subdir `tests.rs`
> (`accounts`, `contracts`, `email`, `offerings`, `stats`, `tokens`, `users`). The only logic module
> with neither is `refund_requests.rs` — but its `process_gated_refund` path is covered cross-module
> by the 9 refund-gate integration tests in `api/src/database/contracts/tests.rs`. Meta files
> (`migration_tests.rs`, `test_helpers.rs`, `tests.rs`, `types.rs`) need no tests. Kept open per the
> literal "without dedicated test files" reading.

> **Closed 2026-08-02 (verified against the actual code; moved out of the open table above):**
> - **#382** `try_trigger_hetzner_provisioning` backward-compat alias — `rg` = 0 matches in
>   dc-agent/api/cli. STALE entry, marked closed.
> - **#373** DRY `extract_contract_id()` shared across 3 provisioners — single shared fn at
>   `dc-agent/src/provisioner/mod.rs:12`, imported by `digitalocean.rs`, `docker.rs`,
>   `proxmox_tests.rs`. STALE entry, marked closed.
> - **#344** additional MOCK tests for the Docker provisioner — `dc-agent/src/provisioner/docker_tests.rs`
>   = 995 lines, 87 mockito-based test fns (image pull, create/inspect/start, verify_setup image
>   found/not-found/custom, network/ipv6 warnings, error paths). Substantially done.
> - **#214** `verify_setup()` check for default_image existence — ships in `docker.rs:638` (compares
>   `config.default_image` against `/images/json` tags), `digitalocean.rs:758` (queries
>   `/v2/images?slug=`), and `proxmox.rs:1138` (template-VM existence, the Proxmox equivalent).
>   3 dedicated docker tests (`test_verify_setup_image_found` / `_not_found` /
>   `_not_found_custom_image`).
> - **#212** pre-built Docker image with openssh-server — `dc-agent/container/` ships `Dockerfile`
>   (ubuntu:22.04 + openssh-server + `PermitRootLogin yes` + sshd ENTRYPOINT), `build.sh`,
>   `publish.sh`; the default image is `ghcr.io/decent-stuff/dc-agent-ssh:latest`
>   (`config.rs::default_docker_image`); tests assert the container CMD no longer runs apt-get.
>   (Note: `container/README.md` header still reads "Ticket 348" — a stale number; the implementation
>   matches #212. README edit out of scope for this docs pass.)
> - **#107** Dark/light mode toggle — `website/src/lib/stores/theme.ts` (dark/light store: system
>   preference, `localStorage` persistence, toggle/set, `matchMedia` live-sync) + `ThemeToggle.svelte`
>   rendered in `routes/dashboard/+layout.svelte` and `DashboardSidebar.svelte` + `theme.test.ts` +
>   extensive `:root[data-theme='light']` rules in `app.css`. Fully shipped.

## Recently closed by this work

### 2026-08-02 session (WAVE-0: prior-session WIP + stale-issue reconciliation)

Reconciled the `docs/OPEN_ISSUES.md` "Deferred — Tech debt / low-value" table against the actual
code (a code-verification pass — no behavior changes), and recorded the 3 prior-session WIP commits
that had landed but were not yet logged.

**Prior-session WIP shipped (3 commits):**

| Commit | Area | Detail |
|--------|------|--------|
| `f186c0d9` | api / chatwoot | **fix(api): chatwoot create_portal must not claim a shared custom_domain.** `create_portal` was sending the shared frontend host as `custom_domain` for every provider — Chatwoot's `custom_domain` is globally unique, so only the FIRST provider could onboard a Help Center; every later one 422'd "Custom domain has already been taken". Fix: send `custom_domain=""` (empty string dodges `URI.parse(nil)` TypeError; a `before_validation` hook normalizes `""→nil` so it passes `allow_nil` uniqueness). TDD regression test added. |
| `e5a1f08e` | docs | **AGENTS.md canonical-source note** — records GitHub Issues as the canonical live source and the in-repo inventory as a categorized snapshot, with the reconcile-before-acting rule. |
| `897a90e5` | ops / secrets | **chore(secrets): re-encrypt common.yaml** — sops 3.9.4 → 3.11.0 (tooling bump; no secret-value changes). |

**Stale-issue reconciliation (code-verified, docs-only):** audited every row of the
"Deferred — Tech debt / low-value" table with `rg` / `find` / `wc -l` against the working tree.
Confirmed **6 stale entries already done in code** and moved them out of the open table: **#382** and
**#373** (backward-compat alias + DRY refactor — 0 matches / single shared fn), **#344** (Docker MOCK
tests — 995-line `docker_tests.rs`, 87 fns), **#214** (`verify_setup` default_image check — ships in
all 3 provisioners + 3 tests), **#212** (pre-built openssh image — `dc-agent/container/` + default
image), **#107** (dark/light toggle — `theme.ts` + `ThemeToggle.svelte` + tests). Kept open with
current evidence: **#444** (partial; progress note refreshed with the real largest-file counts),
**#387** (no implementation found; single-threaded poll loop), **#334** (largely addressed inline;
kept open on the literal "dedicated test files" reading). See the table notes above for per-issue
evidence.

Gates: docs-only — no code touched. `rg -n "try_trigger_hetzner_provisioning|#373|#382" docs/OPEN_ISSUES.md`
records #382/#373 as closed.

### 2026-08-02 session (e2e harness radicalization + UX slop fix + #444 Wave 9/10 + auth single-source)

Continuation of the radical-overhaul mandate (harness + UX + tech debt + robustness) against a
verified-green baseline. 17 commits (`9657dee8`→`749cf876`), TDD-first where applicable, verified
against the real warm stack (api:59011 + web:59010), no first-party mocks. Final gates: smoke
**26/26 in ~28s** (<30s target), clippy **0**, vitest **862**, svelte-check **0/0**.

| Fix | Area | Resolution |
|-----|------|------------|
| Smoke speed: 39.6s → ~28s | E2E harness | `9657dee8`: the `testAccount` authed page fixture did a wasteful double-navigation (logged in, then re-navigated to the same page). Dropped the redundant navigation → smoke **39.6s → ~28s**, zero coverage loss; all 26 smokes green + reliable. |
| Coverage gap: `/dashboard/reputation/[identifier]/trust` | E2E coverage | `9e437e45`: new `reputation-trust.spec.ts` — the reputation trust-report route was an undocumented coverage gap; now driven against the warm stack. |
| No-mock invariant documented | E2E discipline | `41ee69b8`: FLOWS.md now records the 2 first-party fetch mocks as **sanctioned exceptions** (a Mock inventory added) — both are outbound-HTTP-boundary stubs, not first-party-logic mocks. The no-mock invariant holds. |
| Stale smoke-table titles + count drift | Docs | `445a17d4` + `3178799d`: fixed stale smoke-table titles in FLOWS.md; corrected smoke-count drift (27→26 after the SaaS-removal session dropped the subscription spec). |
| Stale-issue reconciliation | Docs | `e775492d` (+ the WAVE-0 pass): #382, #373, #344, #214, #212, #107 all verified **CLOSED** against code evidence; the open tech-debt table went **8→3 rows** (#444, #387, #334 remain). Per-issue evidence is in the WAVE-0 entry above. |
| #444 Wave 9 — `TotpApi` split | Tech debt | `1729e7c6` + `8c6dd37c`: extracted `TotpApi` from `api/src/openapi/accounts.rs` (**2903→2594 lines**). Byte-identical OpenAPI verified via spare-port instance diff (**187 paths / 327 schemas**, empty canonical diff). clippy 0, nextest **44/44**. |
| #444 Wave 10 — `RecoveryApi` split | Tech debt | `f041a121` + `d9e51a58`: extracted `RecoveryApi` from `accounts.rs` (**2594→2442 lines**). Byte-identical OpenAPI; clippy 0, nextest **39/39**. Next candidate: the email-verification cluster (8/10 readiness). |
| #444 Wave 11 — `EmailVerificationApi` split | Tech debt | `24ccacb7` + `5e4c38b9`: extracted `EmailVerificationApi` from `accounts.rs` (**2442→2230 lines**). Byte-identical OpenAPI (187 paths / 327 schemas, empty canonical diff); clippy 0, nextest **37/37**. **accounts.rs is now exhausted for mechanical splits** — the three clean clusters (TOTP/recovery/email-verification, all `#[OpenApi]` handler groups) are done; remaining handlers are interwoven with the `ApiAuthenticatedUser`-gated core and need a focused design pass, not the wave cadence. |
| UX U1: hero trust card was fake data | UX (slop) | `50fb8a15`: (no-mock UX audit) the landing hero "trust card" was a **hardcoded fake** (`provider_alpha` with a deceptive "Updated 2m ago" liveness stamp). Now honestly labeled **"Illustrative example"**; the fake liveness text removed. No misleading data on the landing page. |
| UX U2: all-zero "Marketplace Statistics" | UX (empty state) | `d719df71`: "Marketplace Statistics" rendered all-zeros unconditionally — dishonest on a fresh marketplace. New pure `marketplaceIsEmpty(stats)` helper (4 unit tests, TDD RED→GREEN) gates an honest **"Be Among the First Providers"** early-access reframe instead of showing 0/0/0. vitest 862; zai-vision-verified on the real app. |
| Auth single-source-of-truth fully enforced | Robustness / DRY | `d34e11fb` + `749cf876`: (code-robustness audit R1/R2) dc-agent `api_client.rs` hand-rolled the signed-message layout + header-name literals → now delegates to the canonical `dcc_common::api_auth::{sign_request, HEADER_*}`; api-cli header literals → `HEADER_*` consts. Wire format proven **byte-identical** field-by-field (timestamp unit, nonce, header names, message byte-layout); the unchanged dc-agent auth-guard test stayed green. No outlier remains — the "Signed-request auth — single source" convention is now fully enforced. |

**Net-new findings (documented / tracked):**
- **Code-robustness audit:** most categories CLEAN (timeouts, hex, stale refs, dead code, DB
  defaults, money-path/refund-gate, unwrap/expect, `api.ts`, `danger_accept_invalid_certs`). One
  finding **shipped** (R1/R2 auth single-source, above). One finding **shipped (R3):**
  `StripeClient::new().ok()` silently swallowed Stripe misconfig at 6 sites
  (`admin.rs`/`providers.rs`/`webhooks.rs`/`contracts.rs`/`main.rs`×2). It was money-safe (returned
  `None` → handlers return `Ok(None)` = "refund not performed"), but **invisible** — no warning was
  logged. **Fixed in `b7016c40`** via a DRY `stripe_client_or_warn()` helper (next to `StripeClient`)
  that emits an actionable `tracing::warn!` (names `STRIPE_SECRET_KEY`, lists what is skipped,
  includes the error chain) before returning the same `None` — zero money-behavior change, all
  refund-path tests green. `rg "StripeClient::new().ok()"` now 0.
- **UX audit Low findings (NOT shipped — below/over threshold):** **U3** validators-zeros (an
  environment artifact, not a bug); **U4** provider-gate button hierarchy (confidence 5, below the
  6/10 ship threshold — skipped); **U5** "Welcome back" greeting for first-time users (confidence 6,
  needs a first-visit detection state — parked).

### 2026-08-02 session (drop unused SaaS account-subscription feature)

Removed the unused SaaS account-subscription feature (Free/Pro/Enterprise pricing plans for using
Decent Cloud) FULLY across frontend + backend + DB. This was Feature A; it was confirmed unused —
`account_has_feature` + `count_active_contracts_for_account` were both `#[allow(dead_code)]`, so
runtime feature-gating was never enforced (free plan = unlimited rentals). The DISTINCT per-contract
recurring billing (Feature B: `contract_sign_requests.stripe_subscription_id` /
`.subscription_status` / `.current_period_end_ns` / `.cancel_at_period_end`,
`provider_offerings.is_subscription` / `.subscription_interval_days`, the
`get_subscription_item_id` + `create_usage_record` metered-billing code path in
`cleanup_service.rs`, and the `invoice.paid` / `charge.dispute.*` webhook arms) is PRESERVED
untouched.

| Change | Area | Detail |
|--------|------|--------|
| Backend removal | api crate | Deleted `openapi/subscriptions.rs` (SubscriptionsApi, 5 endpoints) + `database/subscriptions.rs` (SubscriptionPlan/AccountSubscription/SubscriptionEvent + all fns/tests, 1106 LOC total). Unwired from router tuple, ApiTags enum, rate-limiter checkout path + test, and `database/mod.rs` re-exports. Removed `customer.subscription.{created,updated,deleted}` webhook arms + their now-orphaned structs (`StripeSubscription`/`Items`/`Item`/`Price`) + the 3 event registrations in `main.rs`. Trimmed `invoice.payment_failed` to parse + `tracing::warn!` only (dropped the SaaS-specific `subscription_id` inner block). Removed subscription-only `stripe_client.rs` methods (`create_subscription_checkout`, `get_subscription`, `cancel_subscription`, `create_portal_session`, `get_or_create_customer` + `SubscriptionInfo`). KEPT Feature-B `get_subscription_item_id`/`create_usage_record` (used by `cleanup_service.rs`). |
| DB schema | migration 052 | `api/migrations_pg/052_drop_account_subscription_feature.sql`: drops `subscription_events`, `subscription_plans`, 3 accounts indexes, 6 accounts columns (`subscription_*`, `stripe_customer_id`). `contract_sign_requests.*` columns NOT dropped (Feature B). |
| Frontend removal | website | Deleted `routes/dashboard/account/subscription/` (+page.svelte 326L + contact-sales.test.ts), `lib/utils/subscription-plans.{ts,test.ts}`, `tests/e2e/account-subscription.spec.ts`. Removed Subscription tab from `SettingsTabs` (+ test), the subscription card from `account/+page.svelte`, the Subscription API section from `api.ts` (2 interfaces + 5 fns, ~177 LOC). KEPT Contract-type subscription fields (`api.ts` L1361-1365 — Feature B). Updated `route-audit.spec.ts` + `seed-helpers.ts`. |
| Web e2e docs | FLOWS.md | Removed subscription coverage rows + `@account` tag entry; smoke count 27→26; renumbered smoke table. |

Gates: `cargo build -p api --bin api-server` clean; `cargo clippy -p api --tests --all-targets` 0 warnings; `cargo nextest run -p api` green on all touched modules (subscriptions → 0 tests, accounts 122/122, webhooks/rate_limit/stripe_client/cleanup_service 67/67); `npm run check` 0/0; `npx vitest run` 858/64 files; `npm run test:e2e:fast:smoke` 26/26 in 36.7s.

### 2026-08-01 session (clippy cleanup + e2e gap verification + CLI harness + UX root-cause)

Continuation of the radical-harness/UX/tech-debt mandate against the verified real baseline. All
work TDD-first where applicable, verified against the real warm stack (api:59011 + web:59010), no
mocks in first-party paths. **No commits made** — changes are staged in the working tree pending a
user review/commit decision. Final gates: `cargo clippy --workspace --tests --all-targets` → **0
warnings**; cargo lib tests **1469/0**; `npm run test:e2e:fast:smoke` 27/27; rent-flow **4/4**;
`cargo nextest run -p decent-cloud` **63/6**.

| Fix | Area | Resolution |
|-----|------|------------|
| Clippy: 30 → 0 warnings (DRIFT fix) | Tech debt | 10 edits across api + dc-agent. dc-agent `digitalocean.rs`: file-top `#![allow(dead_code)]` (DO API response structs deserialize full shape for fidelity + `digitalocean_tests.rs` assertions — fields read in tests, cannot drop); removed truly-dead `DoErrorResponse` (0 refs incl. tests); `proxmox.rs:729` `while_let_loop` rewrite (`while let Ok((stream,_)) = listener.accept()`). api: `dispute.rs:694` `#[allow(dead_code)]` on test-only helper; removed 2 unused `now_ns` blocks in `tests.rs`; `#[allow(clippy::too_many_arguments)]` on 3 column-binding fns; `#[allow(clippy::type_complexity)]` on money-path `query_as`; `#[allow(dead_code)]` on `RefundGateOutcome::PendingApproval.user_latest_payment_e9s` (money-path audit data, already logged at the gate site). Changed-crate tests: dc-agent 246/246, api stripe_client 18/18, api refund_gate 8/8. |
| `#442` doc drift reconciliation | Docs | OPEN_ISSUES.md listed `#442` BOTH as "Deferred — UX" AND as RESOLVED (`c14cb939`). Reconciled: the Deferred table now reads `_none currently open_`; the deferred note now records the resolution (corrected 2026-08-01); historical session tails struck-through with CLOSED annotation. |
| rent→pay→view→cancel e2e gap — confirmed CLOSED | Coverage | `rent-flow.spec.ts` (4 serial tests, 238L) already drives the real marketplace Rent dialog → signed POST /contracts → rentals list → detail page → signed PUT cancel against the warm stack. Re-ran: **4/4 in 24.8s**. Contract commits at `requested` (cancellable) before Stripe checkout, so drivable without STRIPE_SECRET_KEY. FLOWS.md + OPEN_ISSUES tech-debt rows updated to CLOSED. |
| CLI harness coverage audit + 4 tests + error fix | Coverage / robustness | `cli/tests/cli_flows.rs` +141 lines: pool commands identity guard, register/check-in ghost-identity offline short-circuit, malformed `--amount-dct`/`--amount-e9s` parse rejection, pool-generate missing-pricing-file error. `cli/src/commands/account.rs` amount-parse errors upgraded from bare `ParseFloatError`/`ParseIntError` to detailed `Invalid --amount-dct '{value}': {e}. Pass a decimal number of DC tokens (e.g., --amount-dct 1.5).` (was violating the "provide failure details" rule). `cargo nextest run -p decent-cloud` → **63/6** (was 59/6). |
| JS error "environment variable not found" — root-caused + fixed | UX / debuggable errors | **Root cause:** `api/src/stripe_client.rs:35` `std::env::var("STRIPE_SECRET_KEY")?` propagated `VarError::NotPresent` whose `.to_string()` is the stdlib string "environment variable not found" — bubbling through `create_stripe_checkout_session` → contracts.rs handler → frontend `createRentalRequest` → `RentalRequestDialog.svelte:268` catch. The contract IS created at `requested` before Stripe is called, so the bare error misled (rental succeeded, payment-init failed). Fix: `stripe_client.rs` `.context("STRIPE_SECRET_KEY is not set — Stripe payment processing is unavailable")`; contracts.rs handler now returns `"Rental created but payment could not be initiated: {e}. You can retry payment or cancel from your rentals page."` + `tracing::warn!` server log. 18/18 stripe tests, rent-flow 4/4, live-repro verified against release-mode api-server. |
| Live UX audit (no mocks) | UX | Drove the real app via Playwright Chromium + chrome-cli against the warm stack. Homepage + marketplace: **0 console errors**. Warm-stack API config confirmed correct (`dev-server.sh:280` injects `VITE_DECENT_CLOUD_API_URL` as process env, highest priority over `.env.local`). The single console error surfaced (above) was root-caused to a backend error-message gap, not a frontend bug. |

**Net-new finding (documented, NOT autonomously resolved — below the threshold per AGENTS.md
"conflicting business-logic implementations"):**
- **`cli/src/keygen.rs` standalone binary duplicates `cli/src/commands/keygen.rs` with DIVERGED
  behavior.** The standalone `[[bin]] name="keygen"` is unreferenced by any script/CI/docs/dockerfile
  and looks like a leftover dev/demo tool; it has its own `ALL_LANGUAGES`, `detect_mnemonic`,
  `mnemonic_from_strings` (validates word count 12/15/18/21/24 — the `dc keygen` command does not,
  relying on `bip39::Mnemonic::from_phrase` to reject bad counts). It carries genuine
  sign/verify/mnemonic/seed unit tests. **Recommendation:** delete the standalone binary OR make it
  delegate to the shared `commands/keygen.rs` functions. Decision parked (binary surface change + the
  divergence is not a live bug). Filed here as a tracked finding.

### 2026-07-26 session (refund approval gate + e2e harness expansion + UX audit)

Refund approval gate (user-requested cost-safe billing policy) fully shipped,
e2e harness expanded for both CLI and web, OpenAPI tuple rebalanced to unblock
future #444 splits, and a full live UX audit found no product issues. All work
TDD-first, verified against the real warm stack.

| Fix | Area | Resolution |
|-----|------|------------|
| Refund approval gate — full feature | Backend + admin UI + e2e | **Plan**: `docs/plans/2026-07-26-refund-approval-gate.md` (`6c22263a`). Policy: auto-refund when `refund_e9s ≤ user's latest Stripe payment`; hold for admin approval otherwise; Telegram on every event; unbypassable DB trigger. **Migration 051** (`335386f2`): `refund_requests` table + `enforce_refund_approval_gate` trigger (blocks `payment_status='refunded'` / `stripe_refund_id` first-set without matching `refund_requests` row with `status IN ('auto_issued','approved')`). **DB layer**: `process_gated_refund` replaces direct `issue_audited_refund` calls in ALL 4 refund paths (cancel/reject/dispute_lost/provisioning_failed). **Admin API** (`f7b75b9f`): `GET/POST /admin/refund-requests` (list/approve/decline). **Admin UI** (`b4f1ba3d`): refund-requests section in `/dashboard/admin` with status filter, cap-exceeded badge, inline review panel. **DB gate tests** (`8ec052ad`): 9 integration tests (auto-issue, cap-exceeded hold, admin approve/decline, trigger blocks bypass × 3). **E2E** (`217eee8c`): 3 admin panel tests (API listing, UI decline end-to-end, status filter). |
| CLI e2e harness expansion (#444) | Coverage | `1331273f`/`8699f7cc`/`413d2a28`: 18 new tests (13 offline flows + 3 smoke + 4 IC-mainnet `#[ignore]` + 1 hardened). Default tier 41→59 @0.58s; IC tier 2→6. Found + fixed production bug: `account --transfer-to <bad>` panicked via `IcrcCompatibleAccount::from().expect()` → validates principal at call site. |
| Web e2e: confirmInlineAction helper (#444 audit #11) | DRY | `4f5e4906`: extracted `confirmInlineAction(page, row, {arm, confirm?, secondary?, waitForResponse?})` in `auth-helpers.ts`; applied to 7 inline-delete entities + rentals cancel. ~50 LOC boilerplate collapsed. Audit items #4/#5/#7 verified already shipped. |
| Baseline: auth-capabilities stale OAuth | Test | `d33ff5bc`: 2 `@smoke` tests hardcoded `google_oauth=false` but warm stack now has OAuth on. Rewrote spec env-agnostic: reads real `/api/v1/auth/capabilities`. Smoke 27/27 green. |
| #444: OpenAPI tuple rebalance | Tech debt | `87a48059`: rebalanced `create_combined_api()` from `(9-tuple, 16-tuple)` → `(13-tuple, 12-tuple)` by moving `PoolsApi`/`NotificationsApi`/`SlaApi`/`AllowlistApi` to tuple 1. Verified via clean-room spec diff (empty — 192 paths, 337 schemas both sides). Unblocks future handler splits (accounts.rs recovery/TOTP, offerings.rs recommendations) — tuple 2 now has 4 free slots. |
| Decent-Agents cluster re-verify | Status | **#413 (per-subscription agent identity) CLOSED** — was the key blocker. `anthropic-proxy` crate fully functional (1680 lines, 33/33 tests pass): key injection/stripping, per-identity metering, redaction, loud failure on errors. Remaining issues (#418 beta onboarding, #415/#416 billing/metering, #429-#432 deferred) are product/business decisions, not code-blocked. |
| Live UX audit (no mocks) | UX | `9995fafd`: audited 8 pages (landing, marketplace, login, dashboard, my-rentals, account settings, admin panel, mobile marketplace) against the real warm stack via `browser.js` + zai-vision. **0 product UX issues found.** Fixed `browser.js` `authenticatePage()` — was not setting `first_login_onboarding_completed` in localStorage, so WelcomeModal blocked authed-page screenshots. |
| FLOWS.md gap assessment | Coverage | Wave 2 review: only 2 ⚠️ partial rows + 1 ❌ sub-item remain, ALL blocked on external deps (rent flow excluded from smoke by design; password-resets empty-state needs backing table ID; send-test-email needs MAILCHANNELS_API_KEY). No actionable code gaps. |

**Refund gate — remaining edge:**
- Admin **approve** path calls Stripe via `issue_audited_refund`. E2E tests cover it with `stripe_client=None` (DB integration tests) and via the admin UI (DB-seeded refund_requests, decline fully tested). A full approve→Stripe-refund e2e requires either a Stripe test-mode payment intent or `STRIPE_SECRET_KEY` unset on the test stack.
- `dispute_refund_idempotency_key` dead-code warning in non-test builds (used only by webhooks tests) — cosmetic.

### 2026-07-25 session (#427 core — Anthropic API key reverse proxy)

Shipped the **core** of #427 as a new standalone workspace crate `anthropic-proxy` (decision:
host-side reverse proxy). The customer container's `ANTHROPIC_BASE_URL` points at a host-side
`anthropic-proxy` process that: strips any client-supplied `x-api-key`/`Authorization`/
`anthropic-version`, injects the platform key upstream per-request (the key **never enters the
container**), forwards the request path-transparently to the Anthropic-compatible upstream, streams
the response back, and meters token usage per identity (non-streaming JSON + streaming SSE terminal
`message_delta`). PoC proven end-to-end against the real z.ai Anthropic-compatible endpoint (both
non-streaming + streaming); key redaction verified absent from all logs/errors.

- Acceptance **#1** (architecture decision): done (host-side reverse proxy).
- Acceptance **#2** (proxy injects key + meters per identity): **shipped** — crate + binary, 33 tests
  green (nextest 0.13s), clippy clean, workspace build intact. MeteringRecorder trait leaves the
  DB-backed recorder (writes `agent_runs.claude_{input,output}_tokens`) to #415/#416.
- Acceptance **#3** (remove shared-key mount from container config) + **#4** (migrate beta
  customers): **BLOCKED on #413** — its Rust container-provisioning does not exist yet
  (`rg anthropic_api_key` = 0 Rust hits; #413 is spec-only). Do NOT attempt until #413 lands.

Issue **#427 stays open** (blocked on #413 for #3/#4). dc-agent integration (spawn the proxy as a
host-side process per identity, point the container's `ANTHROPIC_BASE_URL` at it) is also #413 scope.

### 2026-07-25 session (GH issue sweep — #442 / #426 / #444)

Sweep of all open GH issues with parallel subagents. The 3 credential-free items shipped;
**the entire Decent-Agents cluster is blocked** (see blocker note at the foot of this section).

| Fix | Area | Resolution |
|-----|------|------------|
| #442 create-offering price auto-suggest | UX (decided) | `c14cb939`: pre-fill `#monthly-price` with `cost × 1.15` (15% markup) when Hetzner cost known; provider-overridable via a `monthlyPriceTouched` flag (never clobbers a typed value); hint copy "suggested at 15% markup, adjust as needed". Pure `suggestMonthlyPrice(cost)` helper + `DEFAULT_MARKUP` const in `offering-wizard.ts` (10 unit tests); 2 e2e (pre-fill + override-reaches-API). Issue **closed**. |
| #426 out-of-order Stripe webhook (dispute before checkout) | Backend + test | `8ab75838`: **investigated real behavior first** — `checkout.session.completed` sets the PI but never touched `contract_disputes`; an orphan dispute (all lookups fail) stayed orphaned **permanently**. Shipped outcome (a): minimal money-safe `relink_orphan_disputes_for_payment_intent` (one idempotent UPDATE backfilling `contract_id`, `WHERE contract_id IS NULL` ⇒ replay-safe, touches NO money/status column ⇒ cannot double-refund). Wired best-effort into checkout completion. New DB test `test_orphan_dispute_relinks_on_late_checkout_completion` (proven to FAIL on a no-op then PASS). Issue **closed**. Scoped-out retroactive pause/refund replay filed as **#447** (money-path, separate concern). |
| #444 large-file splits (Waves 5-7) | Tech debt | **6 providers.rs splits** (`74fb9248` PoolsApi, `b4259194` NotificationsApi, `ae97cd8f` SlaApi, `290a218f` AllowlistApi, `d94d29af` OfferingCsvApi, `b5aa9acb` ProviderStatsApi): providers.rs 6739→**4280** (−2459); shared helpers (`validate_cloud_offering`, `build_response_metrics`) kept `pub(crate)`. Each verified byte-identical OpenAPI (189 paths / 332 schemas) via spare-port instance diff. **Wave 7** `c7dbf962` split `api-cli.rs` (3753L) → dir-bin (`main.rs` 547L + 13 subcommand modules), zero OpenAPI impact, `--help` byte-identical, 16 tests green. Tuple arity hit the **poem-openapi 16-max** on the 2nd inner tuple → further *handler* splits need a tuple restructure first (Path A); separable providers.rs clusters now exhausted (4280L = interwoven core). #444 stays **open** — next: tuple restructure → accounts.rs (2903L) clusters, then `database/offerings.rs` (2865L) recommendations block. |

**Decent-Agents cluster — BLOCKED on credentials + unbuilt infrastructure (per AGENTS.md mandatory
workflow, STOP + report; not mocked/stubbed).** `scripts/dc-secrets list shared/env` shows only
`TELEGRAM_ADMIN_CHAT_ID` + `PIPELINE_BOT_TOKEN`. Missing: Anthropic API key (#427/#429), Stripe
secret key (#418/#415), Google OAuth client ID + GitHub App credentials (#418). The product infra
also does not exist yet — grep found only the Stripe webhook receiver; there is **no GitHub App
webhook receiver** (so #431's "extend the verifier for two secrets" has nothing to extend), and
no agent-dispatch / metering / proxy subsystem. These are greenfield epics needing creds +
architectural decisions before one-pass production work can begin:

| # | Title | Blocker |
|---|-------|---------|
| 418 | beta onboarding (invite + first-run demo) | Needs Stripe + Google OAuth + GitHub App creds + email/magic-link; the whole onboarding flow is greenfield. |
| 427 | Anthropic API key proxy/sidecar | **Core shipped** (host-side reverse proxy; `anthropic-proxy` crate). Remaining acceptance #3/#4 (remove shared-key mount + migrate beta) blocked on #413's Rust container-provisioning (spec-only today). |
| 415 | subscription billing + active-hour/token caps | Depends on #427 (dispatch enforcement) + Stripe creds. Meter-table scaffold alone can't be PoC'd end-to-end. |
| 416 | usage metering + customer dashboard | Depends on #415 meters. |
| 429 | Anthropic key exfiltration mitigation | Depends on #427 + the agent container infra. |
| 431 | GitHub App webhook secret rotation | Blocked: no GitHub App webhook verifier exists yet (depends on #418). |
| 430 | CODEOWNERS / branch-protection deadlock UX | Depends on #418 onboarding flow. |
| 432 | per-identity observability + incident runbook | Depends on the agent infra (#413) + Anthropic creds. |

### 2026-07-25 session (robustness tail + CLI e2e harness + #445/#446 closure)

Continuation sweep (baseline `6f6548c8` → `dba28955`/`d11c718d`, 17 commits). Closed the two open
in-scope GH issues (#445, #446); finished the robustness tail + hex-decode migration flagged by the
2026-07-25 audits; built a flow-level e2e harness for the `dc` CLI (and fixed two real bugs it
uncovered); closed the e2e harness tail (C1-C3). Live UX audit: **no net-new issues** (drove the
real site across public + authed surfaces).

| Fix | Area | Resolution |
|-----|------|------------|
| #446 recovery-flow e2e "Continue never reaches Processing" | Test (BUG label) | `9b168add`: root cause was STALE TEST ASSERTIONS, not a frontend dead-end — tests expected `.bg-red-500/20` + "Invalid token" but the recover page surfaces API errors in `.bg-danger/10` with "Invalid recovery token hex: …". Added `waitForResponse` + asserted the real error div; renamed the misleading test. Issue closed. |
| #445 verify-email/recovery success → auto-redirect | UX (enhancement) | `3b501c62`: new shared `AutoRedirect.svelte` (4s countdown + `goto` + cleared interval); wired into verify-email + recover success states; 3 manual options always available (countdown copy + inline "Go now" + retained button). New tests drive the real app; also closes the recovery-success-path coverage gap. Issue closed. |
| Robustness tail A1/A3/A4/A5/A6 | Robustness | `902cb032` proxmox verify_api_token 30s timeout (testable `build_verify_client`); `aed335bf` dedup `REQUEST_TIMEOUT_SECS` via `pub(crate) HTTP_TIMEOUT_SECS`; `8b706c45` `run_command_with_timeout` for 4 `upgrade.rs` commands (10s/30s); `df6002f9` gateway ssh 20s `tokio::time::timeout`; `dc0e9432` doctor `ss` failure → `[WARN]`. dc-agent 245/245. |
| hex::decode migration tail (A2) | DRY | 5 USER-INPUT sites migrated to `decode_hex_path`/`decode_pubkey` (`41f3c6fa`, `98432fc4`, `4a8dbd7a`, `c247891d` + `accounts.rs`); detailed errors replace terse `Err(_)`. The remaining 10 are deliberate DB/Stripe-sourced non-fits (documented in `docs/audits/2026-07-25-code-robustness.md`). |
| CLI provider pool commands 100% broken (auth drift) | Bug (critical) | `b48bdb9b` + `7fe544cd`: `cli/commands/provider.rs` had drifted 4 ways from the canonical signer (wrong header names `X-DC-*` vs `X-Public-Key`; millis vs nanos; no nonce; newline-joined vs byte-concat message). Extracted `common/src/api_auth.rs` as the SINGLE signing source (`sign_request` + `build_signed_message`); `api/auth.rs` verify + `api_cli/client.rs` + cli all delegate to it. Also fixed `tiers` omission in the pool-generate schema. |
| CLI flow-level e2e harness | Coverage | `dba28955`: new `cli/tests/cli_flows.rs` (12 tests, 0.316s offline; warm-stack + IC-mainnet tiers). Covers keygen determinism/reimport-recovery/multilang/stdin, ledger-local, all-local listings, subcommand help, invalid-mnemonic rejection; warm-stack tests prove the auth fix against the real API (assert NO 401 + contains "Pool not found"). |
| E2E harness tail C1-C3 | Coverage / UX | `02503591` C1 offering-edit beforeAll sharing (4 tests share seed); `2d82a6d5` C2 agent-pool rename PUT + detail render (new `agent-pool-edit.spec.ts`); `d11c718d` C3 become-provider `?step=N` deep-link (pure `wizard-logic.ts` + 19 unit tests, TDD). |

**Still open / deferred (unchanged):**
- ~~**#442** create-offering price auto-suggest — needs a product decision (margin/heuristics).~~ → **CLOSED later in the 2026-07-25 GH issue sweep** (`c14cb939`; see above). Kept struck-through as a historical record of this session's tail state.
- **#444** remaining large-file splits — roadmap filed (`docs/plans/2026-07-25-large-file-splits-444.md`).
- **10 deliberate hex non-fit sites** — documented in `docs/audits/2026-07-25-code-robustness.md`.

### 2026-07-25 session (fresh sweep: robustness + UX + coverage + #444 split)

Six-wave sweep (baseline `56df84e6` → `64e46ef4`). GH **#441** and **#436** closed as completed;
net-new UX **#5** (offering-edit ownership) filed and fixed; a robustness/DRY pass; one safe #444
split; and e2e coverage + harness improvements. Final: svelte-check 0/0, vitest 847, clippy clean
(3 known baseline warnings, 0 new), cargo `--lib` 1011/0, smoke 27 @ ~33s, full e2e 300/3 (1 known
parallel flake + 2 **pre-existing** recovery-flow failures unrelated to this session — filed).

| Fix | Area | Resolution |
|-----|------|------------|
| #441 subscription trial/CTA mismatch | UX | `b1158bff`: honest copy via `shouldShowTrialCopy(plan)` = `trialDays>0 && stripePriceId`; contact-sales-only plans no longer advertise a trial. `@smoke` test. |
| #436 seed-phrase sign-in hidden behind extra click | UX | `3fa993a4` + `ea29b0a3`: new public `GET /api/v1/auth/capabilities` (`{google_oauth: bool}`); frontend defaults to credential form when OAuth off. Server env = single source of truth. (Success-screen auto-redirect bonus deferred — filed as **#445**.) |
| #5 (net-new) offering-edit ownership | UX/security | `43ffae8e` + `958ebff1`: `/dashboard/offerings/[id]/edit` redirects non-owners to the view-only route; narrowed identity used in the guard. |
| ICPay-cleanup cluster | Backend + seed + UX | Reject non-Stripe currency at offering create/update `79c83657`; migrate ICP offerings/contracts → USD `058a36e6`; remove stale ICP labels + dead ICP price feed `83605227`; remove dead ICP price feed backend `05c27f01`. |
| http timeouts (money/identity/provisioning) | Robustness | `execute_command` setup helper 300s `40d217f8`; cli provider commands `70b6c4ac`; dc-agent manual provisioner webhook `5da340a4`. |
| STRIPE_API_BASE DRY | DRY | `85afbd8c`: `pub const STRIPE_API_BASE` in `stripe_client.rs`, 5 hardcoded URLs removed; contracts test fixtures finished `40d22a0c`. |
| Silent errors logged, dead code removed | Robustness | Log-don't-swallow in dc-agent doctor/proxmox/chatwoot init `f55750d5`; dead `build_auth_headers` + `post_provision` shim removed `11fc0d2c`. |
| Hex decoding DRY + detailed errors | DRY | `d1cce292`: 18 user-input sites → `decode_pubkey`/`decode_hex_path` helpers in `openapi/common.rs`; terse "Invalid format" → detailed (names field + problem). 22 deliberate DB-sourced non-fit sites documented in `docs/audits/2026-07-25-code-robustness.md`. |
| #444 first safe split | Tech debt | `74fb9248`: `PoolsApi` extracted from `providers.rs` (−957 lines, zero behavior change). Roadmap `c4c68e09`. #444 stays open (ongoing). |
| E2E coverage + harness | Coverage | verify-email success path `c8815db4`; cloud-accounts populated + disconnect `54fa508a`; Stripe `checkout.session.completed` money path `0604f360`; self-contained search-dsl `e5911dd4`; helpers promoted `92058c24`; 7 delete specs parametrized `67f84f7f`; route-audit settle-on-fetch `e0726927`; 5 fast smokes `f4893141`. |
| Smoke fast-loop tuning | Test | `64e46ef4`: demoted 5 slow non-critical specs from `@smoke` → 27 tests @ ~33s (was 32 @ ~51s); kept the authed dashboard, anonymous landing/error, verify-email, sign-in, and #441 money-path. |

**Still open / deferred (deliberate):**
- ~~**#442** create-offering price auto-suggest — needs a product decision (margin/heuristics).~~ → **CLOSED** (`c14cb939`; see above). Kept struck-through as a historical record.
- **#444** remaining large-file splits — roadmap filed (`docs/plans/2026-07-25-large-file-splits-444.md`).
- **#436 success-screen auto-redirect bonus** — skipped at the time; filed as **#445** (now **closed** in the continuation session).
- **`scripts/browser.js --seed`** onboarding-flag tooling note — minor test helper, documented in-repo.
- **22 deliberate hex non-fit sites** — documented in `docs/audits/2026-07-25-code-robustness.md`.
- **2 pre-existing `recovery-flow` e2e failures** — filed as **#446** at the time; **RESOLVED** in the continuation session (`9b168add` — stale assertions, not a frontend bug).

### 2026-07-24 session (ICPay retirement + test stabilization)

ICPay (the ICP cryptocurrency payment rail) fully retired — **Stripe is the sole rail** — then a
stabilization pass fixed the flakes exposed by the required DB reset (the migration 049 CHECK edit
changed its checksum). GH **#443** (boot-gate `require_icpay_in_prod`) closed as moot; **#420**
(ICPay payouts) moot. Final baseline: full e2e **299 passed, 0 failed, 2 workers, ~6.6m**; smoke
**23 tests, ~29s**.

| Fix | Area | Resolution |
|-----|------|------------|
| ICPay payment rail fully retired — Stripe is the sole rail | Backend + frontend + config | Backend (`PaymentMethod` enum ICPay variant, `icpay_client`, escrow release/payout subsystem, webhook, endpoint, schema columns + `payment_releases` table, migration 049 CHECK rewrite), frontend (`RentalRequestDialog` Stripe-only, ICPay SDK pkgs removed, admin payout subsystem, env/compose/secrets), config all removed. `payment_method` default → `'test'` (Test absorbs auto-succeed). Commits: `fb4328be` `a773bdd6` `0b564bf9` `02ed7c2a` `1215b077` `1fc1d87f` `5c165eee` `2b18e4c7` `e9b7e0f3` `f0f44c8e` `5a228a63`. Dead `loadStripe` client + stale 'ICP (Internet Computer)' onboarding label also removed (`e7d7b3e4`). |
| Test stabilization (post DB-reset) | Test / reliability | `offering-status-badge` spec made self-seeding — was ambient-data-dependent, broke on DB reset (`469f48b6`); route-audit hardened against transient SvelteKit navigation fetch races (`b473e14c`); local e2e default workers 4→2 for reliability under persistent harness CPU load (`f1b9f088`). |

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
| H2 | Transfers page has no Send/Receive UI | **Feature gap** — P2P send needs IC canister integration (product decision, not a bug). Balance card already explains rentals are per-transaction at checkout. Related: #433 (closed, small-fix), #420 (closed 2026-07-24 — ICPay rail retired). |
| M5 | Billing VAT country EU-only | **Known limitation** — global country list needs server-side VAT rule changes. Low priority pre-launch. |
| L1 | Security page: seed-login device is 'Unnamed Device' | **Minor UX** — consider prompting device name on first login. |

### E2E harness tech debt (in-repo, surfaced 2026-07-23)

| Finding | Status |
|---------|--------|
| Full suite 192s for 205 tests; <60s goal needs multi-stack sharding | **Empirically investigated — sharding does NOT help on this box.** Built full harness (`scripts/e2e-shard.sh`, `dev-server.sh` STACK_INDEX, `fixtures/api-base.ts`). Root cause: 3 shard stacks share ONE Postgres → competing pools = worse DB contention than single-stack's single pool (3×4w=22 fails/4m30s; 3×2w=4 flakes). **Single stack 4 workers = 205/0 green ~192s = proven optimum.** For sharding to truly help, each shard needs its own Postgres instance (future CI-runner work). As a side benefit, dev CORS now correctly allows any localhost origin and the service worker no longer masks API errors. |
| `scripts/browser.js eval --seed <phrase>` throws "UtilityScript.evaluate" | **Minor tooling** — `authenticatePage` (browser.js:332-336) does an extra `goto`+`networkidle`+300ms after seed inject; a SvelteKit client-side redirect/WelcomeModal likely destroys the eval context. `snap`/`shot`/`errs`/`html`/`tour` all work with `--seed`; only `eval` is affected. For authed JS eval, use the e2e framework. |
| `scripts/browser.js --seed` greedily consumes positional args | **Minor tooling** — `--seed <phrase> <url>` fails ("Got 14 words") because the parser consumes all subsequent non-flag args as seed words. Documented usage `snap <url> --seed "$SEED"` (seed last) works. One-line fix possible but it's a test helper, not product. |
| Coverage gap: rent→pay→view→cancel happy path (UI-created contract, not DB-seeded) | **CLOSED (2026-08-01)** — `rent-flow.spec.ts` (4 serial tests) drives the REAL marketplace Rent dialog → signed `POST /api/v1/contracts` → rentals list → detail page → signed `PUT .../cancel`, all against the warm stack. The contract commits at `requested` (cancellable) during create, before Stripe checkout, so the flow is drivable without `STRIPE_SECRET_KEY`. Cancel asserted from BOTH the detail page and the rentals-list card, with DB verification. Only the Stripe SDK script load (external boundary) is mocked. |
| Coverage gap: provider agent-pool mgmt `/dashboard/provider/agents/[pool_id]` | **PARTIALLY CLOSED (2026-07-25, `2d82a6d5` C2)** — pool create + rename PUT + detail-page render now covered in `agent-pool-edit.spec.ts`. Remaining gap: pool revoke/delete UI path (low priority). |

### Deferred product decisions (surfaced 2026-07-23 UX review)

| Finding | Status |
|---------|--------|
| No `?` keyboard-shortcut help overlay | **RESOLVED (2026-07-23; stale entry corrected 2026-07-24)** — `KeyboardHelpOverlay.svelte` exists and is covered by 3 tests in `keyboard-shortcuts.spec.ts` (`? opens help overlay listing all shortcuts` is `@smoke`). This row was stale; corrected. |
| Dashboard shows provider-monitoring stats to brand-new renters | **Design judgment** — fresh renters see "Infrastructure Uptime", "Contracts Monitored", "Red Flags Detected" cards. Non-Providers may find this confusing. Needs product input on conditional rendering. |
