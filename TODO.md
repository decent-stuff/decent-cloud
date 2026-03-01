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
- **User feedback system** — DONE: `contract_feedback` table, `POST /contracts/{id}/feedback` (requester-only, terminal state, once per contract), `GET /contracts/{id}/feedback`, binary yes/no UI on rental detail page, aggregated in `offering-satisfaction-stats`.
- **External benchmarking integration** — Cross-reference provider claims with https://serververify.com/ and https://www.vpsbenchmarks.com/ for independent verification. Price comparison vs market average. *(Multi-session: scraping/API integration + trust score formula.)*

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

### Hardcoded Token Value ($1 USD) in IC canister — HIGH PRIORITY

**Issue:** The IC canister's `refresh_last_token_value_usd_e6()` always returns `1_000_000` ($1 USD). The api-server now fetches real ICP/USD price from CoinGecko for UI display (cached 5 min, `GET /api/v1/prices/icp`), but the on-chain canister price remains hardcoded.
**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`
**Fix:** Use IC HTTP outcalls to fetch from **KongSwap** (backend canister `2ipq2-uqaaa-aaaar-qailq-cai` on ICP mainnet). KongSwap exposes a price query for ICP/USD. Use `ic_cdk::api::management_canister::http_request` for HTTP outcalls or canister-to-canister query. *(Single-session once implemented — ICP HTTP outcall pattern is well-documented.)*

---

## UX Improvements

### VM Rental UX Audit (2026-02-28) — Critical Path Issues

**DONE:**
- **[Rental] Payment method default is ICPay (crypto) for USD offerings** — Stripe is now the default for USD/EUR offerings.
- **[Post-rental] Success message auto-dismisses in 5 seconds** — Checkout success page redirects with welcome banner.
- **[Marketplace] Provider shown as raw hex pubkey** — Shows `@username` when available.
- **[Auth] Seed-phrase-only sign-up is unfamiliar** — Google sign-in is now primary.
- **[Marketplace] Demo offerings hidden by default** — Checkbox defaults to unchecked.
- **[Offering detail] No "time to provision" estimate** — Shows "Setup Time" from trust metrics.
- **[Rental dialog] Price summary is at the bottom** — Price summary now at top of dialog.
- **[Breadcrumb] "Dashboard" link opens wrong view** — Authenticated users now see `/dashboard/rentals`.
- **[Testing tooling] All commands implemented** — `seed-contracts`, `--viewport mobile`, `tour`, `seed-edge-cases` all available.

**Remaining:**

- **[Rental] SSH key is required before payment** — Most non-technical users don't have an SSH key. Fix: move the SSH key section below the payment section, or allow "generate for me" to create a keypair and download the private key automatically. *(Single-session.)*

- **[Marketplace] Currency inconsistency in "Similar Offerings"** — Similar offerings mix `ICP/mo` (seeded demo data) with `USD/mo` (real offerings) without currency normalization. *(Single-session.)*

### UI/UX Review (2026-03-01) — Radical Simplification

**DONE (2026-03-01):**
- **[Sidebar] Removed "My Cloud" section** — Confusing for most users.
- **[Sidebar] Removed "Transfers" and "Invoices" from My Activity** — Obscure features.
- **[Sidebar] Simplified Provider section** — Shows only "Provider Setup" and "My Offerings" until onboarding complete.
- **[Auth] Google sign-in now prominent** — Shows Google sign-in button first.
- **[Dashboard] Quick Actions are role-aware** — Cards shown based on user role.
- **[Marketplace] Filter overload** — Only Type, Region, Price visible by default; "More filters" toggle for advanced.
- **[Marketplace] Badge clutter** — Consolidated into single status chip with tooltip.
- **[Sidebar] "Discover" section label** — Renamed to "Browse".

**High-Impact Remaining:**

- **[Landing page] Too long, diluted value proposition** — 8 sections before footer. Fix: condense to Hero → Social Proof → Trust System → CTA. Move AI Features and detailed benefits to a separate /features page. *(Multi-session: requires content restructuring.)*

- **[First-time user] No guided onboarding** — New authenticated users land on a dashboard with many options. Fix: add a 3-step onboarding wizard on first login: 1) Complete profile (username, email), 2) Add SSH key, 3) Browse marketplace or become provider. *(Multi-session: wizard component + localStorage flag.)*

**Medium Impact:**

- **[Marketplace] Quick filter pills inconsistent** — "Recently Added" and "Most Trusted" are filters, but "GPU Servers", "Budget", "North America", "Europe" are presets. Fix: visually distinguish filter pills (toggle behavior) from preset pills (exclusive selection). *(Single-session: add visual indicator.)*

- **[Offering detail] Too many CTAs** — "Copy link", "Save", "Ask Provider", "Rent this offering" all compete. Fix: primary CTA "Rent" should be prominent; move secondary actions to a "..." menu. *(Single-session: consolidate secondary actions.)*

### VM Rental UX Audit (2026-03-01) — Full Journey Review

**DONE:**
- **[Rental] "Rent" button active on offline providers** — Button now disables when `provider_online === false`.
- **[Rentals] "Total Spent" shows wrong currency** — Now groups by actual contract currency.
- **[Rentals] Contract list shows internal auto-name** — Displays `offering_name` as primary label.
- **[Rentals] SSE (real-time updates) permanently "Disconnected"** — Fixed by adding query param auth fallback for EventSource.
- **[Email verification banner] Interrupts checkout flow** — Banner suppressed on marketplace and rentals paths.

**Remaining:**

- **[Subscription] Rental limit blocks paying customers** — Free tier allows only 1 active rental. This limit makes no sense when users are paying the provider per-rental. Additionally: since account creation requires no KYC (just a seed phrase), this is a Sybil attack vector. **Requires product decision**: what should the free tier allow? If the limit is intended, implement account identity verification (email confirmation at minimum). *(Architectural decision required — do NOT implement without user sign-off.)*

- **[Console] Persistent 404 errors (non-Chatwoot)** — Every page load generates 5–10 `Failed to load resource: 404` errors. Diagnose and fix. *(Requires investigation: intercept network requests to identify failing URLs.)*

- **[Password Resets SSE] Agent auth needs separate fix** — The password-resets page SSE is disabled because it uses agent authentication (`X-Agent-Pubkey`) which requires a different fix. Frontend signs with user identity but backend expects agent identity. *(Single-session: add agent auth query param support.)*

### Backlog

- **[Cloud] Stock tracking for self-provisioned resources** — When a cloud resource is listed on the marketplace, multiple tenants could theoretically rent the same VM. Needs: `stock` field on cloud_resources, 1-to-1 rental enforcement, automated credential sharing when contract is accepted. *(Blocked: billing decisions first.)*

- **[Offerings] Per-offering analytics** — DONE. **Remaining:** Click-through rate (views → rentals) conversion funnel. *(Note: overall conversion rate per offering is shown on the Analytics page.)*

- **[Marketplace] Offering comparison page** — DONE: `/dashboard/marketplace/compare?ids=1,2,3`.

- **[Rentals] Contract lifecycle timing** — DONE: Expected time estimates and overdue warning.

- **[Dashboard] Tenant spending insights** — DONE: Monthly spending widget.

- **[Security] Two-factor authentication (TOTP)** — TOTP-based 2FA for accounts using email/password. *(Multi-session.)*

- **[Global] Dark/light mode toggle** — Theme switcher in dashboard header. *(Multi-session.)*

- **[Provider] Provider performance analytics** — DONE. **Remaining:** Pricing elasticity insights. *(Multi-session.)*

- **[Provider] Request filtering and bulk actions** — DONE: Full implementation including auto-accept.

- **[Tenant] SSH key onboarding guidance** — DONE: Platform-specific tabbed SSH key generation guide.

- **[Marketplace] Trending and new providers sections** — DONE. **Remaining:** "Recommended for you" personalized section. *(Multi-session.)*

- **[Provider] Provider public profile and reputation deep-dive** — Tenants cannot view a provider's historical trust score trend, feedback breakdown, or SLA violation history. *(Multi-session.)*

- **[Offerings] Draft offerings scheduling** — DONE. **Remaining:** "What changed since last save" diff view. *(Multi-session.)*

- **[Tenant] Saved offerings price-change alerts** — Tenants receive no notification when a saved offering changes price or goes out of stock. *(Multi-session.)*

---

## Code Quality Audit (2026-03-01)

### Completed

- **[api-cli] Replace `unreachable!()` with proper error handling** — DONE.
- **[dc-agent] Replace `unreachable!()` with proper match handling** — DONE.
- **[api/database] Remove `unreachable!()` from ledger handlers** — DONE.

### Code Cleanup Needed (Low Priority)

- **[dc-agent] Remove deprecated `traefik_dynamic_dir` field** — `dc-agent/src/config.rs:46-49`. Can be removed after migration period.

### TODOs in Source Code (Track but Not Blocking)

- `api/src/cleanup_service.rs:190` — TODO about Stripe subscription billing integration (tracked in Notification System section)
- `ic-canister/src/canister_backend/generic.rs:362` — TODO about ledger iteration optimization (performance)
- `ic-canister/src/canister_endpoints/generic_anonymous.rs:84` — TODO for CF sync implementation (feature)
- `cli/src/keygen.rs:40` — TODO: Add more languages (nice-to-have)
- `ledger-map/src/ledger_map.rs:19` — TODO: Make configurable (optimization)

### Prepared/Unused Code (Low Priority)

- `api/src/database/reseller.rs` — `#[allow(dead_code)]` structs "Prepared for reseller API feature"
- `api/src/icpay_client.rs` — `#[allow(dead_code)]` structs "Prepared for payment verification feature"

### Large Files (Refactoring Candidates)

- `api/src/openapi/providers.rs` — 5670 lines
- `api/src/bin/api-cli.rs` — 3341 lines
- `api/src/database/contracts.rs` — 3361 lines

*(Split when adding significant new functionality.)*

### Frontend Debug Statements (Low Priority)

Multiple `console.log`/`console.debug` statements in frontend. Remove or convert to proper logging before production:
- `website/src/routes/login/+page.svelte:25`
- `website/src/lib/stores/auth.ts:237`
- `website/src/lib/components/AuthDialog.svelte:15`
- `website/src/lib/components/RentalRequestDialog.svelte:184,199`
- `website/src/lib/components/provider/AgentTable.svelte:17`
- `website/src/routes/dashboard/provider/requests/+page.svelte:171,183`
- `website/src/lib/components/OfferingsEditor.svelte:285-291`

### Database Files Without Dedicated Test Files

Many database modules have no corresponding test file (tests may be in `tests.rs` files):
- `acme_dns.rs`, `agent_delegations.rs`, `agent_pools.rs`, `api_tokens.rs`, `bandwidth.rs`, `chatwoot.rs`, `cloud_accounts.rs`, `cloud_resources.rs`, `core.rs`, `handlers.rs`, `notification_config.rs`, `reputation.rs`, `reseller.rs`, `rewards.rs`, `spending_alerts.rs`, `subscriptions.rs`, `telegram_tracking.rs`, `types.rs`, `user_notifications.rs`, `visibility_allowlist.rs`

**Recommendation:** Add tests for critical paths when modifying.

### Codebase Health Summary

| Metric | Status |
|--------|--------|
| Zombie files | ✅ None |
| `todo!()` / `unimplemented!()` | ✅ None |
| `dbg!()` debug statements | ✅ None |
| Hardcoded credentials | ✅ None |
| Commented-out code | ✅ Clean |
| `panic!()` in production | ✅ Only in tests/build.rs |
| `unreachable!()` in production | ✅ Fixed |
| Frontend console.log | ⚠️ Debug statements present |

**Overall:** Codebase is production-ready.
