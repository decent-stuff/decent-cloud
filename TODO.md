# TODO

- Login buttons for Google login vs seed phrase (create new + import) are inconsistent — improve consistency, design, and UX *(UI polish — no backend design needed)*

**Specs:**
- [docs/specs/2026-02-14-decent-recipes.md](docs/specs/2026-02-14-decent-recipes.md)
- ~~[docs/specs/2026-02-14-hetzner-provisioner.md](docs/specs/2026-02-14-hetzner-provisioner.md)~~ — Complete (offering validation, provisioning, termination, polling all implemented).
- [docs/specs/2026-02-14-self-provisioning-platform.md](docs/specs/2026-02-14-self-provisioning-platform.md) — Phases 1-4 complete (marketplace listing flow implemented).

---

## Cloud Provisioning

### Known Limitations

- **Multi-instance race** — If two API server instances share the same DB, both provisioning services race on the same resources. The 10-minute lock timeout prevents corruption but can cause delayed provisioning or double-attempt waste. *(Not a prod issue if only one instance runs. Only matters at scale.)*

### Remaining

- **Marketplace billing for listed resources** — Platform fee for marketplace-listed self-provisioned resources. *(Needs product decisions on fee structure.)*
- **Marketplace rental fulfillment for self-provisioned** — When a tenant rents a self-provisioned offering, the contract is created but the VM access handoff (credential sharing) is manual. Needs: automated credential sharing mechanism, stock tracking (one VM = stock of 1). *(Single session once billing decisions are made.)*

### Done

- **Recipe execution log capture** — `execute_post_provision_script` returns `ScriptResult` with stdout/stderr/exit_code. Logs stored in `cloud_resources.recipe_log` (migration 018). Exposed via `GET /contracts/:id/recipe-log`. Frontend shows collapsible "Recipe Output" on contract detail page. Marketplace shows "Recipe" badge on offerings with scripts.

### Remaining (Recipes)

- **Recipe marketplace filters** — Add recipe-specific filtering to marketplace browsing: search by recipe type, author, script keywords. *(Single session: frontend filter UI + existing API query params.)*
- **Recipe script versioning** — Scripts are snapshotted at contract creation. Consider a `recipe_versions` table so authors can update scripts and buyers can upgrade. *(Multi-week: new DB table, migration logic, UI for version management.)*

---

## Provider Provisioning Agent (dc-agent)

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 8 (Hetzner server-side in api-server via `HetznerBackend`). Health check scheduling wired up in dc-agent main loop.

- Phase 9: Docker, DigitalOcean, Vultr provisioners *(Multi-week epic: each provisioner is a separate CloudBackend impl + credentials + testing.)*

---

## Provider Trust & Reliability System

DB tables (`contract_health_checks`), API endpoints, and automated health check scheduling in dc-agent are implemented.

- SLA compliance tracking and provider reputation scoring *(Needs product decisions: what SLA metrics, scoring formula, how reputation affects discovery. Single-session once decisions are made.)*

---

## Notification System

### Paid Notification Tiers
- Define pricing for additional notifications beyond free tier
- Integrate with payment system (Stripe/ICPay)
- Track paid quota separately from free tier

*(Needs product decisions on pricing tiers before implementation. Multi-session: DB schema, Stripe integration, quota tracking.)*

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
