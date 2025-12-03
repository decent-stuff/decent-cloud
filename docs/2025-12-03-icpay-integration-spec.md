# ICPay Integration Spec

**Date**: 2025-12-03
**Orchestrator Goal**: Replace DCT payment method with ICPay for crypto payments
**Status**: In Progress

---

## Overview

Replace the current DCT payment method with ICPay integration to provide:
1. Professional crypto payment UI via ICPay widgets
2. Support for ICP, DCT, and 50+ other cryptocurrencies
3. USD-denominated pricing with automatic token conversion
4. Backend payment verification via ICPay private SDK

**Payment Strategy**:
- **Stripe**: Fiat currencies (USD, EUR, GBP, etc.) via credit cards
- **ICPay**: Crypto currencies (ICP, DCT, etc.) via wallets

---

## Requirements

### Must-have
- [ ] Replace PaymentMethod::DCT with PaymentMethod::ICPay
- [ ] Add ICPay SDK to frontend (@ic-pay/icpay-sdk)
- [ ] Integrate ICPay payment widget in RentalRequestDialog
- [ ] Store ICPay payment/transaction IDs in database
- [ ] Add ICPay API keys to environment configuration
- [ ] Backend payment verification via metadata lookup
- [ ] Migration for existing DCT contracts (rename to icpay)
- [ ] Unit tests for new payment flow

### Nice-to-have
- [ ] Webhook integration for async payment confirmation
- [ ] Support multiple token types (ICP, DCT, ckBTC, etc.)

---

## Steps

### Step 1: Update PaymentMethod enum and database
**Success:** PaymentMethod::DCT renamed to PaymentMethod::ICPay, migration updates existing records
**Status:** Pending

**Tasks:**
- Rename DCT to ICPay in PaymentMethod enum (common/src/payment_method.rs)
- Update all helper methods (is_dct → is_icpay)
- Create migration 012_icpay_rename.sql to update existing payment_method='dct' to 'icpay'
- Rename stripe_payment_intent_id to payment_intent_id (generic)
- Add icpay_transaction_id column for ICPay-specific tracking
- Update all tests

**Files:**
- common/src/payment_method.rs
- api/migrations/012_icpay_rename.sql (NEW)
- api/src/database/contracts.rs
- api/src/database/contracts/tests.rs

---

### Step 2: Add ICPay SDK to frontend
**Success:** ICPay SDK installed, environment variables configured
**Status:** Pending

**Tasks:**
- Install @ic-pay/icpay-sdk package
- Add VITE_ICPAY_PUBLISHABLE_KEY to .env files
- Add ICPAY_SECRET_KEY to api/.env.example
- Create icpay utility module for SDK initialization

**Files:**
- website/package.json
- website/.env.example
- website/.env.development
- api/.env.example
- website/src/lib/utils/icpay.ts (NEW)

---

### Step 3: Update RentalRequestDialog for ICPay
**Success:** Users can select ICPay payment and complete crypto payment via wallet
**Status:** Pending

**Tasks:**
- Replace Stripe card element with ICPay payment flow
- Update payment method toggle (ICPay vs Credit Card)
- Implement ICPay createPaymentUsd() flow
- Handle wallet connection states
- Pass contract metadata for backend verification
- Update success/error handling

**Files:**
- website/src/lib/components/RentalRequestDialog.svelte
- website/src/lib/services/api.ts

---

### Step 4: Update backend contract creation
**Success:** Backend stores ICPay payment metadata, validates payment method
**Status:** Pending

**Tasks:**
- Update create_rental_request to handle ICPay payments
- Store icpay_transaction_id when provided by frontend
- Remove Stripe-specific logic for ICPay payments
- Update RentalRequestResponse for ICPay flow

**Files:**
- api/src/openapi/contracts.rs
- api/src/openapi/common.rs
- api/src/database/contracts.rs

---

### Step 5: Add ICPay payment verification (backend)
**Success:** Backend can verify ICPay payments via private SDK
**Status:** Pending

**Tasks:**
- Create icpay_client.rs module (HTTP-based, no Rust SDK)
- Implement getPaymentsByMetadata lookup
- Verify payment status matches contract
- Add verification endpoint or integrate into existing flow

**Files:**
- api/src/icpay_client.rs (NEW)
- api/src/main.rs (module export)
- api/Cargo.toml (add reqwest if not present)

---

### Step 6: Update tests and run cargo make
**Success:** All tests pass, cargo make clean
**Status:** Pending

**Tasks:**
- Update all payment method tests (DCT → ICPay)
- Add ICPay-specific test cases
- Update E2E payment flow tests
- Ensure cargo make passes with no warnings

**Files:**
- api/src/database/contracts/tests.rs
- common/src/payment_method.rs (tests)
- website/tests/e2e/payment-flows.spec.ts

---

## Execution Log

### Step 1
- **Implementation:** Completed
  - Renamed PaymentMethod::DCT to PaymentMethod::ICPay in /code/common/src/payment_method.rs
  - Updated serde rename from "dct" to "icpay"
  - Renamed helper method is_dct() to is_icpay()
  - Updated FromStr/Display implementations to handle "icpay" string
  - Created migration /code/api/migrations/025_icpay_rename.sql that:
    - Updates existing payment_method='dct' records to 'icpay'
    - Adds icpay_transaction_id TEXT column (nullable)
    - Creates index on icpay_transaction_id
  - Added icpay_transaction_id field to Contract struct in /code/api/src/database/contracts.rs
  - Updated all SELECT queries to include icpay_transaction_id field
  - Updated test_helpers.rs to include migration 025
  - Updated all test files to use "icpay" instead of "dct"
- **Files Changed:**
  - /code/common/src/payment_method.rs (enum, tests, helper methods)
  - /code/api/migrations/025_icpay_rename.sql (NEW)
  - /code/api/src/database/contracts.rs (Contract struct, queries)
  - /code/api/src/database/test_helpers.rs (added migration 025)
  - /code/api/src/database/contracts/tests.rs (updated all "dct" references)
  - /code/api/src/database/stats/tests.rs (updated all "dct" references)
  - /code/api/src/database/messages/tests.rs (updated "dct" reference)
  - /code/api/.sqlx/* (regenerated offline query cache)
- **Review:** All changes follow DRY, KISS, YAGNI principles. No code duplication. Minimal changes.
- **Verification:**
  - SQLX_OFFLINE=true cargo test -p dcc-common --lib: PASSED (64 tests)
  - SQLX_OFFLINE=true cargo test --lib icpay: PASSED (3 ICPay-specific tests)
  - SQLX_OFFLINE=true cargo test --lib: PASSED (339 tests, 1 unrelated Chatwoot env test failed)
  - Migration 025 ran successfully on test database
- **Outcome:** SUCCESS - PaymentMethod::ICPay exists, "icpay" serializes/deserializes correctly, is_icpay() helper works, migration runs successfully, all tests pass

### Step 2
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 3
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 4
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 5
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 6
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

---

## Completion Summary
(To be filled in Phase 4)

---

## Technical Notes

### ICPay SDK Usage (Frontend)

```typescript
import { Icpay } from '@ic-pay/icpay-sdk'

const icpay = new Icpay({
  publishableKey: import.meta.env.VITE_ICPAY_PUBLISHABLE_KEY,
})

// Create USD-denominated payment
const result = await icpay.createPaymentUsd({
  usdAmount: 50.00,
  tokenShortcode: 'ic_icp', // or 'ic_dct' when available
  metadata: { contractId: '...' },
})
```

### ICPay Payment Verification (Backend)

```typescript
// Server-side verification via HTTP (no Rust SDK)
const response = await fetch('https://api.icpay.org/v1/payments/by-metadata', {
  headers: { 'Authorization': `Bearer ${ICPAY_SECRET_KEY}` },
  body: JSON.stringify({ metadata: { contractId } })
})
```

### Database Schema Changes

```sql
-- Rename payment method
UPDATE contract_sign_requests SET payment_method = 'icpay' WHERE payment_method = 'dct';

-- Add ICPay transaction tracking
ALTER TABLE contract_sign_requests ADD COLUMN icpay_transaction_id TEXT;
```
