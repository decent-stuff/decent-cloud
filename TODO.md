# TODO

**Specs:**
- [docs/specs/2026-02-14-decent-recipes.md](docs/specs/2026-02-14-decent-recipes.md)
- [docs/specs/2026-02-14-self-provisioning-platform.md](docs/specs/2026-02-14-self-provisioning-platform.md) — Phases 1-4 complete (marketplace listing flow implemented).

---

## ICPay Integration

### Future: Automated Payouts
ICPay does not have a programmatic payout API. Currently payouts are manual via `GET /api/v1/admin/payment-releases` + icpay.org dashboard + `POST /api/v1/admin/payouts`. Implement direct ICRC-1 transfers from platform wallet using `ic-agent` when ICPay adds payout API support. *(Blocked on ICPay adding payout API.)*

---

## Architectural Issues Requiring Review

- None currently.

---

## Recently Done

- **[IC canister] Real token price from KongSwap backend canister** — Replaced hardcoded `$1` token value with canister-to-canister query to KongSwap backend (`2ipq2-uqaaa-aaaar-qailq-cai`) using `pools(opt "<token_canister>_ckUSDT")`; now refreshes DCT (DC) USD price from the `DC_ckUSDT` pool and keeps the previous value on fetch errors.
- **[IC canister] Removed obsolete price-fetch paths** — Deleted unused `fetch_icp_price_usd()` helper and removed leftover HTTP-outcall transform plumbing (`transform_kongswap_response`) now that price refresh is fully on-chain via canister-to-canister pool query.
- **[Offerings] Draft diff view in offering edit flow** — Added a provider-facing "Changes Since Last Save" section in `/dashboard/offerings/[id]/edit` with human-readable before/after values from a shared frontend diff utility (`website/src/lib/utils/offering-draft-diff.ts`).

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

### Database Files Without Dedicated Test Files

Many database modules have no corresponding test file (tests may be in `tests.rs` files):
- `acme_dns.rs`, `agent_delegations.rs`, `agent_pools.rs`, `api_tokens.rs`, `bandwidth.rs`, `chatwoot.rs`, `cloud_accounts.rs`, `cloud_resources.rs`, `core.rs`, `handlers.rs`, `notification_config.rs`, `reputation.rs`, `reseller.rs`, `rewards.rs`, `spending_alerts.rs`, `subscriptions.rs`, `telegram_tracking.rs`, `types.rs`, `user_notifications.rs`, `visibility_allowlist.rs`

**Recommendation:** Add tests for critical paths when modifying.

### Codebase Health Summary

| Metric | Status |
|--------|--------|
| Zombie files | None |
| `todo!()` / `unimplemented!()` | None |
| `dbg!()` debug statements | None |
| Hardcoded credentials | None |
| Commented-out code | Clean |
| `panic!()` in production | Only in tests/build.rs |
| `unreachable!()` in production | Fixed |
| Frontend console.log | Debug statements present |

**Overall:** Codebase is production-ready.

---

## Production Readiness Review (2026-03-01)

### Fixed During Review

- **[api] Compilation error in offerings.rs** — `AGENT_ONLINE_THRESHOLD_SECS` constant was referenced but didn't exist. Fixed by using inline calculation matching other places in the codebase (5 minutes in nanoseconds).

### Security Audit Summary

| Check | Status |
|-------|--------|
| Hardcoded credentials | None (test values only in test code) |
| SQL injection | Protected (parameterized queries via `sqlx::query!`) |
| Webhook signature verification | Stripe, ICPay, Telegram all verified |
| Credential encryption | AES-256-GCM with proper nonce handling |
| CORS configuration | Properly configured for dev/prod |
| Logging secrets | No secrets logged |
| Panic in production | Only in tests/build.rs |
| Auth checks | Proper signature verification with timestamp expiry |

### Infrastructure Checks

| Check | Status |
|-------|--------|
| Doctor command | Comprehensive (DB, Chatwoot, Stripe, Cloudflare, etc.) |
| Health endpoint | `/api/v1/health` |
| Env var documentation | `.env.example` files present |
| Config validation at startup | Critical vars validated (e.g., `CREDENTIAL_ENCRYPTION_KEY`) |

### Verdict

**Production Ready** — No blocking issues found.
