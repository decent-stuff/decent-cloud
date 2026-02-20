# TODO

**Specs:**
- [docs/specs/2026-02-14-decent-recipes.md](docs/specs/2026-02-14-decent-recipes.md)
- [docs/specs/2026-02-14-self-provisioning-platform.md](docs/specs/2026-02-14-self-provisioning-platform.md) — Phases 1-4 complete (marketplace listing flow implemented).

---

## Cloud Provisioning

### Known Limitations

- **Multi-instance race** — If two API server instances share the same DB, both provisioning services race on the same resources. The 10-minute lock timeout prevents corruption but can cause delayed provisioning or double-attempt waste. *(Not a prod issue if only one instance runs. Only matters at scale.)*

### Remaining

- **Marketplace billing for listed resources** — Platform fee for marketplace-listed self-provisioned resources. *(Blocked: needs product decisions on fee structure.)*
- **Marketplace rental fulfillment for self-provisioned** — When a tenant rents a self-provisioned offering, the contract is created but the VM access handoff (credential sharing) is manual. Needs: automated credential sharing mechanism, stock tracking (one VM = stock of 1). *(Blocked: needs billing decisions first.)*

### Remaining (Recipes)

- **Recipe script versioning** — Scripts are snapshotted at contract creation. Consider a `recipe_versions` table so authors can update scripts and buyers can opt-in to upgrades. *(Multi-week: new DB table, migration logic, UI for version management.)*
- **Recipe validation / dry-run** — No way to test a recipe without creating a real contract. Consider: syntax check (shellcheck), dry-run mode that provisions a VM, runs the script, reports results, and tears down. *(Multi-session: needs a dedicated test-run flow distinct from purchase.)*
- **Standalone recipe entity** — Recipes are currently a text field on offerings. A `recipes` table would enable: reuse across multiple offerings, community browsing/forking, ratings, and independent authorship. *(Architectural change — needs design discussion.)*

---

## Provider Provisioning Agent (dc-agent)

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 8 (Hetzner server-side in api-server via `HetznerBackend`). Health check scheduling wired up in dc-agent main loop.

- Phase 9: Docker, DigitalOcean, Vultr provisioners *(Multi-week epic: each provisioner is a separate CloudBackend impl + credentials + testing.)*

---

## Provider Trust & Reliability System

DB tables (`contract_health_checks`), API endpoints, and automated health check scheduling in dc-agent are implemented.

- SLA compliance tracking and provider reputation scoring *(Blocked: needs product decisions on SLA metrics, scoring formula, how reputation affects discovery. Single-session once decisions are made.)*

---

## Notification System

### Paid Notification Tiers
- Define pricing for additional notifications beyond free tier
- Integrate with payment system (Stripe/ICPay)
- Track paid quota separately from free tier

*(Blocked: needs product decisions on pricing tiers before implementation. Multi-session: DB schema, Stripe integration, quota tracking.)*

---

## ICPay Integration

### Future: Automated Payouts
ICPay does not have a programmatic payout API. Currently payouts are manual via `GET /api/v1/admin/payment-releases` + icpay.org dashboard + `POST /api/v1/admin/payouts`. Implement direct ICRC-1 transfers from platform wallet using `ic-agent` when ICPay adds payout API support. *(Blocked on ICPay adding payout API.)*

---

## Architectural Issues Requiring Review

### Hardcoded Token Value ($1 USD)

**Issue:** Token USD value hardcoded instead of fetched from exchanges.
**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`
**FIXME in code:** `refresh_last_token_value_usd_e6()` always returns `1_000_000` ($1 USD). Needs ICPSwap/KongSwap integration. *(Blocked on choosing exchange API. Single-session once decided.)*

---

## UX Improvements

### Tenant (Renter) Experience

- **[Rental flow] No SSH key onboarding before checkout** — A user who has never added an SSH key hits the RentalRequestDialog and must manually paste one. There is no "Generate a key pair" helper, no link to a guide, and the textarea offers zero affordance for first-timers. The save-to-profile flow exists in the profile/security pages but the rental dialog never links there. A first-timer will abandon. **Fix:** Add a "Save this key to your profile" checkbox in the rental dialog, and a one-click "How to generate an SSH key" inline guide. *(Impact: High. Effort: 0.5 day)*

---

### Provider Experience

- **[Provider onboarding] Gatekeeping is opaque and confusing** — "My Offerings" and "Rental Requests" are hidden from the sidebar until onboarding is complete. The only hint is a banner on the Provider Setup page. A provider who goes directly to `/dashboard/offerings` (e.g. from bookmarks or documentation links) gets silently redirected or sees an empty "authenticated" page. **Fix:** Show a locked/dimmed state in the sidebar for gated items with a tooltip "Complete Provider Setup to unlock", so the gate is visible rather than the items being invisible. *(Impact: High. Effort: 1 hour)*

- **[Offerings] CSV-only bulk edit is not discoverable for non-technical users** — The primary way to create/edit offerings is a CSV spreadsheet modal. The "Create Offering" button leads to a form-based page, but the bulk-edit path (which is the only way to manage more than one offering) opens a raw CSV editor. There is no documentation inline about CSV format, column names, or required fields. **Fix:** Add a column reference guide (collapsible) directly above the CSV editor textarea. *(Impact: Medium. Effort: 2 hours)*

- **[Offerings] No delete action on the offerings page** — The offerings grid shows cards with visibility/stock toggles and an "Edit full details" link, but there is no delete button anywhere visible on the offerings list or card. A provider who wants to remove an old offering must navigate into the edit page to find it. **Fix:** Add a delete action (with confirmation dialog) directly on each offering card. *(Impact: Medium. Effort: 2 hours)*

- **[Offerings] "No pool" warning is shown but not actionable** — When offerings have no matching agent pool, an amber banner appears ("X offerings without matching pool — hidden from marketplace"). The banner explains the problem but provides no direct link to the Agents page where pools are configured. **Fix:** Add a "Configure Agents" button/link inside the warning banner. *(Impact: Medium. Effort: 15 minutes)*
