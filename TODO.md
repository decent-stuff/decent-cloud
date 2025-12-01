## Admin Tools ✅ COMPLETED (2025-11-30)
See docs/2025-11-30-admin-tools-spec.md for implementation details.

### CLI Admin Management (`api-cli` binary)
- [x] `api-cli admin grant <username>` - Grant admin access to an account
- [x] `api-cli admin revoke <username>` - Revoke admin access from an account
- [x] `api-cli admin list` - List all admin accounts
- [x] `api-cli test-email --to <email>` - Send test email (absorbs test-email binary)
- [x] `--env dev|prod` flag for environment selection

### Admin Dashboard (Web UI) - `/dashboard/admin`
- [x] Admin-only route with auth guard checking `account.isAdmin`
- [x] View failed emails queue with retry action
- [x] Email queue inspection (pending, sent, failed counts)
- [ ] Send test email to verify configuration (nice-to-have)
- [ ] Account lookup and management (view keys, disable keys, add recovery keys) (nice-to-have)

### Database Operations (Admin API)
- [x] Reset email retry counter for specific email (`POST /admin/emails/reset/:email_id`)
- [x] Reset all failed emails to pending (`POST /admin/emails/retry-all-failed`)
- [x] Get email queue stats (`GET /admin/emails/stats`)
- [ ] View/edit account email verification status (nice-to-have)

## Provider Trust & Reliability System

### Trust Dashboard (Core Display)
Visual trust dashboard on provider profiles and offering listings showing:
- Trust Score (0-100 composite score with color-coded badge)
- Time to Delivery (median hours from payment to working service)
- Success/Completion Rate (% contracts completed vs cancelled/failed)
- Last Activity indicator (real-time "last seen X ago")
- Repeat Customer Count (users who came back for more)
- Active Contract Value ($ currently being served)

### External Benchmarking Integration
Integrate with or scrape external sources for additional trust signals:
- https://serververify.com/ - Server verification and uptime data
- https://www.vpsbenchmarks.com/ - VPS performance benchmarks
- Price comparison vs market average ("15% below market" or "30% above")
- Cross-reference provider claims with independent verification

### Future Tracking (Requires New Infrastructure)

#### In-Contract Communication Tracking
- Track provider response time to user messages/tickets during active contracts
- "Average support response: 2 hours" vs "No responses in 5 days"
- Requires: Message/ticket system between users and providers

#### Service Quality Verification
- Automated health checks on provisioned services
- Uptime monitoring and SLA compliance tracking
- "99.2% uptime in last 30 days" with proof
- Requires: Infrastructure monitoring agents or integration with external monitors

#### User Feedback System (Structured, Not Reviews)
- Post-contract structured survey: "Did service match description?" Y/N
- "Would you rent from this provider again?" Y/N
- Binary signals harder to game than star ratings
- Requires: Post-contract feedback workflow

#### Contract Communication Log
- Timestamp all provider-user interactions
- Detect "provider went silent during contract" patterns
- Requires: Messaging infrastructure

### Display Strategy
- **Provider Profile Page**: Large trust score badge, expandable metrics section, prominent red flag warnings
- **Offering Listings**: Small trust score badge, hover tooltip with top concerns, red border on risky offerings
- **Pre-Checkout Warning**: If provider has any critical red flags, show confirmation dialog with specific concerns
- **Data Freshness**: Calculate from last 90 days, require minimum 5 contracts for score (else "New Provider - Insufficient Data")
