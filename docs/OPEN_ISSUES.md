# Open Issues

**Snapshot:** 2026-07-20. **Canonical source:** GitHub Issues at `decent-stuff/decent-cloud`
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
| 418 | Decent Agents: beta onboarding (invite + first-run demo) | launch | First user-facing DA flow. |
| 433 | No UI to top up account balance — `/dashboard/transfers` only shows history | launch | Found during 2026-07-20 UX audit (`/dashboard/transfers` has no Add Funds / Deposit button). |
| 427 | Anthropic API key proxy/sidecar for per-identity isolation | decent-agents, launch | Required for multi-tenant DA isolation. |
| 416 | Decent Agents: usage metering + customer-facing usage dashboard | decent-agents | |
| 415 | Decent Agents: subscription billing with active-hour + Claude token caps | decent-agents | |
| 410 | Stripe: cleanup stale pending contracts (payment timeout) | stripe | |

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
| 435 | Offering detail SLA chart renders empty gray bars when provider has no SLA data | 2026-07-20 visual audit |

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

## Recently closed by this work (2026-07-20)

| # | Title | Resolution |
|---|-------|------------|
| 434 | Flaky test: `account-notifications.spec.ts` in parallel runs (workers>1) | False alarm — fixed in `81615b77` (P3.5 mock audit): replaced `page.check('#notify-*')` with `locator(...).click()` after `waitFor('visible')` because `waitForLoadState('networkidle')` returned before Svelte finished hydrating channels. |

## In-repo known issues (not on GitHub)

- **Marketplace empty-state hint suggests `product_type:gpu` field syntax but the API rejects it**
  (`Unknown field: product_type`). Pre-existing product bug at
  `website/src/routes/dashboard/marketplace/+page.svelte:1307`. Not fixed in 2026-07-20 session;
  `search-dsl.spec.ts` tests use `price:<=N` instead, which does work.
