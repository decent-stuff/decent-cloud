## Billing & Invoicing ✅ COMPLETE

**Spec:** [2025-12-07-billing-invoicing-spec.md](docs/2025-12-07-billing-invoicing-spec.md)
**Status:** Complete (2025-12-07)

| Phase | Feature                      | Status                     |
|-------|------------------------------|----------------------------|
| 1     | Receipt emails after payment | ✅ Done                     |
| 2     | PDF invoice generation (B2B) | ✅ Done                     |
| 3     | Stripe Tax integration (VAT) | ✅ Infrastructure ready     |
| 4     | User billing settings        | ⏸️ Deferred (nice-to-have) |

**Compliance Gaps & Tax Analysis:** See [spec](docs/2025-12-07-billing-invoicing-spec.md#compliance-gaps--tax-analysis) for:
- Current compliance status
- Environment variables for EU compliance (`INVOICE_SELLER_*`)
- Stripe Tax cost analysis (~0.5% per transaction)
- Manual VAT lookup alternative
- Prepaid vs postpaid payment model analysis

**Note:** Stripe automatic tax requires migrating from Payment Intents to Checkout Sessions. See `api/docs/stripe-tax-integration.md`.

### Compliance Gaps

| Gap                  | Requirement               | Fix                                 | Priority |
|----------------------|---------------------------|-------------------------------------|----------|
| Buyer address        | Required for B2B invoices | ✅ Done (checkout flow + invoices)  | DONE     |
| VAT auto-calculation | Per-country rates         | Migrate to Stripe Checkout Sessions | MEDIUM   |
| VAT ID validation    | VIES API verification     | ~50 lines, optional                 | LOW      |
| Reverse charge       | B2B cross-border          | Schema ready, logic TBD             | LOW      |

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

## Provider Provisioning Agent

**Spec:** [2025-12-07-provider-provisioning-agent-spec.md](docs/2025-12-07-provider-provisioning-agent-spec.md)
**Status:** Ready to implement
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

## Notification System - Deferred Items

### Paid Notification Tiers
- Define pricing for additional notifications beyond free tier
- Integrate with payment system (Stripe/ICPay)
- Track paid quota separately from free tier
- Consider monthly subscription vs pay-per-notification
