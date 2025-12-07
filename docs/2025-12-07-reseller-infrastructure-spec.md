# Phase 1B.2: Reseller Infrastructure

**Status:** In Progress

## Overview

Enable onboarded providers to act as resellers for seeded (external) offerings. Resellers can markup prices, receive orders, and manually fulfill them by ordering from the external provider.

## Requirements

### Must-have
- [x] Database migration for `reseller_relationships` and `reseller_orders` tables
- [x] Reseller models and database CRUD operations
- [x] Reseller API endpoints: list external providers, create/update relationships, list orders, fulfill order
- [ ] Provider dashboard "Reseller" section with external providers list
- [ ] Order fulfillment flow in dashboard
- [ ] Marketplace shows reseller offerings with commission markup

### Nice-to-have
- [ ] Reseller earnings summary/stats
- [ ] Email notification on new reseller order

## Steps

### Step 1: Database Migration
**Success:** Migration adds `reseller_relationships` and `reseller_orders` tables
**Status:** Complete

### Step 2: Reseller Models + Database Layer
**Success:** CRUD operations for reseller relationships and orders
**Status:** Complete

### Step 3: Reseller API Endpoints
**Success:** `/api/v1/reseller/*` endpoints working with tests
**Status:** Complete

### Step 4: Provider Dashboard Reseller Section
**Success:** Provider can view external providers, set commission, see orders
**Status:** Pending

### Step 5: Order Fulfillment Flow
**Success:** Provider can mark order as fulfilled with external order details
**Status:** Pending

### Step 6: Marketplace Reseller Display
**Success:** Offerings with resellers show reseller badge + commission price
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:** Created migration file `api/migrations/036_reseller_infrastructure.sql` with `reseller_relationships` and `reseller_orders` tables
- **Review:** Migration includes proper indexes on reseller_pubkey and status columns, UNIQUE constraint on (reseller_pubkey, external_provider_pubkey), and appropriate foreign key references
- **Verification:** Successfully applied migration on test database and regenerated SQLx cache
- **Outcome:** Complete - Migration ready for use

### Step 2
- **Implementation:** Created `/code/api/src/database/reseller.rs` with:
  - `ResellerRelationship` and `ResellerOrder` structs matching database schema
  - Database CRUD functions: `create_reseller_relationship`, `update_reseller_relationship`, `get_reseller_relationship`, `list_reseller_relationships_for_provider`, `delete_reseller_relationship`
  - Order management functions: `create_reseller_order`, `get_reseller_order`, `list_reseller_orders_for_provider`, `fulfill_reseller_order`
  - Commission validation (0-50% range)
- **Review:** All functions follow existing patterns in `providers.rs` and `offerings.rs`. Used `Vec<u8>` for pubkey fields, `i64` for `_e9s` and `_ns` fields, proper error handling with `anyhow::Result`
- **Verification:** Wrote solid test coverage including:
  - Create/update/delete relationship operations
  - Commission validation (negative and above-range tests)
  - Create and fulfill order workflow
  - List orders with status filtering
  - Non-existent entity error handling
- **Outcome:** Complete - Reseller database layer is functional and tested. Note: SQLx offline mode preparation deferred due to migration sync issues (not blocking for this step)

### Step 3
- **Implementation:** Created `/code/api/src/openapi/resellers.rs` with 7 API endpoints:
  - `GET /reseller/external-providers` - List available external providers
  - `POST /reseller/relationships` - Create reseller relationship
  - `PUT /reseller/relationships/:pubkey` - Update commission/status
  - `DELETE /reseller/relationships/:pubkey` - Delete relationship
  - `GET /reseller/relationships` - List provider's relationships
  - `GET /reseller/orders` - List orders (with status filter)
  - `POST /reseller/orders/:contract_id/fulfill` - Fulfill order
  - Created response types: `ResellerRelationshipResponse`, `ResellerOrderResponse` with hex-encoded pubkeys
  - Added request types in `common.rs`: `CreateResellerRelationshipRequest`, `UpdateResellerRelationshipRequest`, `FulfillResellerOrderRequest`
  - Registered API in `openapi.rs`
- **Review:** Fixed SQLx type inference issues with `as "column!"` syntax. Added test migration to test_helpers.rs. All endpoints require provider authentication.
- **Verification:** 9/9 reseller tests pass, cargo clippy clean (no new errors)
- **Outcome:** Complete - All reseller API endpoints functional

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

## Completion Summary
(Filled in Phase 4)

---

## Technical Details

### Database Schema

```sql
-- Migration: 036_reseller_infrastructure.sql

-- Reseller relationships: who can resell which external provider's offerings
CREATE TABLE IF NOT EXISTS reseller_relationships (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reseller_pubkey BLOB NOT NULL,           -- Onboarded provider acting as reseller
    external_provider_pubkey BLOB NOT NULL,  -- External provider being resold
    -- Commission settings
    commission_percent INTEGER NOT NULL DEFAULT 10,  -- 0-50%, markup on base price
    -- Status
    status TEXT NOT NULL DEFAULT 'active',   -- 'active', 'suspended'
    created_at_ns INTEGER NOT NULL,
    updated_at_ns INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(reseller_pubkey, external_provider_pubkey)
);

CREATE INDEX IF NOT EXISTS idx_reseller_relationships_reseller
    ON reseller_relationships(reseller_pubkey);
CREATE INDEX IF NOT EXISTS idx_reseller_relationships_status
    ON reseller_relationships(status);

-- Track reseller orders (contracts proxied through reseller)
CREATE TABLE IF NOT EXISTS reseller_orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL UNIQUE,        -- FK to contract
    reseller_pubkey BLOB NOT NULL,
    external_provider_pubkey BLOB NOT NULL,
    offering_id INTEGER NOT NULL,            -- Which offering was ordered
    -- Financial breakdown
    base_price_e9s INTEGER NOT NULL,         -- Original price
    commission_e9s INTEGER NOT NULL,         -- Reseller commission
    total_paid_e9s INTEGER NOT NULL,         -- What user paid
    -- External order tracking
    external_order_id TEXT,                  -- Provider's order ID
    external_order_details TEXT,             -- JSON: instance details
    -- Status
    status TEXT NOT NULL DEFAULT 'pending',  -- 'pending', 'fulfilled', 'failed'
    created_at_ns INTEGER NOT NULL,
    fulfilled_at_ns INTEGER,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_reseller_orders_reseller
    ON reseller_orders(reseller_pubkey);
CREATE INDEX IF NOT EXISTS idx_reseller_orders_status
    ON reseller_orders(status);
```

### API Endpoints

```
GET  /api/v1/reseller/external-providers
     → List external providers available for reselling

POST /api/v1/reseller/relationships
     → Create reseller relationship with commission settings

PUT  /api/v1/reseller/relationships/{external_provider_pubkey}
     → Update commission settings

DELETE /api/v1/reseller/relationships/{external_provider_pubkey}
       → Deactivate reseller relationship

GET  /api/v1/reseller/orders
     → List orders needing fulfillment

POST /api/v1/reseller/orders/{contract_id}/fulfill
     → Mark order as fulfilled with external details
```

### Frontend Routes

```
/dashboard/provider/reseller           → Main reseller dashboard
/dashboard/provider/reseller/orders    → Pending orders list
```
