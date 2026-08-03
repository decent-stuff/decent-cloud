# Hetzner first-offerings milestone — operator resells Hetzner

**Created:** 2026-08-03
**Status:** Proposed — needs operator's Hetzner creds + provider identity to execute.
**Aligns to:** `docs/PRODUCT-DIRECTION.md` (the authoritative north star — "OpenRouter,
but for cloud resources").
**Related:** `docs/OPEN_ISSUES.md` § "Real-deployment smoke audit (2026-08-03)"
(finding #2 — prod marketplace empty); `docs/specs/2026-02-14-hetzner-provisioner.md`
(background mechanics).

## Goal

List the **first REAL offerings** on the marketplace, operator-provided, backed by
**Hetzner** capacity. The operator becomes the platform's first real provider. This
ends the **honest-empty-marketplace** period: today the catalog is empty on purpose
(the demo/synthetic offerings were dropped in migration `053`, `c9dfa9d8`, so the
marketplace no longer shows fake placeholder rows) — but "honestly empty" is not the
goal, just the honest interim. Real, purchasable, provisionable offerings are.

This is the concrete near-term step the product direction names: drop demos (done) →
add the first real offerings by reselling Hetzner (this milestone) → generalize later.

## Why this works with the existing code (no new provisioning code)

The Hetzner integration already exists and is wired into the contract lifecycle —
**this is primarily an operational + data task, not new provisioning code.**

- **Hetzner `CloudBackend`** lives at **`api/src/cloud/hetzner.rs`**
  (`HetznerBackend` implements the `CloudBackend` trait: create/list/delete servers,
  SSH-key upload, catalog of server types/locations/images, metrics).
  > Note: the provider-side `dc-agent/src/provisioner/` (digitalocean / docker /
  > proxmox / manual / script) has **no** Hetzner provider — the Hetzner path is the
  > central-API direct cloud-backend, not a dc-agent provisioner.
- **Orchestration** is in `api/src/cloud_provisioning_service.rs`
  (`make_backend()` dispatches `BackendType::Hetzner` → `HetznerBackend::new(token)`).
- **Contract provisioning** already resolves Hetzner config
  (`api/src/database/contracts/provisioning.rs` calls
  `cloud::hetzner::resolve_provisioner_config`).
- **Credentials** are stored per-provider via `api/src/database/cloud_accounts.rs`
  (`BackendType::{Hetzner, ProxmoxApi, Vultr}`).
- **CLI e2e** already exercises the Hetzner cloud-provisioning path end-to-end
  (`api/src/bin/api-cli/e2e.rs` reads `HETZNER_API_TOKEN`).
- **Provider onboarding** (F9, `2c393df9`) now routes "Become a Provider" to real
  technical onboarding at `/dashboard/provider/start`.

So: provision, sign, rent, cancel all work for a Hetzner-backed offering today. What
is missing is **operator data + identity**: a registered provider account, real
offerings with real specs/prices/currency, and the operator's Hetzner creds attached.

## Prerequisites (operator-gated)

1. **Operator's Hetzner API token.** `HETZNER_API_TOKEN` already exists in the
   consolidated outer `secrets/shared/env.yaml` (age-SOPS). Confirm it is attached as
   a `cloud_account` for the operator's provider identity (see open question 3).
2. **Provider identity.** Register the operator as a **provider** in the central API
   (website provider onboarding, now `/dashboard/provider/start` per F9) and set a
   real provider display name (not an auto-generated `@handle`).
3. **Currency / pricing.** Stripe-supported currencies only — `usd` / `eur` / `gbp` /
   `jpy` / `cad` (per `is_stripe_supported_currency`; ICP is retired as both a payment
   rail and an offering currency).
4. **(Investigation) Gateway vs. direct-resell path.** Confirm whether listing a
   Hetzner-backed offering **requires** the operator to run a dc-agent/Proxmox gateway
   host (the provider-side model: Proxmox + Caddy + acme-dns + port-range routing),
   or whether the **direct cloud-backend path** (`api/src/cloud/hetzner.rs` +
   `cloud_provisioning_service.rs`, which already provisions Hetzner VMs from the
   central API without a gateway) is sufficient for a pure-resell offering. See open
   questions.

## Steps (high-level, ordered)

1. **Register the operator as a provider** + set the provider display name
   (`/dashboard/provider/start` flow).
2. **Attach the Hetzner credential** to that provider (`cloud_account`,
   `BackendType::Hetzner`, `HETZNER_API_TOKEN`).
3. **Provision via dc-agent / cloud-backend** with the Hetzner creds and confirm a VM
   can be created + destroyed against the operator's account.
4. **Create the real offerings** (CLI `create-offering` or the website
   create-offering flow) with **real specs / real prices / Stripe-supported currency**,
   mapped to the operator's provider + pool/dc. Price auto-suggest (`#442`, cost × 1.15
   markup) is available as a starting point.
5. **Verify end-to-end:** a renter can **discover → rent → provision → SSH → cancel**.
   Per `AGENTS.md` § MINIMIZE CLOUD SPENDING: use the **cheapest `cx22`** Hetzner
   server type, and **delete the VM immediately** after verification — never leave a
   test VM running unattended.
6. **Seed to prod** (or stage-first then promote): once the offering verifies clean on
   `dc-stage`, promote to `dc-prod` so the public marketplace is no longer empty.

## Open investigation questions (flag, don't fabricate)

These need a concrete read of the offering↔backend wiring + the operator's setup
before execution; they are **not** blockers to writing this plan, but they must be
answered (or explicitly deferred) before step 1:

1. **Gateway model.** Does listing a Hetzner-backed offering require the operator to
   run a Proxmox gateway host (dc-agent provider-side model), or is the **direct
   cloud-backend path** (`api/src/cloud/hetzner.rs`) a complete pure-resell path with
   no gateway VM? The direct path exists and is wired into contract provisioning, but
   whether a marketplace offering can target it **without** a registered gateway/pool
   is unconfirmed. **This is the key question for "how little the operator must run."**
2. **Offering → provider pool / dc mapping.** How does a specific offering bind to a
   specific provider's pool / datacenter (so a renter is routed to the operator's
   Hetzner capacity, not a generic pool)? Confirm the offering-create fields and the
   routing.
3. **Operator account flags.** Does the operator's provider identity need any special
   flagging to list offerings / receive rentals (vs. a normal provider)? Confirm there
   is no allowlist/manual-approval gate in the way.

## Out of scope

- **Multi-provider.** This milestone is **Hetzner-only** (the operator as the single
  first provider). Onboarding arbitrary third-party providers / arbitrary clouds is the
  long-term platform vision, not this milestone.
- **The long-term "any provider, any cloud" API unification** — covered by
  `docs/PRODUCT-DIRECTION.md`; this milestone is the first concrete instance of it,
  not the generalization.
- **F6 top-providers leaderboard** — deferred (premature until real offerings exist;
  ranking demo/synthetic providers would mislead). Natural follow-up **after** this
  milestone lands.
