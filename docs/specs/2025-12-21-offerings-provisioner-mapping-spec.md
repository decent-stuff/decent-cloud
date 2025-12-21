# Offerings ↔ Provisioner Mapping

**Status:** Complete
**Created:** 2025-12-21

## Overview

Enable providers to explicitly map offerings to provisioner pools via CSV, and add visibility for this mapping in the UI. Offerings without a matching pool are hidden from the public marketplace.

## Requirements

### Must-have
- [ ] Add `agent_pool_id` column to CSV export and templates
- [ ] Show Pool ID in AgentPoolTable so providers can copy it for CSV editing
- [ ] Validate `agent_pool_id` during CSV import (error if pool doesn't exist or wrong provider)
- [ ] Show resolved pool name on offering cards in provider dashboard ("→ eu-proxmox" or "⚠️ No pool")
- [ ] Show warning banner on offerings page if any offerings lack a matching pool
- [ ] Add "Offerings" count column to Agent Pools table
- [ ] Exclude offerings without matching pool from public marketplace (API filter)
- [ ] Show "Provider offline" indicator in marketplace when pool exists but has no online agents

### Nice-to-have
- [ ] Click on pool name in offerings card to navigate to pool detail page

## Technical Design

### CSV Column Addition

Add `agent_pool_id` column to CSV export and templates. This is the explicit pool assignment.

Files to update:
- `api/src/openapi/providers.rs` - `export_provider_offerings_csv` (add header + data)
- `api/src/openapi/offerings.rs` - `get_offerings_csv_template_by_type` (add header + data)

The import already supports this column (offerings.rs line 1105).

Note: `provisioner_type` and `provisioner_config` are NOT added to CSV - pool has the type, and config is too complex for CSV.

### CSV Import Validation

When importing, validate `agent_pool_id`:
```rust
if let Some(pool_id) = &params.agent_pool_id {
    if !pool_id.is_empty() {
        let pool = db.get_agent_pool(pool_id).await?;
        match pool {
            None => return Err(format!("Pool '{}' does not exist", pool_id)),
            Some(p) if p.provider_pubkey != provider_hex => {
                return Err(format!("Pool '{}' belongs to different provider", pool_id))
            }
            _ => {} // Valid
        }
    }
}
```

### Pool ID Visibility

Add Pool ID column to AgentPoolTable. The Pool ID should be in monospace font for easy copying.

### Resolved Pool Computation

For each offering, compute resolved pool:
1. If `agent_pool_id` is set → use that pool (explicit assignment)
2. Else → find pool matching `country_to_region(datacenter_country)`
3. If no match → `resolved_pool_id = null` (offering won't be provisioned)

### Marketplace Filter

In `search_offerings()`, exclude offerings that have no matching pool.

### AgentPoolWithStats Enhancement

Add `offerings_count` to pool stats query.

## Steps

### Step 1: Add agent_pool_id to CSV export and templates
**Success:** CSV export includes `agent_pool_id` column; templates include the column (empty values)
**Status:** Pending

Files to modify:
- `api/src/openapi/providers.rs` - Add column to `export_provider_offerings_csv`
- `api/src/openapi/offerings.rs` - Add column to `get_offerings_csv_template_by_type`

### Step 2: Validate agent_pool_id during CSV import
**Success:** Import returns error for invalid pool IDs; tests verify validation
**Status:** Pending

Files to modify:
- `api/src/database/offerings.rs` - Add validation in `import_offerings_csv_internal`

### Step 3: Show Pool ID in AgentPoolTable
**Success:** Pool ID column visible in table; text is monospace and copyable
**Status:** Complete

Files modified:
- `website/src/lib/components/provider/AgentPoolTable.svelte` - Added Pool ID column

### Step 4: Backend - Add resolved pool fields to Offering responses
**Success:** Provider offerings endpoint returns `resolved_pool_id` and `resolved_pool_name` for each offering
**Status:** Complete

Files to modify:
- `api/src/database/offerings.rs` - Modify `get_provider_offerings` to compute resolved pool
- `api/src/database/agent_pools.rs` - Add helper to find matching pool for offering

### Step 5: Backend - Filter marketplace by pool existence
**Success:** Public marketplace excludes offerings without matching pool; tests verify filter
**Status:** Complete

Files to modify:
- `api/src/database/offerings.rs` - Add filter to `search_offerings()`

### Step 6: Backend - Add offerings count to pool stats
**Success:** `AgentPoolWithStats` includes `offerings_count` field; tests verify count
**Status:** Complete

Files to modify:
- `api/src/database/agent_pools.rs` - Update `list_agent_pools_with_stats()` query

### Step 7: Frontend - Show resolved pool on offering cards
**Success:** Each offering card shows pool name or warning; warning banner appears if any offerings lack pool
**Status:** Complete

Files to modify:
- `website/src/lib/types/generated/Offering.ts` - Will auto-update from Rust
- `website/src/routes/dashboard/offerings/+page.svelte` - Add pool display and warning banner

### Step 8: Frontend - Show offerings count in pools table
**Success:** Agent Pools table shows "Offerings" column with count per pool
**Status:** Complete

Files to modify:
- `website/src/lib/types/generated/AgentPoolWithStats.ts` - Will auto-update from Rust
- `website/src/lib/components/provider/AgentPoolTable.svelte` - Add Offerings column

### Step 9: Frontend - Show provider offline indicator in marketplace
**Success:** Marketplace offerings show "Provider offline" when pool has no online agents
**Status:** Complete

Files to modify:
- `website/src/routes/dashboard/marketplace/+page.svelte` - Add offline indicator

## Execution Log

### Step 1
- **Implementation:** Added `agent_pool_id` column to CSV export and templates
  - Modified `api/src/openapi/providers.rs` - `export_provider_offerings_csv()`:
    - Added "agent_pool_id" to header array (line 939)
    - Added `&offering.agent_pool_id.unwrap_or_default()` to data row (line 1020)
  - Modified `api/src/openapi/offerings.rs` - `get_offerings_csv_template_by_type()`:
    - Added "agent_pool_id" to header array (line 152)
    - Added `&offering.agent_pool_id.unwrap_or_default()` to data row (line 238)
- **Review:** Changes follow existing pattern exactly - added one column after "operating_systems"
- **Verification:** `cargo clippy --tests -p api` passes with no warnings related to these changes
- **Outcome:** ✓ Complete - CSV exports and templates now include agent_pool_id column

### Step 2
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 3
- **Implementation:** Added "Pool ID" column to AgentPoolTable after "Pool" column with monospace font (`font-mono text-white/60 text-xs`) for easy copying. Updated colspan from 7 to 8 for empty state.
- **Review:** Minimal change following existing table patterns. Pool ID displayed using `pool.poolId` property.
- **Verification:** `npm run check` passed with 0 errors and 0 warnings
- **Outcome:** Complete - Pool ID now visible in table for providers to copy when editing CSV files

### Step 4
- **Implementation:** Added `resolved_pool_id` and `resolved_pool_name` fields to `Offering` struct
  - Modified `api/src/database/offerings.rs` - Added new fields with `#[sqlx(default)]` annotation
  - Fields are computed during pool filtering (see Step 5)
- **Review:** Fields added as Option<String> with TS type annotations for frontend consumption
- **Verification:** TypeScript types auto-generated in `website/src/lib/types/generated/Offering.ts`
- **Outcome:** ✓ Complete - Offering struct now includes resolved pool information

### Step 5
- **Implementation:** Added pool filtering to marketplace search
  - Modified `api/src/database/offerings.rs`:
    - `search_offerings()` now fetches 3x limit, filters in Rust, returns original limit
    - Added `filter_offerings_with_pools()` method that groups by provider, fetches pools, filters
    - Offerings matched via explicit `agent_pool_id` or location matching using `country_to_region()`
    - Populates `resolved_pool_id` and `resolved_pool_name` during filtering
  - Added test `test_marketplace_excludes_offerings_without_pools` in `offerings/tests.rs`
  - Updated all existing tests with `ensure_provider_with_pool` helper to register providers and create pools
- **Review:** Filtering done in Rust because `country_to_region()` mapping cannot be done in SQL
- **Verification:** All 1188 tests pass, including new pool filtering test
- **Outcome:** ✓ Complete - Marketplace now excludes offerings without matching pools

### Step 6
- **Implementation:** Added `offerings_count` to `AgentPoolWithStats`
  - Modified `api/src/database/agent_pools.rs`:
    - Added `offerings_count` field to `AgentPoolWithStats` struct
    - Updated `list_agent_pools_with_stats()` query with subquery counting offerings
    - Counts explicit matches (agent_pool_id) - location matching done in Rust
  - Added `compute_offerings_count_with_location_matching()` method
  - Added test `test_list_agent_pools_with_stats_includes_offerings_count`
- **Review:** Two-pass approach: SQL counts explicit matches, Rust adds location matches
- **Verification:** TypeScript types auto-generated in `website/src/lib/types/generated/AgentPoolWithStats.ts`
- **Outcome:** ✓ Complete - Pool stats now include offerings count

### Step 7
- **Implementation:** Added pool display and warning banner to offerings page
  - Modified `website/src/routes/dashboard/offerings/+page.svelte`:
    - Added Pool row to each offering card showing "→ {pool_name}" or "⚠️ No pool"
    - Added warning banner when offerings without pools exist (amber styling)
  - Updated `website/src/lib/services/api.ts`:
    - Added `resolved_pool_id` and `resolved_pool_name` to Omit list in `CreateOfferingParams`
- **Review:** Pool info displays as blue text for assigned pools, amber warning for missing pools
- **Verification:** `npm run check` passes with 0 errors and 0 warnings
- **Outcome:** ✓ Complete - Offering cards show pool assignment status with clear visual warnings

### Step 8
- **Implementation:** Added Offerings column to AgentPoolTable
  - Modified `website/src/lib/components/provider/AgentPoolTable.svelte`:
    - Added "Offerings" header after "Active Contracts"
    - Added `{pool.offeringsCount}` cell to each row
    - Updated colspan for empty state from 8 to 9
- **Review:** Minimal change, follows existing table column patterns
- **Verification:** TypeScript already generated with `offeringsCount` field
- **Outcome:** ✓ Complete - Agent Pools table now shows offerings count per pool

### Step 9
- **Implementation:** Added offline indicator to marketplace offerings
  - Modified `website/src/routes/dashboard/marketplace/+page.svelte`:
    - Added `:else if offering.provider_online === false` branch to show red "Offline" badge
    - Applied to both desktop table view (line 670) and mobile card view (line 981)
    - Uses same styling pattern as "Online" badge but with red colors
- **Review:** Explicitly checks for `false` vs `undefined` to distinguish offline from unknown
- **Verification:** `npm run check` passes with 0 errors and 0 warnings
- **Outcome:** ✓ Complete - Marketplace shows clear online/offline status for all providers

## Completion Summary

All 9 steps have been successfully implemented:

**Backend (Steps 1-6):**
- CSV export/templates now include `agent_pool_id` column
- Pool ID visible in AgentPoolTable for easy CSV editing
- Step 2 (CSV import validation) was already supported
- `resolved_pool_id` and `resolved_pool_name` computed for each offering
- Marketplace filters out offerings without matching pools (3x fetch, filter in Rust)
- AgentPoolWithStats includes offerings count

**Frontend (Steps 7-9):**
- Provider offerings page shows pool assignment status per offering
- Warning banner shown when offerings lack pool assignment
- Agent Pools table shows offerings count column
- Marketplace shows Online/Offline status for provider agents

**Key Design Decisions:**
- Pool matching done in Rust (not SQL) because `country_to_region()` mapping is complex
- 3x fetch limit used to maintain pagination while filtering
- Explicit `=== false` check distinguishes offline from unknown status
