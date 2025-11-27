# Marketplace Extension: Multi-Resource Type Support

## Objective

Enable the marketplace to support 8 new resource types and easily add more in the future.

**New Types:** Colocation, SaaS, GPU, Validator, Container, Object Storage, NAS, Kubernetes

**Design Goals:**
- High development velocity for adding new types
- Works on SQLite now, PostgreSQL later
- Minimal code changes per new type
- No breaking changes to existing functionality

---

## Current State

The codebase **already supports extensibility** via:
- `product_type` is a string (not enum) - any value accepted
- Sparse schema - all hardware columns are nullable
- Single `provider_offerings` table for all types
- **Geography built-in:** `datacenter_country`, `datacenter_city`, `datacenter_latitude`, `datacenter_longitude` (with index on country)

**The blocker:** CSV import uses fixed column indices (`record.get(0)`, `record.get(1)`), requiring exactly 38 columns in exact order.

---

## Solution: Header-Based CSV Parsing

**One change unlocks everything:** Parse CSV by column names instead of indices.

Benefits:
- Type-specific CSV templates (colocation needs only ~19 columns, not 38)
- Column order independence
- Add new columns without breaking existing CSVs
- Mixed-type rows in same file

---

## Schema Design

### Common Fields (All Types)

Every offering has these fields regardless of type:

| Field | Required | Purpose |
|-------|----------|---------|
| `offering_id` | ✓ | Provider's unique ID |
| `offer_name` | ✓ | Display name |
| `product_type` | ✓ | Type discriminator |
| `currency`, `monthly_price`, `setup_fee` | ✓ | Pricing |
| `visibility`, `stock_status`, `billing_interval` | ✓ | Availability |
| `datacenter_country`, `datacenter_city` | ✓ | **Geography** |
| `datacenter_latitude`, `datacenter_longitude` | | GPS coords |
| `description`, `product_page_url`, `features` | | Details |
| `min_contract_hours`, `max_contract_hours` | | Contract terms |
| `payment_methods` | | Accepted payments |

### Type-Specific Fields

| Type | Key Fields |
|------|------------|
| **Hardware** (existing) | `processor_*`, `memory_*`, `ssd_*`, `hdd_*`, `traffic`, `uplink_speed`, etc. |
| **Colocation** | `rack_units`, `power_watts`, `network_ports`, `bandwidth_gbps` |
| **SaaS** | `software_name`, `software_version`, `user_quota`, `storage_quota_gb` |
| **GPU** | `gpu_model`, `gpu_count` (+ reuses hardware fields) |
| **Validator** | `blockchain`, `commission_percent`, `uptime_sla_percent`, `minimum_stake` |
| **Container** | `container_runtime`, `max_containers`, `registry_included` |
| **Object Storage** | `storage_class`, `max_objects`, `replication_factor` |
| **NAS** | `nas_protocol`, `max_snapshots`, `raid_level` |
| **Kubernetes** | `k8s_version`, `max_nodes`, `max_pods`, `ingress_included` |

---

## Implementation

### 1. Database Migration (`api/migrations/006_marketplace_extension.sql`)

Add nullable columns for each type:

```sql
-- Colocation (4)
ALTER TABLE provider_offerings ADD COLUMN rack_units INTEGER;
ALTER TABLE provider_offerings ADD COLUMN power_watts INTEGER;
ALTER TABLE provider_offerings ADD COLUMN network_ports INTEGER;
ALTER TABLE provider_offerings ADD COLUMN bandwidth_gbps INTEGER;

-- SaaS (4)
ALTER TABLE provider_offerings ADD COLUMN software_name TEXT;
ALTER TABLE provider_offerings ADD COLUMN software_version TEXT;
ALTER TABLE provider_offerings ADD COLUMN user_quota INTEGER;
ALTER TABLE provider_offerings ADD COLUMN storage_quota_gb INTEGER;

-- GPU (2)
ALTER TABLE provider_offerings ADD COLUMN gpu_model TEXT;
ALTER TABLE provider_offerings ADD COLUMN gpu_count INTEGER;

-- Validator (4)
ALTER TABLE provider_offerings ADD COLUMN blockchain TEXT;
ALTER TABLE provider_offerings ADD COLUMN commission_percent REAL;
ALTER TABLE provider_offerings ADD COLUMN uptime_sla_percent REAL;
ALTER TABLE provider_offerings ADD COLUMN minimum_stake TEXT;

-- Container (3)
ALTER TABLE provider_offerings ADD COLUMN container_runtime TEXT;
ALTER TABLE provider_offerings ADD COLUMN max_containers INTEGER;
ALTER TABLE provider_offerings ADD COLUMN registry_included BOOLEAN;

-- Object Storage (3)
ALTER TABLE provider_offerings ADD COLUMN storage_class TEXT;
ALTER TABLE provider_offerings ADD COLUMN max_objects INTEGER;
ALTER TABLE provider_offerings ADD COLUMN replication_factor INTEGER;

-- NAS (3)
ALTER TABLE provider_offerings ADD COLUMN nas_protocol TEXT;
ALTER TABLE provider_offerings ADD COLUMN max_snapshots INTEGER;
ALTER TABLE provider_offerings ADD COLUMN raid_level TEXT;

-- Kubernetes (4)
ALTER TABLE provider_offerings ADD COLUMN k8s_version TEXT;
ALTER TABLE provider_offerings ADD COLUMN max_nodes INTEGER;
ALTER TABLE provider_offerings ADD COLUMN max_pods INTEGER;
ALTER TABLE provider_offerings ADD COLUMN ingress_included BOOLEAN;
```

Add example offerings for each new type.

### 2. Rust Struct (`api/src/database/offerings.rs`)

Add 27 `Option<T>` fields to `Offering` struct. Update all SQL queries to include new columns.

TypeScript types auto-generate via `ts-rs`.

### 3. Header-Based CSV Parser (`api/src/database/offerings.rs`)

Replace `parse_csv_record()`:

```rust
fn parse_csv_record(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
) -> Result<Offering, String> {
    let header_map: HashMap<&str, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.trim(), i))
        .collect();

    let get = |name: &str| -> Option<&str> {
        header_map.get(name).and_then(|&idx| record.get(idx))
    };

    // Parse fields by name - missing columns return None
    // ...
}
```

Update `import_offerings_csv()` to extract headers and pass to parser.

### 4. Type-Specific Templates (`api/src/openapi/offerings.rs`)

Add endpoint: `GET /offerings/template/:product_type`

Returns CSV with only relevant columns for that type.

### 5. Frontend Updates

**Marketplace page:** Add type filter buttons, update `formatSpecs()` for type-specific display.

**API service:** Add helper to fetch type-specific templates.

---

## Validation

Type-specific validation in `create_offering()` and `update_offering()`:

```rust
fn validate_offering(offering: &Offering) -> Result<(), String> {
    match offering.product_type.as_str() {
        "colocation" => require(offering.rack_units, "rack_units")?,
        "validator" => require(offering.blockchain, "blockchain")?,
        "gpu" => {
            require(offering.gpu_model, "gpu_model")?;
            require(offering.gpu_count, "gpu_count")?;
        }
        "saas" => require(offering.software_name, "software_name")?,
        "container" => require(offering.container_runtime, "container_runtime")?,
        "object_storage" => require(offering.storage_class, "storage_class")?,
        "nas" => require(offering.nas_protocol, "nas_protocol")?,
        "kubernetes" => require(offering.k8s_version, "k8s_version")?,
        _ => {} // Unknown types allowed
    }
    Ok(())
}
```

---

## Testing Checklist

- [ ] Existing hardware CSV import works unchanged
- [ ] Header-based CSV: columns in any order works
- [ ] Header-based CSV: missing optional columns works
- [ ] New type CSVs import correctly (colocation, validator, gpu, saas)
- [ ] Type validation rejects missing required fields
- [ ] Frontend displays type-specific specs
- [ ] `cargo make` clean

---

## Adding Future Types

To add a new resource type:

1. **Migration:** Add new nullable columns
2. **Rust struct:** Add `Option<T>` fields
3. **Validation:** Add type-specific required fields (if any)
4. **Template:** Define column set for that type
5. **Frontend:** Add display formatting

No changes needed to CSV parser or API endpoints.

---

## Non-Goals

- JSON `additional_specs` column - add explicit columns when needed
- Per-type database tables - sparse single table is simpler
- DB-level CHECK constraints - Rust validation is clearer
