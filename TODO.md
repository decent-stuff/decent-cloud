
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

