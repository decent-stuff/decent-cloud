# TODO

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

- **Receipt offering/provider names** — Receipts now show actual offering name and provider name from the database, replacing placeholder text.
- **CSV import Hetzner validation** — CSV-imported offerings with `provisioner_type=hetzner` are now validated against the live Hetzner catalog post-import, matching the behavior of create/update offering endpoints.
- **CHATWOOT_FRONTEND_URL safety** — Replaced `.expect()` panics with proper error responses in 3 Chatwoot endpoints and `anyhow::Context` in email processor. Server no longer panics at request time if env var is unset.
- **Rollback error logging** — Marketplace listing rollback (offering deletion) now logs errors instead of silently discarding them via `let _ =`.
- **Email validation deduplication** — Removed inline regex `.unwrap()` in `update_account_email` and replaced with shared `validate_email()` from email-utils crate.
- **Recipe execution log capture** — `execute_post_provision_script` returns `ScriptResult` with stdout/stderr/exit_code. Logs stored in `cloud_resources.recipe_log` (migration 018). Exposed via `GET /contracts/:id/recipe-log`. Frontend shows collapsible "Recipe Output" on contract detail page. Marketplace shows "Recipe" badge on offerings with scripts.
- **Edit Offering page** — Full edit page at `/dashboard/offerings/[id]/edit` for modifying offering details, pricing, and recipe scripts. Linked from offering cards on the My Offerings page.
- **Offering detail page** — Dedicated marketplace detail page at `/dashboard/marketplace/[id]` showing full specs, description, recipe script preview, and rent button. Linked from offering names in the marketplace table.
- **Recipe starter templates** — Create and Edit pages include a "Start from template" selector with ready-made scripts for Docker, Docker Compose, Podman, Node.js, and Caddy static sites.
- **Author notifications on recipe failure** — `CloudProvisioningService` sends Telegram/email notification to offering owner when recipe script fails (exit code != 0). Uses existing notification infra via `rental_notifications::notify_offering_owner_recipe_failure`. Log truncated to 500 chars.
- **API-level recipe filtering** — `GET /offerings?has_recipe=true` server-side filter. Frontend marketplace "Recipes only" toggle triggers API re-fetch instead of client-side filtering.
- **Login button UX consistency** — Google Sign-In button restyled from white Material Design to dark Industrial Luxe theme. Seed phrase "Generate New" button changed from gradient to flat surface with copper-gold border accent. All three login options now share unified dark-surface visual language.

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

---

## Code Quality (from analysis)

### Test Coverage Gaps

The following areas have zero test coverage and handle critical functionality:

- **OpenAPI endpoint handlers** — 16 files (~10K lines) in `api/src/openapi/` have no tests: providers.rs, contracts.rs, cloud.rs, accounts.rs, admin.rs, chatwoot.rs, invoices.rs, offerings.rs, resellers.rs, stats.rs, subscriptions.rs, system.rs, transfers.rs, users.rs, validators.rs, common.rs. (3 files have tests: agents.rs, vat.rs, webhooks.rs.) *(Multi-week: each file needs integration test setup with DB fixtures.)*
- ~~**Chatwoot client** — `chatwoot/client.rs` has 20+ public async functions with no tests.~~ Fixed: 12 HTTP mock tests added using mockito covering both ChatwootPlatformClient (create_user, configure_agent_bot) and ChatwootClient (list_inboxes, find_or_create_inbox, send_message, fetch_conversation_messages, list_articles, list_portals, update_conversation_status). Test constructors (`new_for_test`) added to both client structs.
- ~~**Database reputation/rewards** — `database/reputation.rs` and `database/rewards.rs` lack tests.~~ Fixed: 6 tests for reputation (positive/negative changes, batch, malformed borsh, aging valid/malformed) and 4 tests for rewards (timestamp-in-value, short-value fallback, placeholder zeros, batch).

### Receipt TODO (Completed)

~~`receipts.rs:100` had `// TODO: Get offering name and provider name from database` — receipts showed placeholder text instead of real offering/provider names.~~ Fixed: receipts now fetch offering name via `get_offering_by_id` and provider name via `get_provider_profile`.
