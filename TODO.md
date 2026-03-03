# TODO

**Specs:**
- [docs/specs/2026-02-14-decent-recipes.md](docs/specs/2026-02-14-decent-recipes.md)
- [docs/specs/2026-02-14-self-provisioning-platform.md](docs/specs/2026-02-14-self-provisioning-platform.md) — Phases 1-4 complete (marketplace listing flow implemented).

---

## Recently Done

- **[IC canister] Real token price from KongSwap backend canister** — Replaced hardcoded `$1` token value with canister-to-canister query to KongSwap backend (`2ipq2-uqaaa-aaaar-qailq-cai`) using `pools(opt "<token_canister>_ckUSDT")`; now refreshes DCT (DC) USD price from the `DC_ckUSDT` pool and keeps the previous value on fetch errors.
- **[IC canister] Removed obsolete price-fetch paths** — Deleted unused `fetch_icp_price_usd()` helper and removed leftover HTTP-outcall transform plumbing (`transform_kongswap_response`) now that price refresh is fully on-chain via canister-to-canister pool query.
- **[Offerings] Draft diff view in offering edit flow** — Added a provider-facing "Changes Since Last Save" section in `/dashboard/offerings/[id]/edit` with human-readable before/after values from a shared frontend diff utility (`website/src/lib/utils/offering-draft-diff.ts`).

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

- None currently.

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
- **[Rental] SSH key is required before payment** — SSH section is after payment, with "Generate for me" and private-key download flow.
- **[Marketplace] Currency inconsistency in "Similar Offerings"** — Similar-offering selection now enforces matching currency, preventing mixed-currency cards.

**Remaining:**
- No open critical-path items from the 2026-02-28 rental UX audit.

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

**DONE (2026-03-01) — Usability & Discoverability Audit:**
- **[Marketplace] Save button not discoverable** — Changed from icon-only to labeled button with "Save"/"Saved" text.
- **[Marketplace] Compare feedback unclear** — Added hint "Add 1 more to compare" when only 1 item selected.
- **[Sidebar] Anonymous users don't see what they're missing** — "My Activity" section now shows preview of features with "Sign In" prompt.
- **[Sidebar] "Provider Setup" naming confusing** — Renamed to "Support Account" (accurately reflects the page content).
- **[Saved] No compare action** — Added "Compare Saved" button when 2+ items saved.
- **[First-time user] No guided onboarding** — DONE (single-session): replaced generic welcome modal with 3-step first-login wizard: profile awareness, SSH key check, and activation CTAs (marketplace/provider). Remaining: persist completion across sessions if needed.
- **[E2E] Onboarding wizard progression coverage** — DONE (2026-03-03): added Playwright spec `website/tests/e2e/first-login-onboarding.spec.ts` validating first-login wizard steps and one-session completion persistence.
- **[API] Provider response metrics endpoint mismatch** — DONE (2026-03-03): fixed backend route to serve contract response metrics on canonical path `/api/v1/providers/:pubkey/response-metrics` (removed conflicting `/contract-response-metrics` route path).

**High-Impact Remaining:**

- **[Landing page] Too long, diluted value proposition** — 8 sections before footer. Fix: condense to Hero → Social Proof → Trust System → CTA. Move AI Features and detailed benefits to a separate /features page. *(Multi-session: requires content restructuring.)*

**Medium Impact:**

- **[Offering detail] Too many CTAs** — DONE: secondary actions moved under overflow menu, primary rent CTA remains prominent.

### Usability Audit (2026-03-01) — Remaining Discoverability Issues

**Low Impact:**

- **[Compare] URL shareability/discoverability** — DONE (2026-03-02): Added explicit **Share comparison** action on `/dashboard/marketplace/compare` that copies a canonical URL and shows visible success/error feedback. Canonicalization now enforces positive numeric IDs, deduplication, and compare cap.
- **[E2E] Compare share action coverage** — DONE (2026-03-03): added Playwright spec `website/tests/e2e/compare-share.spec.ts` covering canonical URL copy + success feedback.
- **[Local E2E] Chatwoot noise in Playwright runs** — DONE (2026-03-03): Playwright local webserver now disables frontend Chatwoot widget env vars during E2E and loads API env from `api/.env.local`; this removes external-widget/API misconfiguration noise from local test runs.

- **[Landing page] CTA "Become a Provider" visible to all** — Shown on landing page but most users are not ready to be providers. Consider showing only after user has rented. *(Product decision needed.)*

### VM Rental UX Audit (2026-03-01) — Full Journey Review

**DONE:**
- **[Rental] "Rent" button active on offline providers** — Button now disables when `provider_online === false`.
- **[Rentals] "Total Spent" shows wrong currency** — Now groups by actual contract currency.
- **[Rentals] Contract list shows internal auto-name** — Displays `offering_name` as primary label.
- **[Rentals] SSE (real-time updates) permanently "Disconnected"** — Fixed by adding query param auth fallback for EventSource.
- **[Email verification banner] Interrupts checkout flow** — Banner suppressed on marketplace and rentals paths.

**Remaining:**

- No open full-journey rental issues from the 2026-03-01 audit.

### UI Consistency Audit (2026-03-01) — Auth Surface Duplication + Button Sizing

**Scope audited:** `https://dev.decent-cloud.org/`, `/login`, `/dashboard/marketplace`, `/dashboard/rentals` (desktop).
**Evidence highlights:**
- `/dashboard/marketplace` previously showed **3 visible "Sign In" buttons** at once (heights: `28px`, `36px`, `36px`).
- `/login` auth actions have mismatched heights (`58px` Google CTA, `36px` seed CTA, `20px` back link button).
- Marketplace table row actions previously mixed sizes in one row (`Rent 28px`, `Save 26px`, `+ Compare 42px`, "More details" icon tap target `12px`).

**DONE (2026-03-02):**

- **[Auth] Too many unauthenticated entry points on dashboard pages** — Fixed by removing sidebar Sign In CTAs for anonymous users; global banner/route auth card remain the only primary auth actions. Validation script: `scripts/poc/assert_single_auth_cta.sh` (now passes with 1 visible CTA on `/dashboard/marketplace`).
- **[Auth] Duplicate login UX implementations across many routes** — Verified complete: dashboard route-level auth gates are standardized on `AuthRequiredCard.svelte` (no remaining custom `Login Required` blocks under `website/src/routes/dashboard`).
- **[Marketplace] Quick filter pills inconsistent** — Fixed by introducing semantic quick-pill variants (`quick-pill-filter` vs `quick-pill-preset`) via shared builder in `website/src/lib/utils/marketplace-ui.ts`.
- **[Marketplace table] Row actions are visually unbalanced and hard to scan** — Fixed by introducing shared row-action class builder (`buildRowActionButtonClass`) and standardized compact control height (`h-7`) for Rent/Save/Compare across desktop and mobile marketplace cards.

- **[Auth page hierarchy] `/login` secondary actions are too de-emphasized** — Fixed. Seed login now uses an explicit secondary CTA style and back action uses a consistent tertiary CTA style via shared auth CTA class builder (`website/src/lib/utils/auth-cta.ts`), applied in `AuthFlow.svelte`, `GoogleSignInButton.svelte`, and `/login`.
- **[Buttons] Design-system rollout (phase 1: auth surface)** — Added shared control-size token `btn-control-md` in `website/src/app.css` and applied it to auth entry points (`AuthFlow`, `GoogleSignInButton`, `AuthPromptBanner`, `AuthRequiredCard`, `/login`). Added PoC script `scripts/poc/probe_auth_button_heights.sh` and unit test `website/src/lib/utils/auth-cta.test.ts`.
- **[Buttons] Complete design-system enforcement on dashboard pages** — Fixed for key routes in this phase: `website/src/routes/dashboard/marketplace/+page.svelte` (active filter and sort CTAs) and `website/src/routes/dashboard/rentals/+page.svelte` (status tab CTAs and peer dashboard actions). Added shared dashboard CTA class builder (`website/src/lib/utils/dashboard-cta.ts`), PoC script (`scripts/poc/assert_dashboard_cta_height_consistency.sh`), and test coverage (`website/src/lib/utils/dashboard-cta.test.ts`). **Follow-up dependency:** rerun `scripts/poc/assert_dashboard_cta_height_consistency.sh` on `https://dev.decent-cloud.org` after next website deploy.

### Backlog

- **[Cloud] Stock tracking for self-provisioned resources** — When a cloud resource is listed on the marketplace, multiple tenants could theoretically rent the same VM. Needs: `stock` field on cloud_resources, 1-to-1 rental enforcement, automated credential sharing when contract is accepted. *(Blocked: billing decisions first.)*

- **[Offerings] Click-through conversion funnel** — Add views → rentals funnel per offering. *(Overall conversion rate per offering is already shown on Analytics page.)*

- **[Security] Two-factor authentication (TOTP)** — TOTP-based 2FA for accounts using email/password. *(Multi-session.)*

- **[Global] Dark/light mode toggle** — Theme switcher in dashboard header. *(Multi-session.)*

- **[Provider] Pricing elasticity insights** — Extend provider analytics with price sensitivity guidance. *(Multi-session.)*

- **[Marketplace] Recommended for you** — Personalized recommendations section in marketplace. *(Multi-session.)*

- **[Provider] Provider public profile and reputation deep-dive** — Tenants cannot view a provider's historical trust score trend, feedback breakdown, or SLA violation history. *(Multi-session.)*

- **[Tenant] Saved offerings price-change alerts** — Tenants receive no notification when a saved offering changes price or goes out of stock. *(Multi-session.)*

---

## Code Quality Audit (2026-03-01)

### Completed

- **[api-cli] Replace `unreachable!()` with proper error handling** — DONE.
- **[dc-agent] Replace `unreachable!()` with proper match handling** — DONE.
- **[api/database] Remove `unreachable!()` from ledger handlers** — DONE (2026-03-01). Fixed `handlers.rs:119` to use proper error handling.
- **[api/auth] Remove unused `authenticate_agent_from_request`** — DONE (2026-03-01). Superseded by `authenticate_provider_or_agent_from_request`.
- **[api/crypto] Mark `ServerEncryptionKey::from_bytes` as test-only** — DONE (2026-03-01).

### Clippy Warnings Analysis (2026-03-01)

The remaining 16 clippy warnings are **false positives** due to Rust's separate compilation:

| Warning | Reason |
|---------|--------|
| `list_admins`, `create_or_update_external_provider`, `count_offerings`, `import_seeded_offerings_csv`, `get_example_offerings`, `is_offering_saved` | Used in `api-cli` binary (different target) |
| `pool`, `update_cloud_resource_status` | Used in tests |
| `decrypt_credentials`, `decrypt_credentials_with_aad`, `ed25519_secret_to_x25519`, `from_json` | Prepared for E2EE credential feature |
| `upsert_spending_alert`, `delete_spending_alert` | Prepared for spending alerts feature |
| `CreateCloudAccountInput`, `CloudAccountWithCatalog`, `CreateCloudResourceInput` | Prepared for self-provisioning API |
| `user_pubkey` field | Used in serialization |

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

**Production code:**
- `website/src/routes/dashboard/rentals/[contract_id]/+page.svelte` — `console.debug` for expected error cases (no usage data, no credentials)
- `website/src/routes/dashboard/provider/sla/+page.svelte` — `console.debug` for missing SLA config (expected)
- `website/src/lib/stores/auth.ts` — `console.error` for authentication failures (appropriate)

**Test fixtures (acceptable):**
- `website/tests/e2e/fixtures/stripe-mock.ts` — Mock Stripe.js logging for E2E tests
- `website/tests/e2e/fixtures/auth-helpers.ts` — Browser console logging for E2E tests

**Recommendation:** Production `console.debug` statements are for expected error cases and are acceptable. Test fixture logging is intentional.

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

---

## Production Readiness Review (2026-03-01)

### Fixed During Review

- **[api] Compilation error in offerings.rs** — `AGENT_ONLINE_THRESHOLD_SECS` constant was referenced but didn't exist. Fixed by using inline calculation matching other places in the codebase (5 minutes in nanoseconds).

### Remaining (Low Priority)

- **[api-server] Graceful shutdown** — Background tasks (cleanup, email processor, payment release, etc.) are aborted on server shutdown rather than gracefully terminated. For production, consider implementing signal handling (SIGTERM/SIGINT) with proper task cancellation via `CancellationToken` or channels. *(Low priority: current behavior is safe, just not graceful. Multi-session to implement properly.)*

- **[api-server] General rate limiting** — Only email verification resend has rate limiting (60-second window). Public endpoints lack general rate limiting. Consider adding rate limiting middleware for unauthenticated endpoints. *(Multi-session: requires design decisions on limits per endpoint.)*

- **[dev] SQLX query cache** — Development environment has stale query cache. Run `cargo sqlx prepare` to regenerate. *(Not a production issue.)*

### Security Audit Summary

| Check | Status |
|-------|--------|
| Hardcoded credentials | ✅ None (test values only in test code) |
| SQL injection | ✅ Protected (parameterized queries via `sqlx::query!`) |
| Webhook signature verification | ✅ Stripe, ICPay, Telegram all verified |
| Credential encryption | ✅ AES-256-GCM with proper nonce handling |
| CORS configuration | ✅ Properly configured for dev/prod |
| Logging secrets | ✅ No secrets logged |
| Panic in production | ✅ Only in tests/build.rs |
| Auth checks | ✅ Proper signature verification with timestamp expiry |

### Infrastructure Checks

| Check | Status |
|-------|--------|
| Doctor command | ✅ Comprehensive (DB, Chatwoot, Stripe, Cloudflare, etc.) |
| Health endpoint | ✅ `/api/v1/health` |
| Env var documentation | ✅ `.env.example` files present |
| Config validation at startup | ✅ Critical vars validated (e.g., `CREDENTIAL_ENCRYPTION_KEY`) |

### Verdict

**Production Ready** — No blocking issues found. The remaining items are enhancements for robustness at scale, not blockers for initial production deployment.
