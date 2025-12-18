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

1. **Auto-Accept Mode for Providers**
   - [ ] Add `auto_accept_rentals` boolean to provider settings
   - [ ] When enabled: skip provider approval step entirely
   - [ ] Contract goes directly from `payment-confirmed` → `accepted` → `provisioning`
   - [ ] Provider can still reject/refund problematic rentals after the fact

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

### Current Phase: Delegated Agent Keys + One-Liner UX ✅ COMPLETE

**Goal achieved:** Provider goes from zero to "healthy on dashboard" with:
```bash
dc-agent setup proxmox --host 192.168.1.100   # Auto-detects identity, generates agent key, registers
dc-agent doctor --verify-api                   # Verifies API connectivity
dc-agent run                                   # Starts polling loop
```

#### Phase 4: Delegated Agent Keys (Security) ✅ COMPLETE
Agent uses separate keypair from provider's main key to limit blast radius if compromised.

- [x] **4.1 Database:** Add `provider_agent_delegations` and `provider_agent_status` tables
- [x] **4.2 API:** Delegation CRUD endpoints (`POST/GET/DELETE /providers/{pubkey}/agent-delegations`)
- [x] **4.3 API:** Modify auth to accept agent keys via `X-Agent-Pubkey` header
- [x] **4.4 Agent:** `dc-agent init` command for keypair generation (also integrated into setup)
- [x] **4.5 Agent:** `dc-agent register` command (integrated into setup for one-liner UX)

#### Phase 5: One-Liner UX ✅ COMPLETE
- [x] **5.1 Setup:** Integrated - `dc-agent setup` now auto-generates agent key and registers
- [x] **5.2 Doctor:** Add `--verify-api` flag for API connectivity test
- [x] **5.3 Heartbeat:** `POST /providers/{pubkey}/heartbeat` endpoint + agent integration
- [ ] **5.4 Dashboard:** Show "online"/"offline" badge on provider cards (frontend-only)

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

**Remaining (nice-to-have):**
- Frontend billing settings UI (backend API ready)

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
