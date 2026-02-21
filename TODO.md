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

DB tables (`contract_health_checks`), API endpoints, and automated health check scheduling in dc-agent are implemented. Provider SLA Monitor page at `/dashboard/provider/sla` shows per-contract uptime with health check history. Tenant health check view available on contract detail page.

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

### Hardcoded Token Value ($1 USD) in IC canister

**Issue:** The IC canister's `refresh_last_token_value_usd_e6()` always returns `1_000_000` ($1 USD). The api-server now fetches real ICP/USD price from CoinGecko for UI display (cached 5 min, `GET /api/v1/prices/icp`), but the on-chain canister price remains hardcoded. The canister uses ICP HTTP outcalls to fetch external data.
**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`
**Fix:** Use IC HTTP outcalls to fetch from ICPSwap or KongSwap. *(Blocked on choosing exchange API and implementing HTTP outcalls in the canister. Single-session once decided.)*

---

## UX Improvements

### Backlog

- **[Cloud] Stock tracking for self-provisioned resources** — When a cloud resource is listed on the marketplace, multiple tenants could theoretically rent the same VM. Needs: `stock` field on cloud_resources, 1-to-1 rental enforcement, automated credential sharing when contract is accepted. *(Blocked: billing decisions first.)*

- **[Offerings] Per-offering analytics** — DONE: `offering_views` table (migration 029), `POST /offerings/{id}/view` (public, IP-hash deduplication per day), `GET /offerings/{id}/analytics` (provider-only), view tracking on marketplace detail page, views shown on provider offerings list.
  - **Remaining:** Time-series analytics (trend over weeks), click-through rate (views → rentals), conversion funnel. *(Multi-session: needs additional DB aggregation and UI charts.)*

- **[Marketplace] Offering comparison page** — DONE: `/dashboard/marketplace/compare?ids=1,2,3` with side-by-side specs, best-value highlighting, rent buttons, and compare toolbar on marketplace listing.

- **[Rentals] Contract lifecycle timing** — DONE: Expected time estimates per stage and overdue warning (with Contact Provider / Cancel actions) in rental detail page.

- **[Dashboard] Tenant spending insights** — DONE: Monthly spending widget on dashboard for tenants, showing this month vs. last month, trend direction, top 3 active contracts by cost, and projected month-end spend.

- **[Security] Two-factor authentication (TOTP)** — TOTP-based 2FA for accounts using email/password (not seed-phrase accounts which already have key-based auth). *(Multi-session: TOTP secret generation, QR code display, verification middleware.)*

- **[Global] Dark/light mode toggle** — Theme switcher in dashboard header. Persist in localStorage. *(Multi-session: the app is currently dark-only; adding a light theme requires defining a full light-mode color palette and updating all components with conditional classes. Not trivial.)*

- **[Provider] Provider performance insights dashboard** — Providers have separate earnings, feedback, and SLA pages but no unified analytics view. Needs: a new `/dashboard/provider/analytics` page showing which offerings are most rented, view-to-rental conversion rates, tenant satisfaction trends, and pricing elasticity insights. *(Multi-session: needs time-series offering analytics backend endpoints.)*

- **[Provider] Request filtering and bulk actions** — Provider cannot filter pending rental requests by offering type, duration, or tenant trust score; cannot bulk-accept/reject; cannot set auto-accept rules. *(Single-session: client-side filtering + bulk action UI on existing endpoint.)*
  - Dependency: Would benefit from offering analytics (views vs. rentals) to inform auto-accept thresholds.

- **[Tenant] SSH key onboarding guidance** — Tenants renting for the first time receive connection details with no guidance on generating SSH keys, no platform-specific instructions (Windows/Mac/Linux), and no "test connection" button. *(Single-session: expand connection details section in rental detail page with collapsible SSH help.)*

- **[Marketplace] Trending and recommendations section** — Marketplace has search but no proactive discovery: no "Trending this week", "New providers", or "Recommended for you" sections. Needs: trending endpoint (most-viewed/rented in last 7d from `offering_views`), frontend recommendation cards. *(Single-session backend + frontend once `offering_views` accumulates data.)*
  - Dependency: Requires `offering_views` data (migration 029, now live).

- **[Provider] Provider public profile and reputation deep-dive** — Tenants cannot view a provider's full public profile: historical trust score trend, feedback breakdown by offering type, response-time statistics, or SLA violation history. No provider comparison tool. *(Multi-session: historical trust data endpoints, profile page, comparison view.)*

- **[Offerings] Draft offerings scheduling** — Providers can create draft offerings but cannot schedule a future publish date, bulk-publish drafts, or see what changed since last save. *(Single-session: add `publish_at` field to offerings, scheduled publish logic in backend, UI controls.)*
  - Dependency: `is_draft` field already exists (migration 027).

- **[Tenant] Saved offerings price-change alerts** — Tenants can save offerings but receive no notification when a saved offering changes price or goes out of stock. *(Multi-session: needs price-history tracking table, notification integration.)*
