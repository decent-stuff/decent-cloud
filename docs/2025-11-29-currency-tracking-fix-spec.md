# Currency Tracking and Display Fix
**Status:** In Progress
**Date:** 2025-11-29

## Problem Statement
Contracts are displaying incorrect currency (USD) when they should show the actual offering currency (EUR, ICP, etc.). Example:
- Offering "Performance VPS" has price 15 EUR
- Contract shows "15.00 USD" instead of "15.00 EUR"

## Root Cause
Migration 013_contract_currency.sql defaults currency to `'usd'`:
```sql
ALTER TABLE contract_sign_requests ADD COLUMN currency TEXT NOT NULL DEFAULT 'usd';
```

This causes:
1. Silent data corruption - wrong currency displayed without any indication
2. Violates principle of "fail fast" - should show "???" if currency unknown
3. May be applying DEFAULT even when code passes explicit currency value

## Requirements

### Must-have
- [ ] Change DEFAULT from `'usd'` to `'???'` to make errors obvious
- [ ] Verify contract creation correctly passes offering.currency
- [ ] Update any existing contracts with wrong currency from offering data
- [ ] Test that new contracts get correct currency from offerings
- [ ] Verify UI displays currency correctly across all views

### Nice-to-have
- [ ] Add database constraint to prevent invalid currency codes (except '???')
- [ ] Log warning if currency is '???' to help debugging

## Steps

### Step 1: Fix migration default value
**Success:** Migration uses '???' as DEFAULT instead of 'usd'
**Status:** Pending

### Step 2: Create data migration to fix existing contracts
**Success:** All existing contracts have correct currency from their offering
**Status:** Pending

### Step 3: Verify contract creation logic
**Success:** New contracts correctly inherit currency from offerings
**Status:** Pending

### Step 4: Test end-to-end currency flow
**Success:** E2E tests pass showing correct currency from offering → contract → UI
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:**
  - Changed migration 013 DEFAULT from 'usd' to '???'
  - Created migration 014 to fix existing contracts with wrong currency
- **Files:**
  - api/migrations/013_contract_currency.sql
  - api/migrations/014_fix_contract_currency_data.sql
- **Review:** Pending
- **Verification:** Pending
- **Outcome:** Pending

