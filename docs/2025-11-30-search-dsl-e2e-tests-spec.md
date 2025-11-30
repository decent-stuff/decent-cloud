# Search DSL E2E Tests
**Status:** In Progress

## Requirements

### Must-have
- [x] Show example offerings in marketplace (remove filter)
- [ ] Add "Demo" badge to example offerings in UI
- [ ] Disable "Rent" button for example offerings
- [ ] E2E test for DSL text search input (e.g., `price:<=100`)
- [ ] E2E test for type filter buttons (All, Compute, GPU, Storage, Network)
- [ ] E2E test for combined filters (type button + DSL query)
- [ ] E2E test for empty results state (using impossible query)
- [ ] E2E test for search results count changing based on filter

### Nice-to-have
- [ ] E2E test for invalid DSL query error handling

## Test Data Strategy

**Decision:** Show existing example offerings in marketplace, marked as "Demo"

Rationale:
- Reuses existing migration data (008_example_offerings.sql)
- Better UX - users see offerings even when marketplace is empty
- DRY - no duplicate test data to maintain
- Example offerings already have variety: compute, gpu, storage, network, dedicated

Changes needed:
1. Backend: Remove example provider filter from search queries
2. Frontend: Add "Demo" badge + disable rent for example offerings
3. API: Return `is_example` flag to frontend

## Steps

### Step 1: Show Example Offerings in Marketplace
**Success:** Example offerings appear in marketplace search results. API returns `is_example` flag.
**Status:** Complete

Files:
- `api/src/database/offerings.rs` - Remove example provider filter, add is_example field
- `api/src/openapi/offerings.rs` - Update response type if needed

### Step 2: Add Demo Badge and Disable Rent for Examples
**Success:** Example offerings show "Demo" badge, rent button is disabled with tooltip.
**Status:** Pending

Files:
- `website/src/routes/dashboard/marketplace/+page.svelte` - UI changes
- `website/src/lib/services/api.ts` - Update Offering type with is_example

### Step 3: Create E2E Tests for Search DSL
**Success:** All E2E tests pass in `search-dsl.spec.ts`:
1. Type filter buttons work (GPU shows only GPU offerings)
2. DSL text input works (price:<=50 filters results)
3. Combined filters work (type button + DSL query)
4. Empty results state shown for impossible query
5. Results count updates correctly

**Status:** Pending

Files:
- `website/tests/e2e/search-dsl.spec.ts` - E2E test file

### Step 4: Final Verification
**Success:** All E2E tests pass with `npm run test:e2e`, cargo make clean.
**Status:** Pending

## Execution Log

### Step 1: Show Example Offerings in Marketplace (2025-11-30)
**Implementation:**

Changed files:
- `/code/api/src/database/offerings.rs` - Modified database layer
- `/code/api/src/database/offerings/tests.rs` - Updated tests

Key changes:
1. Added `is_example: bool` field to `Offering` struct with TypeScript export
2. Removed example provider filter (`o.pubkey != ?`) from `search_offerings()` method
3. Removed example provider filter from `search_offerings_dsl()` method
4. Removed example provider filter from `count_offerings()` method
5. Added SQL calculation: `CASE WHEN lower(hex(o.pubkey)) = ? THEN 1 ELSE 0 END as is_example` to all SELECT queries
6. Updated all methods that return Offering to include is_example field:
   - `search_offerings()`
   - `search_offerings_dsl()`
   - `get_provider_offerings()`
   - `get_offering()`
   - `get_example_offerings()`
   - `get_example_offerings_by_type()`
7. Updated test Offering structs to include `is_example: false`
8. Updated tests to expect example offerings in results (changed from exact counts to `>=` checks)
9. Renamed tests from "excludes_private_and_example" to "excludes_private"

**Outcome:** SUCCESS
- Example offerings now appear in marketplace search results
- API correctly identifies and returns is_example flag
- Tests updated to account for example offerings in result sets
- Type safety maintained with TypeScript exports

**Notes:**
- Example provider pubkey: `6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572`
- Migration 008 provides 10 example offerings (2 compute, 2 gpu, 2 storage, 2 network, 2 dedicated)
- Kept `example_provider_pubkey()` helper function for calculating is_example field

## Completion Summary
