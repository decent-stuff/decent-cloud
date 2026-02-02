# TODO

## HIGH PRIORITY: Self-Hosted Resource Management

**Goal:** Transform platform from pure marketplace into unified resource management + marketplace. Users can manage own infrastructure, rent from others, and deploy value-add services on top.

### Vision
```
┌─────────────────────────────────────────────────────────────────┐
│                    Decent Cloud Platform                        │
├─────────────────────────────────────────────────────────────────┤
│  Layer 3: Value-Add Services                                    │
│  - Coding agents (OpenCode, Aider)                              │
│  - AI tools (OpenClaw, local LLMs)                              │
│  - Custom applications deployed on rented/owned resources       │
├─────────────────────────────────────────────────────────────────┤
│  Layer 2: Resource Marketplace                                  │
│  - Rent from others (paid or free)                              │
│  - Offer own resources to others                                │
├─────────────────────────────────────────────────────────────────┤
│  Layer 1: Own Infrastructure Management                         │
│  - Register own servers/agents                                  │
│  - Private by default (only visible to owner)                   │
│  - Self-rental is FREE (no payments)                            │
└─────────────────────────────────────────────────────────────────┘
```

### Key Features
1. **Visibility Controls** (per offering/resource):
   - `private` - Only visible to owner (default)
   - `shared` - Visible to specific users/accounts
   - `public` - Listed in marketplace

2. **Self-Rental = Free**: Same pubkey as requester and provider → no payment required

3. **Flexible Pricing**: Offerings to others can be free or paid

4. **Value-Add Deployment**: Derived services are independent (no relationship tracking)

### Design Decisions
- **Infrastructure**: Users run `dc-agent` on their own servers (same as providers)
- **Identity**: Same pubkey = same person (simple verification)
- **Derived services**: Completely independent offerings, no parent-child tracking
- **Abuse prevention**: None for now (simplicity over complexity)

### Implementation Phases

**Phase 1: Visibility & Self-Rental** ✅ COMPLETE (2026-02-02)
- [x] `visibility` field already exists in offerings table
- [x] Filter offerings API by visibility + requester pubkey:
  - `GET /offerings/:id` - returns 404 for non-public unless requester is owner
  - `GET /providers/:pubkey/offerings` - returns only public offerings (public API)
  - `GET /provider/my-offerings` - returns ALL offerings for authenticated provider
- [x] `POST /contracts` enforces visibility (can only rent public OR own offerings)
- [x] Detect self-rental (requester_pubkey == provider_pubkey) → skip payment
  - Payment amount set to 0 for self-rental
  - Payment status set to "succeeded" immediately
  - Stripe checkout skipped, auto-accept triggered
- [x] UI: "My Resources" section on dashboard showing all offerings with "Rent Free" button
- [x] `OptionalApiAuth` extractor added for endpoints that need optional authentication

**Phase 2: Shared Visibility** ✅ COMPLETE (2026-02-02)
- [x] Add `visibility_allowlist` table (offering_id, allowed_pubkey)
  - Migration: `006_visibility_allowlist.sql`
  - Database functions: `add_to_allowlist`, `remove_from_allowlist`, `get_allowlist`, `is_in_allowlist`, `can_access_offering`
- [x] Filter shared offerings to allowlisted users only
  - Updated `GET /offerings/:id` visibility check to include allowlist
  - Updated `POST /contracts` visibility check to include allowlist
- [x] API endpoints for allowlist management:
  - `GET /providers/:pubkey/offerings/:id/allowlist` - List allowlist entries
  - `POST /providers/:pubkey/offerings/:id/allowlist` - Add to allowlist
  - `DELETE /providers/:pubkey/offerings/:id/allowlist/:allowed_pubkey` - Remove from allowlist
- [x] UI: Manage who can see/rent your shared offerings
  - Visibility toggle now cycles: public → shared → private
  - QuickEditOfferingDialog shows allowlist management when visibility is "shared"
  - Add/remove users by public key

**Phase 3: Templated Deployments**
- [ ] Add `post_provision_script` or `cloud_init_config` field to offerings
- [ ] dc-agent executes deployment script after VM provisioning
- [ ] Enable "rent base VM + auto-deploy service" pattern

---

## API-CLI Testing Framework

**Goal:** Enable automated E2E testing of the full VM provisioning flow without the website frontend, for CI/CD pipelines and AI agent-assisted testing.

### Completed ✓
- Identity management (generate, import, list, show, delete) with storage at `~/.dc-test-keys/`
- Ed25519ph signing client matching API authentication requirements
- Account operations (create, get, update-email, add-ssh-key, list-ssh-keys)
- Contract operations (list-offerings, create, get, wait, list, cancel)
- Offering/Provider listing commands
- Health checks for all external services (API, Stripe, Telegram, Cloudflare, MailChannels)
- Gateway connectivity testing structure (ssh, tcp, contract)
- E2E test scaffolding (provision, lifecycle, all)
- `--skip-payment` flag with `set_payment_status_for_testing()` DB function

### Remaining Work
- [x] **Fix api-server build** - oauth2 crate compatibility issues ✓ (98d219c)
- [ ] **Test contract create/wait/cancel** - Requires running provider with dc-agent
- [ ] **Test gateway commands** - Requires active contracts with assigned gateways
- [ ] **Test DNS commands** - Requires Cloudflare credentials in environment
- [ ] **Full E2E test run** - Provision → verify SSH → cleanup cycle

### Usage
```bash
# Generate test identity
api-cli identity generate --name test1

# Create account
api-cli --api-url https://api.decent-cloud.org account create --identity test1 --username myuser --email test@example.com

# List offerings and create contract (with skip-payment for testing)
api-cli contract list-offerings --limit 10
api-cli contract create --identity test1 --offering-id 1 --ssh-pubkey "ssh-ed25519 ..." --skip-payment

# Wait for provisioning and test gateway
api-cli contract wait <contract-id> --identity test1 --state provisioned --timeout 300
api-cli gateway contract <contract-id> --identity test1
```

---

## HIGH PRIORITY: Streamlined Rental UX (Pick → Pay → Use)

**Priority:** CRITICAL - Competitive feature parity with modern cloud providers
**Goal:** Eliminate provider approval bottleneck; enable instant provisioning like AWS/GCP/Azure

### Current Flow (Too Slow)
```
User picks offering → Pays → Waits for provider approval → Waits for provisioning → Gets access
```

### Target Flow (Modern Cloud UX)
```
User picks offering → Pays → Instantly provisioned → Gets SSH access
```

### Completed ✓
- **Auto-Accept Mode:** Provider setting to skip approval step, contracts go directly to provisioning
- **Instant SSH Key Injection:** SSH key required at rental, validated, passed to dc-agent, email notification on ready
- **Pre-configured Offering Templates:** Providers can specify VM templates per offering via `template_name` field
  - Database migration adds `template_name` column to offerings
  - DC-agent reads template from `provisioner_config` JSON (auto-generated from template_name)
  - Template override hierarchy: contract → offering → agent default
  - CSV import/export supports template configuration

### Remaining Work

**Provisioning Time Target:** < 60 seconds from payment to SSH access
- Auto-accept + template selection now enable this target
- Actual timing depends on VM clone speed and network configuration

---

## Provider Provisioning Agent

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete through Phase 5.5 (Agent Pools)
**Priority:** HIGH - Critical for automated cloud platform

Software that providers run to automatically provision services when contracts are accepted.

### Completed ✓ (Phases 1-5.5)
- Agent skeleton, TOML config, Ed25519 auth, Proxmox provisioner
- Setup wizard (`dc-agent setup proxmox`), test provisioning
- Delegated agent keys with one-liner UX
- Agent pools with location-based routing and race condition prevention
- Dashboard online/offline badges, pool management UI

### Phase 6: Health & Reputation ✓
- [x] **6.1 API:** `POST /contracts/{id}/health` endpoint + `contract_health_checks` table ✓ (b3b714a)
- [x] **6.2 API:** Uptime calculation per provider ✓ (`GET /providers/{pubkey}/health-summary` endpoint)
- [x] **6.3 Dashboard:** Show uptime percentage on provider profile ✓ (TrustDashboard + reputation page)

#### Phase 7: Credential Encryption
- [ ] Encrypt VM passwords with requester's pubkey (Ed25519→X25519)
- [ ] Frontend decryption with user's private key
- [ ] Auto-delete credentials after 7 days

### Future Phases
- Phase 8: Hetzner Cloud provisioner
- Phase 9: Docker, DigitalOcean, Vultr provisioners

---

## Provider Trust & Reliability System

### External Benchmarking Integration
Integrate with or scrape external sources for additional trust signals:
- https://serververify.com/ - Server verification and uptime data
- https://www.vpsbenchmarks.com/ - VPS performance benchmarks
- Price comparison vs market average ("15% below market" or "30% above")
- Cross-reference provider claims with independent verification

### Service Quality Verification
- Automated health checks on provisioned services
- Uptime monitoring and SLA compliance tracking
- "99.2% uptime in last 30 days" with proof
- Requires: Infrastructure monitoring agents or integration with external monitors

### User Feedback System (Structured, Not Reviews)
- Post-contract structured survey: "Did service match description?" Y/N
- "Would you rent from this provider again?" Y/N
- Binary signals harder to game than star ratings
- Requires: Post-contract feedback workflow

---

## Contract-Specific Customer-Provider Messaging

**Status:** Backend infrastructure exists (SLA tracking, message events), but no usable UI.
**Priority:** MEDIUM - Improves customer experience and enables SLA enforcement

### Motivation
When customers rent services, they need a way to communicate with providers about that specific contract (questions, issues, configuration). Currently:
- Backend can create Chatwoot conversations with `contract_id` metadata
- Message events are tracked for SLA response time calculation
- Provider response metrics are exposed via API (`/providers/:pubkey/metrics`)
- BUT: No frontend UI for customers to access these conversations

### What Exists (Backend)
- `chatwoot_message_events` table tracks messages per contract
- SLA breach detection in `email_processor.rs`
- Provider response time metrics calculation
- Webhook handler extracts `contract_id` from conversation custom_attributes

### What's Missing (Frontend)
- "Contact Provider" button on rentals page that opens contract-specific chat
- Provider inbox/dashboard showing contract conversations
- Link between Chatwoot conversation and contract in UI

### Implementation Approach
1. Add "Contact Provider" button to `/dashboard/rentals` for each contract
2. Button opens Chatwoot widget pre-configured with contract context
3. Or: Embed mini-chat directly in contract details view
4. Provider sees conversations tagged by contract in their Chatwoot inbox

### Alternative: Remove SLA Tracking
If contract-specific messaging isn't needed, the SLA tracking infrastructure (`chatwoot_message_events`, response metrics) could be removed to simplify the codebase.

---

## Usage-Based Billing for Rentals - Backend ✅ COMPLETE

**Priority:** MEDIUM - Frontend UI remaining
**Status:** Backend complete, frontend pending

### What's Implemented ✅

**Database Schema:**
- `contract_usage` table with units tracking, billing periods, Stripe integration
- `contract_usage_events` table for event-level tracking
- `provider_offerings` has: `billing_unit`, `pricing_model`, `price_per_unit`, `included_units`, `overage_price_per_unit`, `stripe_metered_price_id`

**API Endpoints:**
- `POST /contracts/{id}/usage` - Record usage events (event_type, units_delta, heartbeat_at, source, metadata)
- `GET /contracts/{id}/usage` - Get current billing period usage

**Database Functions:**
- `record_usage_event()` and `get_current_usage()` in `api/src/database/contracts.rs`

### What's Missing (Frontend)

- [ ] Offering Creation UI: billing_unit dropdown, pricing_model toggle, overage pricing fields
- [ ] Rental Dashboard: usage display, "X of Y hours used" progress bar, estimated overage charges
- [ ] Stripe metered billing end-to-end testing

---

## Notification System

### Paid Notification Tiers
- Define pricing for additional notifications beyond free tier
- Integrate with payment system (Stripe/ICPay)
- Track paid quota separately from free tier
- Consider monthly subscription vs pay-per-notification

## ICPay Integration

### Manual Payout Requirement
**ICPay does NOT have a programmatic payout API.** Provider payouts must be done manually:
1. View pending releases: `GET /api/v1/admin/payment-releases`
2. Create payouts in icpay.org dashboard (Payouts section)
3. Mark as paid: `POST /api/v1/admin/payouts`

### Future: Automated Payouts
To automate payouts, implement direct ICRC-1 transfers from platform wallet using `ic-agent`.
See [completed spec](docs/completed/2025-12-05-icpay-escrow-payments-spec.md#future-work-automated-provider-payouts) for research details. Requires: platform wallet key management decision.

---

## Rental State Machine Review (2025-12-09)

### Current State Flow
```
requested → accepted → provisioning → provisioned → active
    ↓          ↓           ↓
 rejected   cancelled   cancelled
```

Payment status runs parallel: `pending → succeeded/failed → refunded`

### Other Findings from State Machine Audit

**Low Priority Issues:**

1. **"active" status never assigned** - Contracts stay "provisioned" forever. Both states treated identically in queries. Consider consolidating to single state or adding explicit transition.

2. **No centralized state transition validator** - Each endpoint manually checks valid transitions. Risk of invalid transitions if new endpoints added. Consider creating `validate_transition(old, new) -> bool` function.

3. **Float arithmetic for payment calculation** - Uses `(price * hours / 720.0) * 1e9`. Consider integer-only math to avoid precision loss. Location: `contracts.rs:391-392`

4. **ICPay webhook race condition** - Frontend sets `icpay_transaction_id`, webhook sets `icpay_payment_id`. If webhook arrives first, potential data conflict. Low risk in practice.

5. **No contract archival** - Old completed/cancelled contracts stay in DB indefinitely. Consider cleanup job for contracts older than N years.

**Already Good:**
- ✅ Clean separation of contract status vs payment status
- ✅ Prorated refund calculation is solid
- ✅ Status change history tracked for audit
- ✅ Transaction boundaries for atomic updates
- ✅ Stripe refunds work correctly

---

## Nice-to-Have Improvements

### Cosmetic: Username-Based URLs

**Priority:** LOW - Cosmetic improvement, not blocking anything
**Status:** Documented for future consideration

The frontend currently uses pubkeys in URLs (e.g., `/dashboard/user/abc123...`). While functional, usernames would be cleaner. The account system already links pubkeys to usernames via `account_public_keys` table.

**If implemented later:**
- Add `GET /accounts/by-username/{username}` endpoint
- Frontend resolves pubkey→username for display
- Optional: Support username in URL paths with redirect

No database changes needed - the account→pubkey linking already exists.

---

## Architectural Issues Requiring Review

**Added:** 2025-01-21 (from codebase audit)
**Priority:** MEDIUM - Technical debt that should be addressed

### 1. Duplicate API Response Types (DRY Violation)

**Issue:** `ApiResponse<T>` and related types are duplicated between `api` crate and `dc-agent` crate.

**Locations:**
- `api/src/openapi/common.rs:18` - OpenAPI version with poem attributes
- `dc-agent/src/api_client.rs:30` - Client version without poem attributes

**Also duplicated:**
- `HeartbeatResponse`
- `VmBandwidthReport`
- `ReconcileResponse`, `ReconcileKeepInstance`, `ReconcileTerminateInstance`
- `ContractPendingTermination`

**Impact:** Changes must be synchronized manually; risk of drift.

**Recommended Fix:** Create a shared `dcc-api-types` crate with serde-only types that both crates can depend on. OpenAPI attributes would be added via wrapper types in the API crate.

### 2. ~~Chatwoot Client Error Detail Loss~~ ✅ FIXED (2025-01-21)

**Status:** Fixed - replaced `.unwrap_or_default()` with proper error context in all HTTP response body extractions across:
- `api/src/chatwoot/client.rs` (26 instances)
- `api/src/support_bot/embeddings.rs`
- `api/src/notifications/telegram.rs`
- `api/src/support_bot/llm.rs`

### 3. ~~Hex Decoding Without Validation in Auth~~ ✅ FIXED

**Status:** Fixed - all instances now use proper `match` error handling with logging and appropriate error responses. Verified no `hex::decode...unwrap_or_default` patterns remain in `api/src/openapi/`.

### 4. ~~Commented-Out ICRC3 Modules~~ ✅ FIXED (2026-02-02)

**Status:** Removed dead ICRC3 code:
- Deleted `ic-canister/src/canister_endpoints/icrc3.rs`
- Deleted `ic-canister/src/canister_backend/icrc3.rs`
- Removed commented module declarations from both mod.rs files

Note: `pre_icrc3.rs` is kept as it provides active `get_transactions` and `get_data_certificate` endpoints.

### 5. Hardcoded Localhost URLs as Defaults

**Issue:** Multiple environment variables default to localhost URLs that would break in production if not overridden.

**Examples:**
- `GOOGLE_OAUTH_REDIRECT_URL` defaults to `http://localhost:59011/api/v1/oauth/google/callback`
- `FRONTEND_URL` defaults to `http://localhost:59010` in multiple places
- Inconsistent defaults: some use `59010`, others use `59000`

**Impact:** Misconfiguration risk if deploying without setting all env vars.

**Recommendation:** Either require these vars (fail startup if missing) or use production URLs as defaults with localhost override for development.

### 6. Inconsistent Error Handling Patterns

**Issue:** The codebase uses multiple error handling strategies inconsistently.

**Patterns found:**
1. Custom error types: `TransferError`, `CryptoError`, `AuthError`, `LedgerError`
2. `anyhow::Result<T>` with `.context()`
3. `anyhow::bail!()` for validation
4. `.ok()` to discard errors
5. `.unwrap_or_default()` to silence failures
6. `panic!()` in public APIs (e.g., `common/src/dcc_identity.rs`)

**Impact:** Inconsistent behavior and debugging difficulty.

**Recommendation:** Establish a project-wide error handling policy and apply it consistently. Public functions should return `Result`, not panic.

### 7. Timestamp Handling with `.unwrap_or(0)`

**Issue:** Multiple places use `.timestamp_nanos_opt().unwrap_or(0)` which would silently use epoch time on error.

**Locations:** `api/src/database/agent_delegations.rs:264, 362, 410`

**Impact:** If timestamp calculation fails, code would use 1970-01-01 as timestamp, causing subtle bugs.

**Recommendation:** Since `timestamp_nanos_opt()` only fails for dates beyond year ~2262, this is extremely unlikely. However, logging would make such an edge case visible.

### 8. ~~Geographic Region Mapping Duplicate~~ ✅ FIXED

**Status:** Moved to `common/src/regions.rs` in `dcc-common` crate. Both `api/src/regions.rs` and `dc-agent/src/geolocation.rs` now re-export from `dcc_common::regions`.

### 9. ~~CLI Commands with `todo!()` Stubs~~ ✅ FIXED (2026-02-02)

**Status:** Removed all non-functional CLI commands:
- Deleted `cli/src/commands/contract.rs` and `cli/src/commands/offering.rs`
- Removed `ProviderCommands::UpdateProfile` and `UpdateOffering` stubs
- Removed `common/src/contract_refund_request.rs` (unused)
- Cleaned up corresponding argparse definitions

Users should use `api-cli` for contract/offering management instead.

### 10. Hardcoded Token Value ($1 USD)

**Issue:** Token USD value is hardcoded instead of fetched from exchanges.

**Location:** `ic-canister/src/canister_backend/generic.rs:75-78`
```rust
// FIXME: Get the Token value from ICPSwap and KongSwap
let token_value = 1_000_000; // $1 USD hardcoded
```

**Impact:** All financial calculations using token value are incorrect.

### 11. Hardcoded Secrets in Version Control

**Issue:** HMAC secrets hardcoded in environment files.

**Locations:**
- `cf/.env.prod:70` - Production HMAC secret
- `cf/.env.dev:75` - Development HMAC secret

**Impact:** Security vulnerability. Secrets should be loaded from secure storage, not version control.

### 12. Half-Implemented Canister Proxy

**Issue:** ICP canister method proxy returns stub error.

**Location:** `api/src/main.rs:214-227`
```rust
// TODO: Implement ICP agent and actual canister calls
```

**Impact:** Canister integration non-functional.

### 13. Missing `api-server doctor` Checks

**Issue:** Doctor command missing checks for critical integrations.

**Missing:**
- Stripe webhook registration verification
- Email deliverability test
- DKIM DNS record check
- Cloudflare DNS API access
- OAuth redirect URI accessibility
- ICPay API connectivity

**Recommendation:** Add comprehensive checks for all external service integrations.

### 14. Manual Setup Steps That Could Be Automated

**Issue:** Several setup steps require manual intervention when APIs support automation.

**Examples:**
- Stripe webhook creation (can use Stripe API)
- DKIM DNS record setup (can use Cloudflare API already in codebase)
- Proxmox template VM creation (partially automated but not discoverable)

**Recommendation:** Add `api-server setup-<service>` commands for each integration.

---
