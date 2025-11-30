# Tier 2: Abandonment Velocity Metric
**Status:** In Progress

## Requirements
### Must-have
- [ ] Calculate abandonment velocity: ratio of recent cancellation rate vs historical baseline
- [ ] Add `abandonment_velocity` field to `ProviderTrustMetrics` struct
- [ ] Display metric in TrustDashboard with appropriate warning styling
- [ ] Unit tests covering positive/negative paths and edge cases
- [ ] Update TODO.md: remove infeasible Tier 2 metrics, mark this complete

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
**Status:** Pending

### Step 3: Unit Tests + Cleanup
**Success:** Tests cover all edge cases, TODO.md updated, cargo make clean
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:** Added `abandonment_velocity` field to `ProviderTrustMetrics` struct (line 788-792). Implemented SQL queries to calculate recent (30d) and baseline (31-90d) cancellation rates using `status_updated_at_ns` timestamp. Logic handles all edge cases per spec: returns None if baseline_total == 0, uses recent_rate directly if baseline_rate == 0, otherwise calculates velocity as recent_rate / baseline_rate.
- **Review:** cargo make passed. TypeScript types exported successfully.
- **Outcome:** Backend implementation complete. Metric correctly calculates abandonment velocity ratio.

### Step 2
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

### Step 3
- **Implementation:** (pending)
- **Review:** (pending)
- **Outcome:** (pending)

## Completion Summary
(To be filled in Phase 4)
