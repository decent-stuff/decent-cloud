# Tier 2: Abandonment Velocity Metric
**Status:** Complete ✅

## Requirements
### Must-have
- [x] Calculate abandonment velocity: ratio of recent cancellation rate vs historical baseline
- [x] Add `abandonment_velocity` field to `ProviderTrustMetrics` struct
- [x] Display metric in TrustDashboard with appropriate warning styling
- [x] Unit tests covering positive/negative paths and edge cases
- [x] Update TODO.md: remove infeasible Tier 2 metrics, mark this complete

### Nice-to-have
- [ ] Include abandonment velocity in trust score calculation (penalty for high velocity)

## Design

### Metric Definition
**Abandonment Velocity** = `recent_cancellation_rate / baseline_cancellation_rate`

Where:
- `recent_cancellation_rate` = cancellations in last 30 days / total completed+cancelled contracts in last 30 days
- `baseline_cancellation_rate` = cancellations in 31-90 days ago / total completed+cancelled contracts in that period

### Interpretation
- `None` - Insufficient data (no contracts in baseline period)
- `< 1.0` - Improving (fewer cancellations recently)
- `1.0` - Stable
- `> 1.5` - Concerning (50%+ increase in cancellation rate)
- `> 2.0` - Critical (doubled cancellation rate)

### Edge Cases
- No contracts in baseline period → `None`
- No contracts in recent period → `0.0` (no recent cancellations = good)
- Zero baseline cancellation rate → Use `recent_rate` directly as velocity (treat 0 baseline as 1 cancellation equivalent to avoid division by zero)

## Steps

### Step 1: Backend Implementation
**Success:** `abandonment_velocity` field added to struct, SQL query implemented, returns correct values
**Status:** Complete

### Step 2: Frontend Display
**Success:** TrustDashboard shows abandonment velocity with warning styling for high values
**Status:** Complete

### Step 3: Unit Tests + Cleanup
**Success:** Tests cover all edge cases, TODO.md updated, cargo make clean
**Status:** Complete

## Execution Log

### Step 1
- **Implementation:** Added `abandonment_velocity` field to `ProviderTrustMetrics` struct (line 788-792). Implemented SQL queries to calculate recent (30d) and baseline (31-90d) cancellation rates using `status_updated_at_ns` timestamp. Logic handles all edge cases per spec: returns None if baseline_total == 0, uses recent_rate directly if baseline_rate == 0, otherwise calculates velocity as recent_rate / baseline_rate.
- **Review:** cargo make passed. TypeScript types exported successfully.
- **Outcome:** Backend implementation complete. Metric correctly calculates abandonment velocity ratio.

### Step 2
- **Implementation:** Added `getVelocityStatus` helper function following exact pattern of `isNoResponseConcerning`. Displays abandonment velocity in new section after no_response_rate_pct with: value formatted as ratio (e.g., "1.5x"), color coding (green <1.5, yellow 1.5-2.0, red >2.0), tooltip description "Ratio of recent (30d) to baseline (31-90d) cancellation rate", and status badges for warning/critical thresholds.
- **Review:** `npm run check` passed with 0 errors and 0 warnings.
- **Outcome:** Frontend implementation complete. TrustDashboard displays abandonment velocity metric with appropriate warning styling.

### Step 3
- **Implementation:** Added 6 unit tests to `/code/api/src/database/stats/tests.rs` following exact pattern of Tier 3 tests: (1) `test_abandonment_velocity_none_no_baseline` - returns None when no baseline contracts exist, (2) `test_abandonment_velocity_zero_no_recent` - returns 0.0 when baseline exists but no recent contracts, (3) `test_abandonment_velocity_stable` - returns ~1.0 when rates are equal (20% baseline, 20% recent), (4) `test_abandonment_velocity_improving` - returns 0.5 when recent rate is lower (40% baseline, 20% recent), (5) `test_abandonment_velocity_spike` - returns 5.0 when recent rate spikes (10% baseline, 50% recent), (6) `test_abandonment_velocity_baseline_zero_cancellations` - returns recent_rate directly (0.2) when baseline has 0% cancellation rate. Updated TODO.md to mark abandonment velocity complete and remove infeasible Tier 2 metrics (Cancellation Cluster Detection, Overcommitment Warning, Price Spike Detection).
- **Review:** `cargo nextest run -p api` passed all 351 tests including 6 new abandonment velocity tests. `cargo make` passed cleanly with all 489 tests passing.
- **Outcome:** Unit tests complete. All tests assert meaningful behavior without overlap. TODO.md updated to reflect completion.

## Completion Summary

**Status:** COMPLETE (2025-11-30)

All requirements met:
- Backend calculates abandonment velocity as ratio of recent (30d) to baseline (31-90d) cancellation rate
- Handles all edge cases correctly: None when no baseline data, 0.0 when no recent activity, recent_rate when baseline_rate is 0.0
- Frontend displays metric in TrustDashboard with color-coded warnings (green <1.5, yellow 1.5-2.0, red >2.0)
- 6 unit tests cover all positive/negative paths and edge cases
- TODO.md updated to mark complete and remove infeasible Tier 2 metrics
- All tests passing (351 unit tests, 489 total with canister tests)

**Key Decisions:**
- Used `status_updated_at_ns` timestamp for measuring when contracts were cancelled (consistent with other metrics)
- Baseline period: 31-90 days ago (avoids overlap with recent 30d period)
- Edge case handling follows spec exactly: None for insufficient data, 0.0 for improvement, direct recent_rate when baseline has no cancellations
