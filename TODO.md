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

- **[Offerings] Per-offering analytics** — DONE: `offering_views` table (migration 029), `POST /offerings/{id}/view` (public, IP-hash deduplication per day), `GET /offerings/{id}/analytics` (provider-only), view tracking on marketplace detail page, views shown on provider offerings list. Daily view trends: `GET /offerings/{id}/view-trends?days=30` returns `Vec<DailyViewTrend>` (day, views, unique_viewers); sparkline shown in provider offerings list.
  - **Remaining:** Click-through rate (views → rentals) conversion funnel. *(Note: overall conversion rate per offering is now shown on the Analytics page at `/dashboard/provider/analytics`.)*

- **[Marketplace] Offering comparison page** — DONE: `/dashboard/marketplace/compare?ids=1,2,3` with side-by-side specs, best-value highlighting, rent buttons, and compare toolbar on marketplace listing.

- **[Rentals] Contract lifecycle timing** — DONE: Expected time estimates per stage and overdue warning (with Contact Provider / Cancel actions) in rental detail page.

- **[Dashboard] Tenant spending insights** — DONE: Monthly spending widget on dashboard for tenants, showing this month vs. last month, trend direction, top 3 active contracts by cost, and projected month-end spend.

- **[Security] Two-factor authentication (TOTP)** — TOTP-based 2FA for accounts using email/password (not seed-phrase accounts which already have key-based auth). *(Multi-session: TOTP secret generation, QR code display, verification middleware.)*

- **[Global] Dark/light mode toggle** — Theme switcher in dashboard header. Persist in localStorage. *(Multi-session: the app is currently dark-only; adding a light theme requires defining a full light-mode color palette and updating all components with conditional classes. Not trivial.)*

- **[Provider] Provider performance analytics** — DONE: `/dashboard/provider/analytics` page shows per-offering view-to-rental conversion rates (views 7d/30d, rentals 7d/30d, conversion %, revenue 30d). Backend: `GET /providers/{pubkey}/offering-conversion-stats` (authenticated, provider-only). Sidebar nav item added. DONE: Tenant satisfaction trends — `GET /providers/{pubkey}/offering-satisfaction-stats` returns per-offering `service_matched` and `would_rent_again` counts + composite satisfaction rate %; color-coded table on analytics page.
  - **Remaining:** Pricing elasticity insights. *(Multi-session: needs price-history tracking joined with rental volume data.)*

- **[Provider] Request filtering and bulk actions** — DONE: Provider can filter pending rental requests by offering (dropdown) and duration range (min/max hours). Filtered set is used by Accept All / Reject All batch actions. DONE: Rule-based auto-accept — `auto_accept_rules` table (migration 032), full CRUD API, duration-threshold enforcement in provisioning service, UI panel on requests page.
  - **Remaining:** Nothing — auto-accept is fully implemented.

- **[Tenant] SSH key onboarding guidance** — DONE: Rental request dialog now has platform-specific tabbed SSH key generation guide (macOS/Linux, Windows PowerShell, Windows PuTTY) with copy buttons for each command.

- **[Marketplace] Trending and new providers sections** — DONE: `GET /api/v1/offerings/trending` (top offerings by 7-day views) + "Trending this week" carousel. DONE: `GET /api/v1/providers/new` (providers joined last 90 days with public offerings) + "New to the platform" provider cards on marketplace. Migration 031 adds `created_at` to `provider_profiles`. DONE: Plain-text search — `text_search` field on `SearchOfferingsParams`; queries without `:` now route to ILIKE name/description match instead of DSL parser (which required `field:value` syntax).
  - **Remaining:** "Recommended for you" personalized section. *(Needs user behavior tracking and personalization logic. Multi-session.)*

- **[Provider] Provider public profile and reputation deep-dive** — Tenants cannot view a provider's historical trust score trend, feedback breakdown by offering type, or SLA violation history. No provider comparison tool. *(Multi-session: historical trust data endpoints, profile page with timeline, comparison view.)*

- **[Offerings] Draft offerings scheduling** — DONE: `publish_at` field (migration 030) on offerings. When `is_draft=true` and `publish_at <= NOW()`, `PublishScheduledService` (60s interval) auto-publishes. UI: schedule picker on create/edit pages (shown when draft=true), "Scheduled" badge with publish time on offerings list. DONE: Bulk-publish — `POST /offerings/bulk-publish` (provider-only, 1-100 ids) with checkbox UI on offerings page.
  - **Remaining:** "What changed since last save" diff view. *(Multi-session: needs per-field change tracking.)*

- **[Tenant] Saved offerings price-change alerts** — Tenants can save offerings but receive no notification when a saved offering changes price or goes out of stock. *(Multi-session: needs price-history tracking table, notification integration.)*
