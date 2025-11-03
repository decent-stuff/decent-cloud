# Ledger Entries API

## Overview

The `ledger_entries` endpoint provides access to committed ledger data with optional inclusion of uncommitted (next_block) entries. This is the primary method for synchronizing historical ledger data to external systems like the Cloudflare API.

## Endpoint

```candid
ledger_entries: (
    label: opt text,
    offset: opt nat32,
    limit: opt nat32,
    include_next_block: opt bool
) -> (record {
    entries: vec record {
        label: text;
        key: vec nat8;
        value: vec nat8;
    };
    has_more: bool;
    total_count: nat32;
}) query;
```

## Parameters

- **label** (optional): Filter entries by label type
  - Examples: `"ProvRegister"`, `"ProvProfile"`, `"ProvOffering"`, `"UserRegister"`, `"DCTokenTransfer"`
  - If `None`, returns all label types

- **offset** (optional): Starting position for pagination (default: 0)

- **limit** (optional): Maximum number of entries to return (default: 100)

- **include_next_block** (optional): Whether to include uncommitted entries from next_block (default: false)
  - `false`: Returns only committed ledger entries
  - `true`: Returns committed entries + uncommitted next_block entries

## Response

- **entries**: Array of ledger entries with label, key, and value
- **has_more**: Boolean indicating if more entries are available
- **total_count**: Total number of entries matching the filter

## Usage Examples

### Get all committed entries
```typescript
const result = await actor.ledger_entries([], [], [], [false]);
// or simply
const result = await actor.ledger_entries([], [], [], []);
```

### Get provider profiles with pagination
```typescript
const result = await actor.ledger_entries(
    ["ProvProfile"], // label filter
    [0],             // offset
    [50],            // limit
    [false]          // committed only
);
```

### Get all data including uncommitted
```typescript
const result = await actor.ledger_entries(
    [],      // all labels
    [0],     // from start
    [100],   // 100 entries
    [true]   // include next_block
);
```

## Comparison with next_block_entries

| Feature | `ledger_entries` | `next_block_entries` |
|---------|------------------|---------------------|
| Data source | Committed blocks | Uncommitted next_block buffer |
| Historical data | ✅ Yes | ❌ No |
| Real-time updates | ✅ With `include_next_block=true` | ✅ Yes |
| Pagination | ✅ Yes | ✅ Yes |
| Label filtering | ✅ Yes | ✅ Yes |
| Use case | Historical sync | Real-time monitoring |

## Supported Labels

- **ProvRegister**: Provider registrations
- **UserRegister**: User registrations
- **ProvProfile**: Provider profile updates
- **ProvOffering**: Provider offering updates
- **ProvCheckIn**: Provider check-ins
- **ContractSignReq**: Contract signature requests
- **ContractSignReply**: Contract signature replies
- **DCTokenTransfer**: Token transfers
- **DCTokenApproval**: Token approvals
- **RewardDistr**: Reward distributions
- **RepChange**: Reputation changes
- **RepAge**: Reputation age adjustments
- **LinkedIcIds**: Linked IC identities

## Implementation Notes

1. **Chronological Order**: Entries are returned in insertion order (chronological)

2. **Operation Filtering**: Only `Upsert` operations are returned (deleted entries are excluded)

3. **Memory Efficiency**: Pagination is recommended for large datasets to avoid memory issues

4. **Consistency**: 
   - When `include_next_block=false`: Returns consistent snapshot of committed data
   - When `include_next_block=true`: Includes latest uncommitted data (may change before commit)

## Testing

Comprehensive tests are available in `tests/test_canister.rs`:
- `test_ledger_entries_empty`: Empty ledger case
- `test_ledger_entries_with_committed_data`: Basic committed data retrieval
- `test_ledger_entries_with_next_block_included`: Including uncommitted data
- `test_ledger_entries_filter_by_label`: Label filtering
- `test_ledger_entries_pagination`: Pagination correctness
- `test_ledger_entries_pagination_with_filter`: Combined filtering and pagination
- `test_ledger_entries_comparison_with_next_block_entries`: Behavior comparison

Run tests with:
```bash
cargo test test_ledger_entries
```

## Migration from next_block_entries

If you're currently using `next_block_entries` for data synchronization, consider migrating to `ledger_entries` with `include_next_block=true` to get both historical and real-time data in a single call.

**Before:**
```typescript
// Only gets uncommitted data - misses historical entries
const result = await actor.next_block_entries([], [], []);
```

**After:**
```typescript
// Gets all data - committed + uncommitted
const result = await actor.ledger_entries([], [], [], [true]);
```
