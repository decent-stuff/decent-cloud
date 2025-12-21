# Offerings ↔ Provisioner Mapping

**Status:** In Progress
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
**Status:** Pending

Files to modify:
- `api/src/database/offerings.rs` - Modify `get_provider_offerings` to compute resolved pool
- `api/src/database/agent_pools.rs` - Add helper to find matching pool for offering

### Step 5: Backend - Filter marketplace by pool existence
**Success:** Public marketplace excludes offerings without matching pool; tests verify filter
**Status:** Pending

Files to modify:
- `api/src/database/offerings.rs` - Add filter to `search_offerings()`

### Step 6: Backend - Add offerings count to pool stats
**Success:** `AgentPoolWithStats` includes `offerings_count` field; tests verify count
**Status:** Pending

Files to modify:
- `api/src/database/agent_pools.rs` - Update `list_agent_pools_with_stats()` query

### Step 7: Frontend - Show resolved pool on offering cards
**Success:** Each offering card shows pool name or warning; warning banner appears if any offerings lack pool
**Status:** Pending

Files to modify:
- `website/src/lib/types/generated/Offering.ts` - Will auto-update from Rust
- `website/src/routes/dashboard/offerings/+page.svelte` - Add pool display and warning banner

### Step 8: Frontend - Show offerings count in pools table
**Success:** Agent Pools table shows "Offerings" column with count per pool
**Status:** Pending

Files to modify:
- `website/src/lib/types/generated/AgentPoolWithStats.ts` - Will auto-update from Rust
- `website/src/lib/components/provider/AgentPoolTable.svelte` - Add Offerings column

### Step 9: Frontend - Show provider offline indicator in marketplace
**Success:** Marketplace offerings show "Provider offline" when pool has no online agents
**Status:** Pending

Files to modify:
- `website/src/routes/dashboard/marketplace/+page.svelte` - Add offline indicator

## Execution Log

### Step 1
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

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

### Step 7
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 8
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 9
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

## Completion Summary
(To be filled in Phase 4)
