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

### Hardcoded Token Value ($1 USD) in IC canister

**Issue:** The IC canister's `refresh_last_token_value_usd_e6()` always returns `1_000_000` ($1 USD). The api-server now fetches real ICP/USD price from CoinGecko for UI display (cached 5 min, `GET /api/v1/prices/icp`), but the on-chain canister price remains hardcoded. The canister uses ICP HTTP outcalls to fetch external data.
**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`
**Fix:** Use IC HTTP outcalls to fetch from ICPSwap or KongSwap. *(Blocked on choosing exchange API and implementing HTTP outcalls in the canister. Single-session once decided.)*

---

## UX Improvements

### Completed (2026-02-21)

- **[Provider] Batch actions on rental requests — "Accept All" / "Reject All"** ✅
- **[Marketplace] Offering allowlist management** ✅ — UI available in `/dashboard/offerings` per-offering "Allowlist" button.
- **[Account] SSH keys on Security page** ✅ — `ExternalKeysEditor` added to `/dashboard/account/security`.
- **[Stats] Platform stats: total volume + transfers on homepage** ✅ — Dashboard shows 7 metrics including Total Volume (ICP) and Total Transfers.
- **[Navigation] Breadcrumb on agent pool detail page** ✅
- **[Contracts] Provisioning failure details visible to tenant** ✅ — Contract detail shows failure banner with `provisioning_instance_details` content and actionable next steps.
- **[Contracts] Bandwidth usage chart on contract detail** ✅ (2026-02-21) — New tenant-authenticated endpoint `GET /api/v1/users/:pubkey/contracts/:id/bandwidth` + SVG chart on `/dashboard/rentals/[contract_id]`.
- **[Provider] Trust metrics dedicated shareable page** ✅ (2026-02-21) — New route `/dashboard/reputation/[identifier]/trust` + "Share Trust Report" link on reputation page.
- **[Agent] Per-agent status accuracy** ✅ (2026-02-21) — DB migration adds `agent_pubkey` as primary key to `provider_agent_status`; heartbeat now keyed per-agent instead of per-provider.
- **[Account] Notifications tab in Account Settings** ✅ (2026-02-21) — New `/dashboard/account/notifications/+page.svelte` with email/Telegram/SMS toggles, per-channel test buttons, and today's usage stats. Added "Notifications" tile to Account Settings grid.
- **[Provider] Per-contract earnings breakdown** ✅ (2026-02-21) — Sortable "Contract Earnings" table added to `/dashboard/provider/earnings` showing offering, status, payment (ICP), duration, and created date for every provider contract. Data sourced from `GET /api/v1/users/:pubkey/activity`.
- **[Rentals] Search/filter rentals list** ✅ (2026-02-21) — Text search input above the status tabs on `/dashboard/rentals`; client-side filters by contract ID or offering name. Clear button and contextual empty-state message included.
- **[Marketplace] Default to hiding demo offerings** ✅ (2026-02-21) — `showDemoOfferings` default changed from `true` to `false`; `clearFilters()` also resets to `false`. Label already reads "Show demo offerings".
- **[Dashboard] Personalized activity on home for authenticated users** ✅ (2026-02-21) — Recent Activity section now shows expiry dates (amber if <24 h) on active tenant contracts, plus a new "As Provider (last 3)" subsection with payment amounts and a link to the earnings page.
- **[Pricing] Real ICP/USD price feed** ✅ (2026-02-21) — New `GET /api/v1/prices/icp` endpoint with 5-minute in-memory cache (`PriceCache` in `api/src/price_cache.rs`). Frontend shows "≈ $X.XX/mo" on marketplace offering cards and offering detail pages. Landing page stats show "≈ $X" USD hint for Total Volume.
- **[Marketplace] Active filter chips row** ✅ (2026-02-21) — Dismissible chip row appears above results when any filter is active. Each chip shows the active filter and can be removed individually; "Clear all" resets all. Type multi-select, region/country/city cascade, price, specs, virt, trust, and boolean filters all supported.
- **[Contracts] SSH connection guide on contract detail** ✅ (2026-02-21) — Collapsible "How to Connect" section with 3 tabs (Linux/macOS, Windows Terminal, PuTTY) on the contract detail page, shown only for provisioned/active gateway contracts.
- **[Dashboard] Provider pending requests action banner** ✅ (2026-02-21) — Amber action banner on dashboard home shows count of pending rental requests when provider has offerings with unreviewed requests. Links to `/dashboard/provider/requests`.
- **[Marketplace] Simplified search placeholder** ✅ (2026-02-21) — Changed from cryptic "e.g., type:gpu, price:<=100" to plain "Search by name, description, or type...".
- **[Provider] Per-offering performance stats on earnings page** ✅ (2026-02-21) — New `getProviderOfferingStats` authenticated API function + "Offering Performance" table on `/dashboard/provider/earnings` showing total requests, active, cancelled, expired, and revenue per offering.
- **[Marketplace] Sort by trust score** ✅ (2026-02-21) — 3-button sort group (Price ↑, Price ↓, Trust ↓) replaces the single price toggle. Offerings with highest trust score sort first.
- **[Rentals] "Renew" action on expired/cancelled contracts** ✅ (2026-02-21) — "Renew" button on rentals list and contract detail pages for cancelled/rejected/failed contracts; navigates directly to `/dashboard/marketplace/[offering_id]`.
- **[Offerings] Provider profile sidebar on offering detail** ✅ (2026-02-21) — Sticky sidebar on `/dashboard/marketplace/[id]` shows provider name (linked), TrustBadge, reliability bar, rental count, and "View Provider Profile" button. Fetches `ProviderTrustMetrics` alongside the offering.
- **[Provider] Profile completeness indicator** ✅ (2026-02-21) — Progress bar + checklist on `/dashboard/provider/support` showing 6 completeness items (name, description, website, logo, contacts, help center); score 0–100%; incomplete items link to the relevant settings section.
- **[Dashboard] Tenant empty state with CTAs** ✅ (2026-02-21) — "Deploy your first VM" CTA card in Recent Activity section when tenant has no contracts; "Get Started" card copy updated on dashboard home.
- **[Security] Seed phrase backup reminder** ✅ (2026-02-21) — Dismissible amber banner in dashboard layout for seed-phrase-type identities; persisted via `localStorage`; "Back Up Now" links to `/dashboard/account/security`; mutually exclusive with email verification banner.
- **[Marketplace] "Recently Added" and "Most Trusted" quick-filter badges** ✅ (2026-02-21) — Two pill buttons above marketplace search: "Recently Added" (filters to offerings ≤7 days old, sorts newest-first using new `created_at_ns` DB field) and "Most Trusted" (sorts by trust score descending). Added `created_at_ns: Option<i64>` to `Offering` struct + SELECT queries; sqlx cache updated.
- **[Provider] Offering performance time-series chart** ✅ (2026-02-21) — New authenticated endpoint `GET /api/v1/providers/:pubkey/offering-stats-history?weeks=N` returning `OfferingStatsWeek[]` (week, offering, requests, active, revenue). SVG bar chart added to `/dashboard/provider/earnings` showing weekly requests (indigo) + active contracts (emerald) for the last 8 weeks.
- **[Marketplace] Quick filter presets** ✅ (2026-02-21) — Four preset pill buttons (GPU Servers, Budget <$20/mo, North America, Europe) above marketplace search. Toggle behavior: clicking active preset deactivates; active preset highlighted in theme color.
- **[Contracts] Auto-renewal opt-in** ✅ (2026-02-21) — DB migration adds `auto_renew` flag to contracts; `PUT /api/v1/contracts/:id/auto-renew` endpoint; background `AutoRenewalService` (runs every 6 h, renews contracts expiring within 48 h by creating a new rental request and clearing the flag); toggle UI on contract detail page.
- **[Provider] Earnings fee breakdown** ✅ (2026-02-21) — Revenue Overview restructured to show Gross Revenue / Platform Fee (0 ICP) / Net Earnings rows; Contract Earnings table adds Platform Fee and Net columns.

---

## UX Improvements (Backlog)

