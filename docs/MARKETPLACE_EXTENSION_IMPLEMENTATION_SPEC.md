# Marketplace Extension: Multi-Resource Type Support

## Status

**Phase 1 Complete:** Header-based CSV parsing + GPU support implemented.

See [MARKETPLACE_GPU_EXTENSION_SPEC.md](MARKETPLACE_GPU_EXTENSION_SPEC.md) for implementation details.

---

## Objective

Enable the marketplace to support multiple resource types beyond compute/dedicated servers.

**Implemented:** GPU
**Planned:** Colocation, Validator, Storage, Container, Kubernetes, Saas, etc.

**Design Goals:**
- High development velocity for adding new types
- Works on SQLite now, PostgreSQL later
- Minimal code changes per new type
- No breaking changes to existing functionality

---

## Architecture

### What's Done

1. **Header-based CSV parsing** - Column order no longer matters, new columns can be added without breaking existing CSVs
2. **GPU fields** - `gpu_count`, `gpu_memory_gb` (extends existing `gpu_name`)
3. **Frontend GPU support** - Filter button, type-specific display

### Existing Extensibility

- `product_type` is a free-form string (any value accepted)
- Sparse schema - all type-specific columns are nullable
- Single `provider_offerings` table for all types
- Geography built-in: `datacenter_country`, `datacenter_city`, `datacenter_latitude`, `datacenter_longitude`

---

## Adding New Resource Types

Same 3-step pattern for each type:

### 1. Migration

```sql
ALTER TABLE provider_offerings ADD COLUMN new_field TYPE;
```

### 2. Rust Struct

Add `Option<T>` fields to `Offering` in `api/src/database/offerings.rs`. Update all SELECT/INSERT/UPDATE statements.

### 3. Frontend

- Add case to `formatSpecs()` in `+page.svelte`
- Add filter button if desired

No CSV parser changes needed - header-based parsing handles new columns automatically.

---

## Planned Type-Specific Fields

| Type           | Key Fields                                          |
|----------------|-----------------------------------------------------|
| **GPU** ✅      | `gpu_name`, `gpu_count`, `gpu_memory_gb`            |
| **Colocation** | `rack_units`, `power_watts`, `bandwidth_gbps`       |
| **Validator**  | `blockchain`, `commission_percent`, `minimum_stake` |
| **Storage**    | `storage_capacity_gb`, `storage_type`               |

Add fields only when implementing that type (YAGNI).

---

## Testing Checklist

- [x] Existing hardware CSV import works unchanged
- [x] Header-based CSV: columns in any order works
- [x] Header-based CSV: missing optional columns works
- [x] GPU CSV imports correctly
- [x] Frontend displays GPU-specific specs
- [x] `cargo make` clean

---

## Non-Goals

- JSON blob columns - explicit columns are type-safe and queryable
- Per-type database tables - single sparse table is simpler
- DB-level CHECK constraints - Rust validation is clearer
- Type-specific required field validation - all fields optional for flexibility
