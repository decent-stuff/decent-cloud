
## Architectural Issues Requiring Review

(No current issues)

---

## ICPay Integration (In Progress)

**Spec:** [2025-12-05-icpay-escrow-payments-spec.md](docs/2025-12-05-icpay-escrow-payments-spec.md)
**Status:** PLANNING

### Current State
- Basic SDK initialization: ✅ Done
- Frontend payment execution: ❌ Broken (missing wallet config)
- Backend verification: ❌ Stub only
- Webhooks: ❌ Not implemented
- Refunds: ❌ Not implemented for ICPay

### Planned Features
- Prorated payments/refunds (match Stripe parity)
- Daily payment release to providers
- Webhook integration for real-time confirmation
- Provider payout system

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
