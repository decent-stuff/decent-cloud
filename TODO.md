# TODO

## Provider Provisioning Agent

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** Ready to implement, large feature, needs /orchestrate
**Priority:** HIGH - Critical for automated cloud platform

Software that providers run to automatically provision services when contracts are accepted. Transforms Decent Cloud from "marketplace with manual fulfillment" to "automated cloud platform."

**Prerequisite:** ✅ Payments fully working (Stripe + ICPay complete)

### Key Components
- Polling-based agent daemon (`dc-agent`)
- Provisioner plugins: Hetzner, Proxmox, Docker, Manual
- Health check reporting → feeds into reputation
- Credential encryption (user's pubkey)

### Implementation Order
1. API extensions (provisioning endpoints)
2. Agent MVP with manual provisioner
3. Hetzner Cloud provisioner
4. Health check + reputation integration

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
