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

- **[Rental flow] Stripe path SSH key save** — ✅ Fixed: SSH key is now stored in localStorage before Stripe redirect and saved to profile in the rentals page on return.

- **[Contract lifecycle] No email notification link to rentals page** — When a user receives an email notifying them their resource is ready, there is no direct link to the contract in the rentals page. The email should include a deep-link URL with `?contract=<id>` so the user lands directly on the correct contract card. *(Low effort: update email template to include `/dashboard/rentals?contract=<id>`.)*

- **[Marketplace UX] No "become a provider" CTA on marketplace** — When a user with no offerings browses the marketplace, they see only rentable offerings. A secondary CTA (e.g., a banner at the top or a sidebar card: "Have infrastructure to share? Become a provider →") would drive provider acquisition. *(Low effort: add a dismissible banner to the marketplace page for authenticated users with zero offerings.)*

- **[Rentals page] SSH key display is truncated** — The SSH key on the rental card is truncated with `truncateHash()`, but SSH keys start with the same prefix (`ssh-ed25519 AAAA...`) so all truncated previews look identical. Consider showing the last 20 characters of the key instead, or the key comment (email part after the last space). *(Low effort: change truncation display logic.)*

### Provider Experience

- **[Provider onboarding] Direct navigation to gated routes** — ✅ Fixed: All gated provider pages (My Offerings, Earnings, Rental Requests, Agents, Reseller) now show an inline "Provider Setup Required" banner when onboarding is incomplete, with a direct link to the setup page.

- **[Provider earnings] No chart for revenue over time** — The earnings page shows totals but no time-series chart. Providers can't see if they're growing or declining. Adding a simple revenue-by-month bar chart would dramatically improve the earnings page. *(Medium effort: requires adding a time-bucketed earnings endpoint and a chart component.)*

- **[Provider agents] Setup token UX is opaque** — The agent setup flow requires generating a setup token and running `dc-agent setup token` manually. There is no status indicator showing whether a dc-agent has successfully connected to the API. The Agents page should show: connected agents, last-seen timestamp, and health status for each pool. *(Medium effort: `last_seen_at` is tracked in DB; surface it in the agent pool table.)*

- **[Offerings] Pool assignment UX** — When a provider creates an offering but has no agent pools, the offering shows a warning ("No pool") but no inline "Create a pool" CTA. The warning banner links to Agents, but a more direct flow would be: clicking "No pool" opens the pool creation dialog inline. *(Low effort: add a link or mini-dialog from the offering card's "No pool" indicator.)*

- **[Dashboard] Quick Actions link to "My Offerings" is ungated** — The dashboard home page's Quick Actions grid links authenticated users to `/dashboard/offerings` regardless of onboarding status. For new providers this shows the "Provider Setup Required" banner, but the link label should instead say "Provider Setup" and go to `/dashboard/provider/support` when onboarding is not complete. *(Medium effort: dashboard home page needs to check onboarding status to conditionally render the link.)*
