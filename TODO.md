
## Provider Trust & Reliability System

### External Benchmarking Integration
Integrate with or scrape external sources for additional trust signals:
- https://serververify.com/ - Server verification and uptime data
- https://www.vpsbenchmarks.com/ - VPS performance benchmarks
- Price comparison vs market average ("15% below market" or "30% above")
- Cross-reference provider claims with independent verification

### In-Contract Communication & Tracking

**Solution: Chatwoot Integration** - See [Integration Plan](docs/chatwoot-integration-plan.md)

Using Chatwoot Community Edition (MIT license, self-hosted, free) to provide:

- [x] Research ticketing/messaging solutions
- [x] Select solution: Chatwoot Community Edition
- [x] Design authentication integration (HMAC for customers, separate credentials for providers)
- [x] Document integration plan
- [ ] Deploy Chatwoot infrastructure (Docker + Redis, uses existing PostgreSQL)
- [ ] Implement backend integration (Rust: HMAC generation, API client, webhooks)
- [ ] Implement frontend integration (SvelteKit: widget component)
- [ ] Hook provider registration → Chatwoot agent creation
- [ ] Hook contract creation → Chatwoot conversation creation

**Features delivered by Chatwoot:**
- Message/ticket system between users and providers
- Response time tracking (First Response Time metrics built-in)
- Timestamped communication logs
- "Provider went silent" detection via webhook events
- Multi-channel support (web, email, WhatsApp, Telegram, etc.)
- Mobile apps for providers
- Email notifications
- AI-assisted replies (OpenAI BYOK)

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
