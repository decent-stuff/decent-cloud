# Marketplace buy-flow execution — seed, verify, fix friction, codify

**Created:** 2026-08-11
**Status:** Phase 1 (real provisioning) + Phase 2 (UX audit + fixes) COMPLETE. Phase 3 (codify
wallet+provisioning e2e) remaining.
**Aligns to:** `docs/PRODUCT-DIRECTION.md` (north star) + `docs/OPEN_ISSUES.md` § #0.
**Related:** `docs/plans/2026-08-03-hetzner-first-offerings.md` (milestone plan — this is its execution log).

## Goal

Make the **#0 product priority** real: a new user can **discover → rent → pay → provision → SSH →
cancel** with minimal friction, against a REAL Hetzner-backed offering. This plan is the concrete
execution (seed → verify → fix → codify), not the milestone rationale (see the 2026-08-03 plan).

## Phase 1 — Seed + verify real cloud-resell provisioning ✅ DONE

**Outcome:** the full `discover → rent → provision → SSH → cancel` lifecycle was verified against a
REAL Hetzner cx23 VM (one VM, created then deleted, zero orphans). Confidence **8/10** the
cloud-resell path is production-ready.

### What was seeded (LOCAL dev DB only, NOT prod)
- **Reseller identity** registered from `DC_PROD_RESELLER_SEED` (derived pubkey `1ed6136d…f53d`
  matches `DC_PROD_RESELLER_PUBKEY`). `account_public_keys` + `provider_profiles` rows created.
- **Hetzner cloud account** `operator-hetzner-dev` (`cloud_accounts` id `23633710-…`, token
  live-validated + encrypted, `is_valid=true`).
- **Cloud-resell offering db id 1628** — slug `hetzner-cx23-dev-msnph3b3`,
  `provisioner_type=hetzner`, `provisioner_config={cx23,nbg1,ubuntu-24.04}`, `currency=USD`,
  `monthly_price=6`, `visibility=public`, `agent_pool_id=NULL`. Marketplace-visible.

### Verified lifecycle (contract `89c5ab77…`)
`requested → accepted` (instant, auto_accept) → `active` (~56s provisioning) → `cancelled`.
`provisioning_instance_details={public_ip:"46.225.238.4",ssh_port:22}`.
**SSH worked:** `ssh root@46.225.238.4` → `PROVISION_OK`, hostname `dc-recipe-89c5ab772404`,
Ubuntu 24.04. Cancel → VM deleted (Hetzner GET 404), SSH key removed, zero orphans (3 checks).

### Bugs found + fixed (TDD, 4 commits)
| Bug | Commit | Summary |
|-----|--------|---------|
| BUG-1 | `a1624cf3` | `dc-auth.js` cancel used POST; endpoint is PUT. |
| BUG-2 | `53e3479f` | Email-verify gate blocked ALL dev rentals (no local email path). Dev bypass: `is_production_env()` gates auto-verify at account creation; rental-time gate LEFT INTACT for prod. |
| BUG-3 | `d9c31073` | `get_provider_auto_accept_rentals` defaulted FALSE (contradicting schema TRUE) → cloud-resell rentals hung at `requested`. Fixed `unwrap_or(false)`→`unwrap_or(true)`. |
| FRICTION-5 | `39dd719c` | Cloud-resell instance_details had misleading `gateway_*` fields. New `build_direct_ssh_instance_details()` emits only `{connection_type:"direct_ssh",public_ip,ssh_port:22}`. |

**Product question for operator:** should the rental-time email-verification gate
(`contracts.rs:659-668`) block rentals in PROD? Currently it does (anti-Sybil). Dev bypass added at
account-creation only; prod unchanged.

## Phase 2 — Browser UX audit + friction fixes ✅ DONE

**Outcome:** a real-browser UX audit (no mocks, no cloud spend) found **17 friction points**; the
9 highest-value were fixed (9 commits). The buy flow is now: honest stats, accessible keyboard-
friendly rent dialog with inline wallet balance, no dead-end links, trimmed form.

### Fixes shipped (9 commits, all `andris-k85`, NOT pushed)
| Area | Commit(s) | Fix |
|------|-----------|-----|
| Rent dialog | `48ab7aa8` | Real `<form>`, autofocus, focus trap, Escape, `role="dialog"`/`aria-modal` (mirrors KeyboardHelpOverlay). TDD: `rent-dialog-keyboard.spec.ts`. |
| Rent dialog | `57e47e70` | Inline wallet balance + sufficient/insufficient indicator in payment step. |
| Honest stats | `e6e611fe` | New `is_rentable_now()` single source of truth; `/api/v1/stats` 6/0/7 → 1/1/1 (matches the 1 rentable offering). |
| Marketplace | `ce53c5c3` | Similar Offerings no longer links to dead/offline offerings (shared `isOfferingRentable`). |
| Marketplace | `22bafed2` | Dead quick-filter presets (GPU/NA → 0 matches) hidden via live-data availability. |
| Marketplace | `2b102d42` | Duplicate desktop sort control removed (select `md:hidden`). |
| Marketplace | `789529b4` | SLA empty-card em-dash wall → warm "new provider" copy. |
| Rentals | `7b0e8f9e`+`1e15dd51` | Provider name now clickable (links to profile, matches marketplace). |

### Known limitation
At Playwright's 1280×720 viewport the rent dialog's SSH field + Pay button still sit just below the
fold (Resource Details consumes the top). On typical ≥900px desktops they fit. Fully fixing 720px
needs collapsing Resource Details (deferred — useful context, out of this session's scope).

### Verification
- Full e2e suite: **329/331 pass** (8.9m). The 2 failures are **pre-existing**, both confirmed:
  - `payment-flows:167` (webhook 500) — needs `STRIPE_WEBHOOK_SECRET` (not set in slim local stack);
    fails in isolation too. Pre-existing.
  - `billing-settings:45` (navigation) — passes in isolation; test-order contamination hazard
    (shared authenticated state). Pre-existing.
- Backend `cargo clippy --tests -p api`: clean. `npm run check`: 0 errors.

## Phase 3 — Codify wallet+provisioning buy flow as e2e (REMAINING)

The `rent-flow.spec.ts` covers discover→rent→cancel at `status='requested'` (before payment). The
**gap** is the wallet-payment + real-provisioning path. To codify WITHOUT per-run cloud spend:
- Extend the rent flow to assert **wallet debit** on a non-cloud-resell (auto-advancing) offering —
  covers the money path (top-up → debit → refund-on-cancel) without provisioning a VM.
- Add a **gated** real-provisioning spec (cloud-resell offering 1628 → provision → SSH assertion →
  cancel) guarded behind an env flag (`E2E_REAL_PROVISIONING=1`) so it runs only on explicit
  request (cheapest cx23, immediate cancel). Default-off to avoid routine spend.
- Update `website/tests/e2e/FLOWS.md` with the new flow rows.

## Operator decisions needed
1. **Publish real offerings to PROD** (the only remaining gate to a non-empty public marketplace —
   B8). Local verification is complete; prod needs the operator's go-ahead to spend + go public.
2. **Email-verify gate in prod** — keep (anti-Sybil, current) or relax? (Phase 1 product question.)
3. **Confirm Phase 3 real-provisioning e2e approach** is acceptable (gated, default-off, cheapest VM).
