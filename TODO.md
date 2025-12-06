
## Architectural Issues Requiring Review

(No current issues)

---

## Notification System - Deferred Items

### Rate Limiting (Free Tier)
- Enforce daily limits in notification sending logic:
  - Email: Unlimited
  - Telegram: 50/day free
  - SMS: 5/day free => consider using https://textbee.dev/
- Check usage before sending, return error when limit exceeded
- Frontend already shows usage vs limits

### Paid Notification Tiers
- Define pricing for additional notifications beyond free tier
- Integrate with payment system (Stripe/ICPay)
- Track paid quota separately from free tier
- Consider monthly subscription vs pay-per-notification

---

## ICPay Integration

**Spec:** [2025-12-05-icpay-escrow-payments-spec.md](docs/2025-12-05-icpay-escrow-payments-spec.md)
**Status:** COMPLETE (with manual payout limitation)

### Implemented
- ✅ Frontend wallet integration (SDK + widget)
- ✅ Backend payment verification
- ✅ Webhook handler with HMAC-SHA256 signature verification
- ✅ Prorated refunds on cancellation
- ✅ Daily payment release tracking (PaymentReleaseService)
- ✅ Admin endpoints for viewing pending releases

### Manual Payout Requirement
**ICPay does NOT have a programmatic payout API.** Provider payouts must be done manually:
1. View pending releases: `GET /api/v1/admin/payment-releases`
2. Create payouts in icpay.org dashboard (Payouts section)
3. Mark as paid: `POST /api/v1/admin/payouts`

### Future: Automated Payouts
To automate payouts, implement direct ICRC-1 transfers from platform wallet using `ic-agent`.
See spec for research details. Requires: platform wallet key management decision.

---

## Provider Trust & Reliability System

### External Benchmarking Integration
Integrate with or scrape external sources for additional trust signals:
- https://serververify.com/ - Server verification and uptime data
- https://www.vpsbenchmarks.com/ - VPS performance benchmarks
- Price comparison vs market average ("15% below market" or "30% above")
- Cross-reference provider claims with independent verification

### In-Contract Communication & Tracking

**Stack:** Chatwoot (MIT, self-hosted) + custom AgentBot + notification bridge.
See [Support Bot & Notification System spec](docs/specs/support-bot-notification-system.md).
In-progress: see docs/2025-12-04-support-bot-notification-system-spec.md

**Chatwoot provides:**
- Ticketing/messaging between users and providers
- Help Center (native KB for provider FAQs)
- Response time tracking, CSAT surveys
- Multi-channel (web, email, Telegram, WhatsApp)
- Webhooks for bot integration and escalation

**We build:**
- AI Bot (~200 lines): answers from Help Center articles, cites sources
- Notification Bridge (~150 lines): alerts providers via Telegram/SMS on escalation

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
