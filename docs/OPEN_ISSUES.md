# Open Issues

> **Hard rule: Do not add entries with value below 4.** Every row must carry `value [1-10]` and
> `effort [S/M/L]`. If a piece of work is done, remove it from this file — this is a quick
> reference of what is LEFT, not a historical log. Completed work lives in `git log`.

**Canonical source:** GitHub Issues at `decent-stuff/decent-cloud`
(`gh issue list --repo decent-stuff/decent-cloud --state open`). This file is a categorized
snapshot for quick local reference; GitHub remains the source of truth. Reconcile drift before
acting.

```bash
gh issue list --repo decent-stuff/decent-cloud --state open --json number,title,labels
```

## Product north star

**decent-cloud is "OpenRouter, but for cloud resources"** — a proxy/reselling platform unifying
many cloud providers behind one common API. Near-term: the operator resells Hetzner. The
marketplace buy flow (discover → rent → pay → provision → SSH → use → cancel) was verified
end-to-end against a real Hetzner cx23 VM and merged (PR #479). **Going public is now the gate.**

## Open items

| ID | Description | value [1-10] | effort [S/M/L] | Status / notes |
|----|-------------|:---:|:---:|----------------|
| **PROD-LIVE** | Publish real Hetzner-backed offerings to the PROD marketplace + authorize ongoing spend | **10** | **S** | The local path is fully verified + merged (PR #479, commit `05849932`): discover → rent → pay → provision → SSH → cancel against a real cx23 (created ~56s, zero orphans). All creds present (`HETZNER_API_TOKEN_DEV`, `DC_PROD_RESELLER_SEED`). **Blocked only on operator sign-off to go PUBLIC.** Plan: `docs/plans/2026-08-11-marketplace-buy-flow-execution.md`. |
| **PR-487** | Test DB pool recycling (eliminates ~2600 CREATE/DROP per CI run) | **7** | **S** | Code written on PR #487 (`fix/test-db-pool-recycling`); awaiting review + merge. |
| **EMAIL-GATE** | Should the email-verification gate block rentals in PROD? | **6** | **S** | Operator decision. The gate currently hard-rejects unverified email at contract-create with an upstream warning (UX shipped). In PROD the tradeoff is fraud/spam prevention vs. buyer friction. Needs an operator call on whether to keep, soften, or remove it for the public launch. |
| **A5** | Concurrent ticket processing in dc-agent (`JoinSet`+`Semaphore`) | **5** | **M** | Bottleneck = serial `for contract in &contracts` at `dc-agent/src/main.rs:1739`. Design complete: codebase is already concurrency-safe (provisioners `Send+Sync`, deterministic VMID, gateway `Arc<Mutex>`); add `max_concurrent_provisioning` config knob with **DEFAULT=1** (behaviorally identical). Ship machinery first, raise N only after operator verifies node headroom. **Deferred — needs operator sign-off on concurrency level.** |
| **TD-1** | #444 — Large source-file splits (>2700 lines) | **5** | **L** | Deferred. `accounts.rs` 2230→973 DONE (2026-08-11). Remaining: `dc-agent/src/main.rs` **3681L**, `api/src/openapi/providers.rs` **4082L**, `api/src/database/offerings.rs` **2981L**, `api/src/database/cloud_resources.rs` **2701L**. No clean decoupled clusters — needs focused design passes, not the wave cadence. `spec_snapshot` guard locks byte-identical OpenAPI for any future `*Api` split. Roadmap: `docs/plans/2026-07-25-large-file-splits-444.md`. |

## Deferred epics (de-prioritized)

> **Decent Agents epic (C1/C2/C3)** is de-prioritized but NOT stopped. Hetzner resell has now
> landed (PR #479), so this is unblocked in principle — but it remains a future multi-session
> investment, not near-term work. Specs are valid.

| ID | Description | value [1-10] | effort [S/M/L] | Status / notes |
|----|-------------|:---:|:---:|----------------|
| **C1** | #418 — Decent Agents beta onboarding (magic-link → Stripe → GitHub App → demo PR → invite gate) | **4** | **L** | Multi-week new product surface. Spec: `2026-04-25-decent-agents-github-integration-spec.md`. Needs the identity-provisioning foundation + GitHub App onboarding flow (no webhook receiver exists yet). |
| **C2** | #427 — Anthropic API key reverse proxy (per-identity isolation + metering) | **4** | **L** | Core shipped (`anthropic-proxy` crate, 33 tests). Acceptance #3/#4 (remove shared-key mount + migrate beta) need the #413 identity-provisioning subsystem. |
| **C3** | #415/#416 — Decent Agents billing (subscriptions + active-hour/token caps) + metering dashboard | **4** | **L** | Depends on #427 dispatch enforcement + new `agent_runs`/metering tables. `STRIPE_SECRET_KEY` present. |

## Deferred post-launch (≥20 paying customers)

Tracked in GitHub as `deferred-post-launch`. Valid but parked until the platform has real traction.

| # | Title | value [1-10] | effort [S/M/L] |
|---|-------|:---:|:---:|
| 429 | Decent Agents: Anthropic key exfiltration mitigation (read-only mounts, egress monitoring) | 4 | L |
| 430 | Decent Agents: CODEOWNERS / branch-protection deadlock surfaced at onboarding | 4 | M |
| 431 | Decent Agents: GitHub App webhook secret rotation procedure + ops runbook | 4 | M |
| 432 | Decent Agents: per-identity observability + incident response runbook | 4 | M |

## Future work (proposals, not yet filed)

| ID | Description | value [1-10] | effort [S/M/L] | Status / notes |
|----|-------------|:---:|:---:|----------------|
| **API-TOKEN** | Non-custodial API key / service token (scoped, revocable; auth without the master seed) | **6** | **L** | Today provider auth = Ed25519 master key (BIP-39 seed), so automation must hold the root credential. Proposal: hashed revocable tokens in the DB (mirroring `cloud_accounts.credentials_encrypted`), a token-auth path alongside Ed25519 in `common/src/api_auth.rs`. Aligns with the operator-resell-at-scale direction; removes root-credential exposure from automation. Not blocking — the env.yaml-seed path works today (`repo/AGENTS.md` → "Acting as an existing provider identity autonomously"). |

## Killed / obsolete (deliberately not pursuing)

| Spec | Reason |
|------|--------|
| `2025-12-07-provider-catalog-seeding` Phase 1/2 (catalog verification + provider claim flow) | Predates operator-resell direction; curated seeding + claim flow off the roadmap. Phase 1A scraper kept. |
| `2026-02-14-decent-recipes` author-commission revenue model | Superseded by the operator-resell model. Recipe execution itself is kept; only the commission split is killed. |
| `2025-12-25-provider-tunnel-relay` (frp relay) | Superseded by the public-IP-per-host gateway architecture. |

## Scope rules (per `repo/AGENTS.md` + `repo/PROMPT.md`)

- **In scope:** labeled `launch`, `stripe`, or `decent-agents` WITHOUT `deferred-post-launch`.
- **Deferred:** labeled `deferred-post-launch`. Valid but parked until ≥20 paying customers.
- **IMPORTANT 7:** when an issue completes, remove it from this file and mark it completed/closed
  everywhere else it is mentioned. This file is NOT a changelog.
