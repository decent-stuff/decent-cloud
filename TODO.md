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

### Phase 6: Health & Reputation
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
