# Marketplace Extension: GPU Support

## Objective

1. Make CSV parser header-based (column-order agnostic)
2. Add GPU-specific fields to support GPU/AI workload offerings

---

## Phase 1: Header-Based CSV Parser

**File:** `api/src/database/offerings.rs`

**Current problem:** `parse_csv_record()` uses fixed column indices (0-37). Adding/reordering columns breaks parsing.

**Solution:** Parse by column header name instead of index.

```rust
fn parse_csv_record(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
) -> Result<Offering, String> {
    // Build header->index map
    let col: HashMap<&str, usize> = headers.iter()
        .enumerate()
        .map(|(i, h)| (h.trim(), i))
        .collect();

    let get = |name: &str| col.get(name).and_then(|&i| record.get(i));
    // ... parse fields by name
}
```

**Changes:**
- `import_offerings_csv()` - extract headers, pass to parser
- `parse_csv_record()` - accept headers param, lookup by name
- Update tests to verify column-order independence

---

## Phase 2: GPU Schema Extension

**File:** `api/migrations/006_gpu_fields.sql`

```sql
ALTER TABLE provider_offerings ADD COLUMN gpu_count INTEGER;
ALTER TABLE provider_offerings ADD COLUMN gpu_memory_gb INTEGER;
```

**File:** `api/src/database/offerings.rs`

Add to `Offering` struct:
```rust
#[ts(type = "number | undefined")]
pub gpu_count: Option<i64>,
#[ts(type = "number | undefined")]
pub gpu_memory_gb: Option<i64>,
```

Update all SELECT statements to include new columns.

---

## Phase 3: Frontend Display

**File:** `website/src/routes/dashboard/marketplace/+page.svelte`

1. Add GPU to type filters:
```typescript
{ id: 'gpu', label: 'GPU', icon: '🎮' }
```

2. Update `formatSpecs()` to handle GPU type:
```typescript
if (type.includes('gpu')) {
    if (offering.gpu_name) specs.push(offering.gpu_name);
    if (offering.gpu_count) specs.push(`${offering.gpu_count}x`);
    if (offering.gpu_memory_gb) specs.push(`${offering.gpu_memory_gb}GB VRAM`);
}
```

---

## File Changes

| File                                                    | Change                                            |
|---------------------------------------------------------|---------------------------------------------------|
| `api/migrations/006_gpu_fields.sql`                     | Add 2 columns                                     |
| `api/src/database/offerings.rs`                         | Header-based CSV, 2 struct fields, update SELECTs |
| `api/src/openapi/offerings.rs`                          | Update CSV template with new columns              |
| `website/src/routes/dashboard/marketplace/+page.svelte` | GPU filter + formatSpecs case                     |
| `website/src/lib/types/generated/Offering.ts`           | Auto-generated                                    |

---

## Adding Future Resource Types

Same 3-step pattern:

1. **Migration:** `ALTER TABLE provider_offerings ADD COLUMN x ...`
2. **Rust:** Add `Option<T>` field to `Offering`, update SELECTs
3. **Frontend:** Add filter button, add case to `formatSpecs()`

No CSV parser changes needed - header-based parsing handles new columns automatically.

---

## Testing

```bash
cd <absolute-path-to-repo>/api && cargo clippy --benches --tests --all-features && cargo +nightly-2025-08-04 fmt --all && cargo nextest run
cd <absolute-path-to-repo>/website && npm run check && npm run test
cargo make  # Must pass

# Verify:
# 1. Existing CSV imports still work
# 2. CSV with gpu_count, gpu_memory_gb columns imports correctly
# 3. CSV with columns in different order imports correctly
# 4. GPU offerings display correctly in frontend
```
