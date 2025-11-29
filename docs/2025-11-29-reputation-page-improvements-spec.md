# Reputation Page Improvements: Rental Metrics and Currency Fixes
**Status:** In Progress
**Date:** 2025-11-29

## Problem Statement

The reputation page doesn't clearly show critical rental behavior patterns that could indicate issues:

**Example from https://decent-cloud.org/dashboard/reputation/3e9f603a...**:
- Shows "2 rentals as requester, 2 as provider"
- All 4 rentals show "Duration: 720 hours" but all were cancelled
- Cannot see actual runtime (how long they ran before cancellation)
- Cannot see that 100% were cancelled quickly (concerning pattern)
- Amounts shown in wrong currency (ICP instead of actual offering currency)

**User Requirements:**
1. Show actual rental duration (not just planned duration)
2. Show early cancellation % (within 1h, 24h, 7d, 180d of payment)
3. Fix currency defaults - use '???' not 'ICP'/'USD'/'DCT'
4. Make suspicious patterns obvious

## Requirements

### Must-have
- [ ] Expose `status_updated_at_ns` in Contract struct and API
- [ ] Calculate actual runtime duration (time from created_at_ns to status_updated_at_ns for cancelled/completed)
- [ ] Add early cancellation metrics to reputation page (% cancelled within 1h, 24h, 7d, 180d)
- [ ] Create NEW migration 017 to drop DEFAULT from contract_sign_requests.currency
- [ ] Remove formatBalance 'dct' default - return error if currency missing
- [ ] Update all formatBalance callsites to handle missing currency explicitly
- [ ] Verify all INSERT statements provide explicit currency value
- [ ] Show actual runtime prominently in rental cards
- [ ] Add summary stats showing cancellation patterns

### Nice-to-have
- [ ] Visual indicators for concerning patterns (e.g., >50% cancelled within 1h)
- [ ] Success rate metric (% completed vs cancelled)
- [ ] Average contract duration for completed rentals

## Steps

### Step 1: Add status_updated_at_ns to Contract struct and API
**Success:**
- Contract struct includes status_updated_at_ns field
- All SELECT queries include status_updated_at_ns
- TypeScript types updated
- Backend compiles cleanly
**Status:** Pending

### Step 2: Calculate and display actual rental duration
**Success:**
- Frontend calculates runtime from created_at_ns to status_updated_at_ns (or now if active)
- Rental cards show "Actual runtime: X hours/days" prominently
- Distinguish between "Planned: 720h" and "Actual: 0.5h"
**Status:** Pending

### Step 3: Add cancellation metrics calculation
**Success:**
- Calculate % of rentals cancelled within: 1h, 24h, 7d, 180d
- Calculate for both "as requester" and "as provider" separately
- Show in summary stats section
**Status:** Pending

### Step 4: Remove ALL currency defaults - enforce explicit values
**Success:**
- Create migration 017 to drop DEFAULT from contract_sign_requests.currency column
- Migration should fail on INSERT if currency not provided (NOT NULL without DEFAULT)
- Remove 'dct' default from formatBalance function parameter
- Update all formatBalance callsites to pass explicit currency or show error
- Verify all INSERT statements provide explicit currency value
- Tests pass with explicit currency enforcement
**Status:** Pending

### Step 5: Improve UI to highlight patterns
**Success:**
- Summary cards show cancellation metrics prominently
- Visual warnings for concerning patterns (>50% cancelled <1h, >80% cancelled <24h)
- Clear distinction between planned vs actual duration
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

### Step 2
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

### Step 3
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

### Step 4
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

### Step 5
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

## Completion Summary
(To be filled in Phase 4)
