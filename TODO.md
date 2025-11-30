- improve search support, if possible add a minimal search DSL for filtering results, e.g. like on ServerHunter https://www.serverhunter.com/#query=stock:(in_stock+OR+unknown)
- Admin UI with support for:
    - sending test and other emails
    - performing actions in the name of other users (impersonating)

## Provider Trust & Reliability System

### Trust Dashboard (Core Display)
Visual trust dashboard on provider profiles and offering listings showing:
- Trust Score (0-100 composite score with color-coded badge)
- Time to Delivery (median hours from payment to working service)
- Success/Completion Rate (% contracts completed vs cancelled/failed)
- Last Activity indicator (real-time "last seen X ago")
- Repeat Customer Count (users who came back for more)
- Active Contract Value ($ currently being served)

### Red Flag Detection (Prominent Warnings)

#### Tier 1 - Critical Flags
- **Early Cancellation Rate**: % contracts cancelled within first 10% of duration (threshold: >20% = critical)
- **Provider Response Time**: Average hours from request to first response (threshold: >48h = critical)
- **Provisioning Failure Rate**: % accepted contracts never provisioned (threshold: >15% = critical)
- **Rejection Rate**: % contract requests rejected (threshold: >30% = critical)
- **Negative Reputation Trend**: Sum of negative reputation changes in last 90 days (threshold: <-50 = critical)
- **Money at Risk**: Total $ in "stuck" contracts (requested/pending/accepted >72h without progress)
- **Ghost Risk**: Provider with no check-in in >7 days but has active contracts

#### Tier 2 - Behavioral Anomaly Detection
- **Cancellation Cluster Detection**: Alert when 3+ cancellations occur within 48 hours (pattern suggests sudden degradation)
- **Overcommitment Warning**: Provider has >2x their historical average active contracts
- **Price Spike Detection**: Provider raised prices >50% recently (could indicate desperation)
- **Abandonment Velocity**: Sudden spike in cancellations after stable history

#### Tier 3 - Contextual Info
- **Refund Processing Speed**: Average time from cancellation to refund
- **Provider Tenure**: New (<5 contracts), Growing (5-20), Established (20+)
- **Average Contract Duration** vs expected (contracts ending early = quality issues)
- **No Response Rate**: % requests still in "requested" status after 7 days

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
