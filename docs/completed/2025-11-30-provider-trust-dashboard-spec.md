# Provider Trust Dashboard
**Status:** Complete

## Requirements

### Must-have - DONE
- [x] Landing page: Update messaging to address user fears and trust guarantees
  - Verified: `TrustGuaranteesSection.svelte` imported in `+page.svelte`
  - Verified: `HeroSection.svelte`, `FeaturesSection.svelte`, `BenefitsSection.svelte` updated
- [x] Backend: Provider reliability metrics calculation (6 core metrics)
  - Verified: `get_provider_trust_metrics()` in `api/src/database/stats.rs`
- [x] Backend: Red flag detection (7 critical flags)
  - Verified: `calculate_trust_score_and_flags()` in `api/src/database/stats.rs`
- [x] Backend: Trust score composite calculation (0-100)
  - Verified: Same function with penalty/bonus system
- [x] API: GET /providers/{pubkey}/trust-metrics endpoint
  - Verified: `get_provider_trust_metrics` in `api/src/openapi/providers.rs`
- [x] Frontend: Trust Dashboard component on provider profile page
  - Verified: `TrustDashboard.svelte` exists and imported in `reputation/[pubkey]/+page.svelte`
- [x] Frontend: Trust badge component created
  - Verified: `TrustBadge.svelte` exists
- [x] Tests: Unit tests for metric calculations
  - Verified: 10 trust-specific tests in `stats/tests.rs`

### Must-have - DONE (continued)
- [x] Step 7: Frontend - Trust badge on offering cards
  - Added TrustBadge to marketplace page offering cards
  - Shows trust score from cached provider data
- [x] Step 8: API - Include trust_score in provider listings
  - Added `trust_score` and `has_critical_flags` columns to `provider_profiles` table
  - Migration 018_provider_trust_cache.sql
  - Updated search_offerings query to JOIN with provider_profiles
  - Cache updated when trust metrics are fetched
- [x] Step 9: Frontend - Pre-checkout warning dialog for risky providers
  - Warning integrated directly into RentalRequestDialog.svelte
  - Shows warning when: has_critical_flags=true OR trust_score < 50
  - Links to provider's reputation page for review

### Nice-to-have - DEFERRED
- [ ] Behavioral anomaly detection (cancellation clusters, overcommitment)
- [ ] Price comparison vs market average
- [ ] Caching for expensive metric calculations (partially done: trust_score cached in provider_profiles)

## Architecture

### Data Model

```rust
/// Provider trust metrics - all derivable from existing contract data
pub struct ProviderTrustMetrics {
    pub pubkey: String,

    // Core metrics
    pub trust_score: i64,                   // 0-100 composite
    pub time_to_delivery_hours: Option<f64>, // Average provisioning time
    pub completion_rate_pct: f64,           // % contracts completed
    pub last_active_ns: i64,                // Last check-in timestamp
    pub repeat_customer_count: i64,         // Users with >1 contract
    pub active_contract_value_e9s: i64,     // $ currently serving
    pub total_contracts: i64,               // Track record size

    // Red flags (each is Option - None if insufficient data)
    pub early_cancellation_rate_pct: Option<f64>,
    pub avg_response_time_hours: Option<f64>,
    pub provisioning_failure_rate_pct: Option<f64>,
    pub rejection_rate_pct: Option<f64>,
    pub negative_reputation_90d: i64,
    pub stuck_contracts_value_e9s: i64,     // Money at risk
    pub days_since_last_checkin: i64,

    // Flags
    pub is_new_provider: bool,              // <5 completed contracts
    pub has_critical_flags: bool,           // Any critical threshold exceeded
    pub critical_flag_reasons: Vec<String>, // Human-readable explanations
}
```

### Trust Score Calculation

```
Base Score = 100

Penalties:
- Early cancellation rate >20%: -25 points
- Provisioning failure >15%: -20 points
- Rejection rate >30%: -15 points
- Response time >48h: -15 points
- Negative reputation <-50 (90d): -15 points
- No check-in >7 days with active contracts: -10 points
- Stuck contracts >$5000: -10 points

Bonuses:
- Repeat customers >10: +5 points
- Completion rate >95%: +5 points
- Response time <4h: +5 points

Minimum score: 0
Maximum score: 100
```

### Critical Flag Thresholds

| Flag | Threshold | Severity |
|------|-----------|----------|
| Early cancellation rate | >20% | CRITICAL |
| Response time | >48 hours | CRITICAL |
| Provisioning failure | >15% | CRITICAL |
| Rejection rate | >30% | CRITICAL |
| Negative reputation (90d) | <-50 | CRITICAL |
| Stuck contracts | >$5,000 | WARNING |
| Ghost risk | >7 days inactive + active contracts | CRITICAL |
| New provider | <5 contracts | INFO |

## Execution Log

### Step 0: Landing Page Update - DONE
- **Implementation:** Updated 4 components:
  - `HeroSection.svelte`: Trust-focused rotating words
  - `FeaturesSection.svelte`: Trust features (Trust Score, Red Flags, Escrow, Transparency)
  - `BenefitsSection.svelte`: Provider accountability, user protection focus
  - `TrustGuaranteesSection.svelte`: New - "Your Fears. Our Guarantees."
- **Verification:** Components exist, imported in `+page.svelte`
- **Outcome:** Success

### Step 1: Database Queries - DONE
- **Implementation:** `get_provider_trust_metrics()` in `api/src/database/stats.rs`
- **Verification:** Function exists with 14 SQL queries
- **Outcome:** Success

### Step 2: Trust Score Calculation - DONE
- **Implementation:** `calculate_trust_score_and_flags()` in `api/src/database/stats.rs`
- **Tests:** 10 unit tests (8 score tests + 2 integration tests)
- **Verification:** Tests pass
- **Outcome:** Success

### Step 3: API Endpoint - DONE
- **Implementation:** `GET /providers/:pubkey/trust-metrics` in `api/src/openapi/providers.rs`
- **Verification:** Endpoint exists
- **Outcome:** Success

### Step 4: Frontend Components - DONE
- **Components:**
  - `TrustDashboard.svelte` - Full dashboard component
  - `TrustBadge.svelte` - Compact badge component
  - `getProviderTrustMetrics()` - API function in `api.ts`
  - `ProviderTrustMetrics.ts` - Generated TypeScript types
- **Outcome:** Success

### Step 5: Integration - DONE
- TrustDashboard integrated into `reputation/[pubkey]/+page.svelte`
- TrustBadge integrated into marketplace offering cards (`marketplace/+page.svelte`)
- **Outcome:** Success

### Step 6: Final Review - DONE
- **Clippy:** 4 warnings (non-blocking)
- **Tests:** 27 stats tests pass (10 trust-specific)
- **Build:** Website builds clean
- **Outcome:** Success

## Completion Summary
**Completed:** 2025-11-30 | **Agents:** 1/15 | **Steps:** 9/9
Changes: 12 files, +150/-20 lines, 256 tests pass
Requirements: 11/11 must-have, 0/3 nice-to-have (deferred)
Tests pass ✓, cargo make clean ✓

### Implementation Notes
- Trust score cached in `provider_profiles.trust_score` to avoid N+1 queries
- Trust cache updated automatically when `get_provider_trust_metrics()` is called
- Pre-checkout warning threshold: has_critical_flags=true OR trust_score < 50
- Warning integrated inline in RentalRequestDialog (not a separate dialog) for better UX

### Verification Summary (2025-11-30)
All must-have items verified present in codebase:
- `TrustDashboard.svelte`, `TrustBadge.svelte` - frontend components exist
- `TrustBadge` imported and used in `marketplace/+page.svelte` (lines 242-248)
- Pre-checkout warning in `RentalRequestDialog.svelte` (lines 239-276)
- Migration `018_provider_trust_cache.sql` adds trust_score/has_critical_flags
- `Offering.ts` generated types include trust_score/has_critical_flags fields
