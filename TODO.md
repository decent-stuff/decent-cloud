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

- **[Provider] Offering creation wizard** — `/dashboard/offerings/create` is a long single-page form. Break into wizard steps: (1) Basic Info + Pricing, (2) Specs + Location, (3) Recipe/Script, (4) Preview. Reduces cognitive load for new providers. *(Single-session once decided.)*

- **[Marketplace] Pre-rental contact flow** — Let tenants ask a question before creating a contract. "Ask Provider" button opens Chatwoot pre-auth conversation. Reduces failed rentals from unmet expectations. *(Single-session: Chatwoot conversation creation + UI.)*

- **[Offerings] Marketplace preview before publishing** — Side panel on offering create/edit showing exactly how the card will look in the marketplace. Prevents providers from publishing with broken descriptions. *(Single-session: reuse existing offering card component.)*

- **[Offerings] Pricing guidance** — On offering creation form, show "Median price for this spec in this region: X ICP/mo" pulled from marketplace data. Helps new providers price competitively. *(Single-session: new API endpoint aggregating offering stats.)*

- **[Offerings] Per-offering analytics** — Impression count, click-through rate, conversion rate (views → rentals) per offering on the offerings list. Helps providers optimize. *(Multi-session: needs impression tracking in DB, analytics aggregation endpoint.)*
  - Dependency: Requires impression/view event logging (new DB table `offering_views`).

- **[Offerings] Save as Draft** — Offering creation allows saving incomplete form without publishing. Draft offerings visible only to provider, not in marketplace. *(Single-session: add `is_draft` boolean on offerings table + DB migration + filter from public endpoints.)*

- **[Account] Account deletion / Danger Zone** — Secure account deletion with data wipe confirmation (type "DELETE" prompt). Required for GDPR compliance. *(Single-session: new DELETE endpoint + modal UI.)*

- **[Security] Two-factor authentication (TOTP)** — TOTP-based 2FA for accounts using email/password (not seed-phrase accounts which already have key-based auth). *(Multi-session: TOTP secret generation, QR code display, verification middleware.)*

- **[Global] Dark/light mode toggle** — Theme switcher in dashboard header. Persist in localStorage. *(Single-session: Tailwind dark: classes + toggle component. Note: requires adding dark: classes across all components.)*

- **[Tenant] Budget alerts / spending cap** — Set monthly ICP spending limit; receive notification when approaching or exceeding cap. *(Single-session: new `spending_alert_config` table + background check against monthly contract costs.)*
