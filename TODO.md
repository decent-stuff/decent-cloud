# TODO

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

### Required Changes

1. **Auto-Accept Mode for Providers** ✅ COMPLETE
   - [x] Add `auto_accept_rentals` boolean to provider settings (migration 048)
   - [x] API endpoints: GET/PUT `/provider/settings/auto-accept`
   - [x] When enabled: skip provider approval step entirely
   - [x] Contract goes directly from `payment-confirmed` → `accepted` → `provisioning`
   - [x] Works for both ICPay (immediate) and Stripe (after webhook confirmation)
   - [x] Provider can still reject/refund problematic rentals after the fact

2. **Pre-configured Offering Templates**
   - [ ] Providers define offering specs tied to VM templates
   - [ ] dc-agent auto-provisions matching template when contract created
   - [ ] No human-in-the-loop for standard offerings

3. **Instant SSH Key Injection**
   - [x] SSH key now required at rental time (done)
   - [x] SSH key validated both frontend and backend (done)
   - [x] SSH key passed to dc-agent during provisioning (done)
   - [x] User notified via email when VM ready (done)

4. **Provisioning Time Target**
   - Goal: < 60 seconds from payment to SSH access
   - Currently: minutes-hours (depends on provider approval)
   - With auto-accept + dc-agent: ~30-60 seconds (clone + boot + network)

### Implementation Notes
- Auto-accept should be opt-in for providers (some may want manual review)
- High-trust providers (verified, good uptime) could default to auto-accept
- Consider tiered SLAs: "Instant" (auto-accept) vs "Standard" (manual review)

---

## Provider Provisioning Agent

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** MVP complete (Phase 4-5 done: delegated keys, heartbeat, doctor --verify-api)
**Priority:** HIGH - Critical for automated cloud platform

Software that providers run to automatically provision services when contracts are accepted. Transforms Decent Cloud from "marketplace with manual fulfillment" to "automated cloud platform."

**Prerequisite:** ✅ Payments fully working (Stripe + ICPay complete)

### Completed ✓
- [x] Agent skeleton with polling loop (`dc-agent run`)
- [x] Configuration file parsing (TOML)
- [x] Ed25519 authentication with API
- [x] Proxmox provisioner (full VM lifecycle)
- [x] Setup wizard (`dc-agent setup proxmox`)
- [x] Test provisioning (`dc-agent test-provision`)
- [x] API: `GET /providers/{pubkey}/contracts/pending-provision`
- [x] API: `PUT /provider/rental-requests/{id}/provisioning`

### Phases 4-5: Delegated Agent Keys + One-Liner UX ✅ COMPLETE

**Goal achieved:** Provider goes from zero to "healthy on dashboard" with:
```bash
dc-agent setup proxmox --host 192.168.1.100   # Auto-detects identity, generates agent key, registers
dc-agent doctor --verify-api                   # Verifies API connectivity
dc-agent run                                   # Starts polling loop
```

- [x] **4.1 Database:** Add `provider_agent_delegations` and `provider_agent_status` tables
- [x] **4.2 API:** Delegation CRUD endpoints (`POST/GET/DELETE /providers/{pubkey}/agent-delegations`)
- [x] **4.3 API:** Modify auth to accept agent keys via `X-Agent-Pubkey` header
- [x] **4.4 Agent:** `dc-agent init` command for keypair generation (also integrated into setup)
- [x] **4.5 Agent:** `dc-agent register` command (integrated into setup for one-liner UX)
- [x] **5.1 Setup:** Integrated - `dc-agent setup` now auto-generates agent key and registers
- [x] **5.2 Doctor:** Add `--verify-api` flag for API connectivity test
- [x] **5.3 Heartbeat:** `POST /providers/{pubkey}/heartbeat` endpoint + agent integration
- [x] **5.4 Dashboard:** Show "online"/"offline" badge on marketplace offerings (frontend + API)

### Phase 5.5: Agent Pools ✅ COMPLETE (2025-12-20)

**Spec:** [agent-pools.md](docs/specs/agent-pools.md)

Enables multiple DC-Agents (one per hypervisor) with load distribution and location-based routing. Prevents race conditions where multiple agents try to provision the same contract.

- [x] **Database:** `agent_pools`, `agent_setup_tokens` tables + lock columns on contracts
- [x] **Pool CRUD:** Create/list/update/delete pools with location + provisioner type
- [x] **Setup Tokens:** One-time tokens for agent registration to specific pools
- [x] **Agent Setup:** `dc-agent setup token --token <TOKEN>` command
- [x] **Two-Phase Provisioning:** Lock acquisition prevents race conditions
- [x] **Pool Filtering:** Agents only see contracts matching their pool's location
- [x] **Background Jobs:** Lock expiry + token cleanup in cleanup_service
- [x] **Frontend:** Pool management UI at `/dashboard/provider/agents`

#### Phase 6: Health & Reputation
- [ ] **6.1 API:** `POST /contracts/{id}/health` endpoint + `contract_health_checks` table
- [ ] **6.2 API:** Uptime calculation per provider
- [ ] **6.3 Dashboard:** Show uptime percentage on provider profile

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

## Billing & Invoicing - ✅ COMPLETE

**Spec:** [2025-12-08-billing-remaining-items-spec.md](docs/2025-12-08-billing-remaining-items-spec.md)
**Completed:** 2025-12-08

All billing features implemented:
- ✅ Stripe Checkout Sessions with automatic_tax
- ✅ VAT ID validation via VIES API (`POST /api/v1/vat/validate`)
- ✅ Reverse charge logic for B2B EU transactions
- ✅ User billing settings API (`GET/PUT /api/v1/accounts/billing`)
- ✅ Frontend redirects to Stripe Checkout
- ✅ Account subscription plans (Free/Pro/Enterprise) with Stripe Billing

**Remaining (nice-to-have):**
- Frontend billing settings UI (backend API ready)

---

## Usage-Based Billing for Rentals

**Priority:** HIGH - Enables flexible pricing models for different workloads
**Status:** Planned

### Overview

Extend the fixed-price subscription system to support usage-based billing per rental/offering. This enables:
- **Flat monthly fees** for dedicated servers (current model)
- **Per-minute billing** for ephemeral workloads (CI runners, burst compute)
- **Hourly/daily billing** for development environments
- **Overage charges** for exceeding included quotas

### Billing Units

Each offering specifies its billing unit:
| Unit | Use Case | Example |
|------|----------|---------|
| `month` | Dedicated servers, long-term VPS | $99/month flat |
| `day` | Development environments | $5/day |
| `hour` | GPU compute, batch jobs | $2.50/hour |
| `minute` | CI runners, serverless | $0.05/minute |

### Pricing Models

1. **Flat Fee** (`flat`) - Fixed price for the billing period, no usage tracking
2. **Usage + Overage** (`usage_overage`) - Base included usage + per-unit overage charges

Example overage model:
```
Base: $50/month includes 100 hours
Overage: $0.75/hour after 100 hours
User uses 150 hours → $50 + (50 × $0.75) = $87.50
```

### Implementation Plan

#### Phase 1: Schema Changes

**Migration: Add billing fields to offerings**
```sql
ALTER TABLE provider_offerings ADD COLUMN billing_unit TEXT DEFAULT 'month';
  -- 'minute', 'hour', 'day', 'month'

ALTER TABLE provider_offerings ADD COLUMN pricing_model TEXT DEFAULT 'flat';
  -- 'flat', 'usage_overage'

ALTER TABLE provider_offerings ADD COLUMN price_per_unit REAL;
  -- Price per billing_unit (e.g., $0.05 per minute)

ALTER TABLE provider_offerings ADD COLUMN included_units INTEGER;
  -- For overage model: units included in base price (NULL = unlimited for flat)

ALTER TABLE provider_offerings ADD COLUMN overage_price_per_unit REAL;
  -- Price per unit after included_units exhausted
```

**Migration: Usage tracking table**
```sql
CREATE TABLE contract_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    billing_period_start INTEGER NOT NULL,  -- Unix timestamp
    billing_period_end INTEGER NOT NULL,
    units_used REAL NOT NULL DEFAULT 0,     -- Accumulated usage
    units_included REAL,                    -- Snapshot from offering
    overage_units REAL DEFAULT 0,           -- units_used - units_included (if positive)
    reported_to_stripe INTEGER DEFAULT 0,   -- Boolean: usage reported to Stripe
    stripe_usage_record_id TEXT,            -- Stripe usage record ID
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);
```

#### Phase 2: Stripe Metered Billing Integration

**Stripe Setup:**
1. Create metered prices in Stripe Dashboard with `recurring.usage_type = metered`
2. Store `stripe_metered_price_id` in offerings table
3. Use `aggregate_usage = sum` for cumulative billing

**Usage Reporting Flow:**
```
Contract Active
    ↓
dc-agent reports uptime/usage → API stores in contract_usage
    ↓
End of billing period (cron job or Stripe invoice.created webhook)
    ↓
API calls stripe.subscription_items.create_usage_record()
    ↓
Stripe generates invoice with usage charges
```

#### Phase 3: API Endpoints

```
POST /contracts/{id}/usage
  - dc-agent reports usage (minutes/hours active)
  - Body: { "units": 60, "timestamp": 1703001234 }

GET /contracts/{id}/usage
  - Get current billing period usage
  - Response: { "units_used": 120, "units_included": 100, "overage": 20, "estimated_charge": 15.00 }

POST /internal/billing/report-usage (cron job)
  - Aggregate and report usage to Stripe for all active contracts
  - Called at end of each billing period
```

#### Phase 4: Frontend Changes

**Offering Creation UI:**
- Add billing_unit dropdown (minute/hour/day/month)
- Add pricing_model toggle (flat vs usage+overage)
- Conditional fields for overage pricing

**Rental Dashboard:**
- Show current usage for active contracts
- Display "X of Y hours used" progress bar
- Show estimated charges for overage

### Files to Modify

**Backend:**
- `api/migrations/059_usage_billing.sql` - Schema changes
- `api/src/database/offerings.rs` - Add billing fields
- `api/src/database/contracts.rs` - Add usage tracking
- `api/src/stripe_client.rs` - Add `create_usage_record()` method
- `api/src/openapi/contracts.rs` - Add usage reporting endpoints
- `api/src/cleanup_service.rs` - Add end-of-period usage aggregation job

**Frontend:**
- `website/src/lib/services/api.ts` - Usage API functions
- `website/src/routes/dashboard/provider/offerings/` - Billing config UI
- `website/src/routes/dashboard/rentals/` - Usage display

### Design Decisions

1. **Usage source:** Heartbeat-based - API calculates usage duration from existing agent heartbeats (no new agent changes needed)

2. **Billing period alignment:** Per-contract - Period starts at contract creation (simpler implementation)

3. **Grace period:** 24-hour grace window for late reports, then estimate from last known heartbeat state

---

## Notification System

### Paid Notification Tiers
- Define pricing for additional notifications beyond free tier
- Integrate with payment system (Stripe/ICPay)
- Track paid quota separately from free tier
- Consider monthly subscription vs pay-per-notification

## Rentals Deep-Linking - ✅ COMPLETE

**Completed:** 2025-12-16

Direct links to specific contracts work via `/dashboard/rentals?contract=<contract_id>`:
- URL param `contract` is read on page load
- Page auto-scrolls to the matching contract
- Highlighted contract has blue ring styling to stand out

---

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
