# Marketplace Extension Implementation Spec

**Status:** 🔴 Not Started
**Estimated Effort:** 5 days
**Start Date:** TBD
**Completion Date:** TBD

---

## Executive Summary

The current marketplace uses a **single-table approach** with 38 hardware-focused columns in `provider_offerings`. Extending this to support 4+ new resource types (Colocation, Software/SaaS, GPU Compute, Validators) would result in a sparse table with 80-100+ columns where most fields are NULL for any given offering.

**Recommendation:** **Hybrid Typed Table Architecture** - Use a unified offerings table with common fields plus JSON columns for type-specific specs, combined with type-based CSV templates and UI forms. This provides extensibility without sacrificing type safety or developer experience.

**Impact:** ~800-1200 lines of code changes, 3-5 days implementation, zero breaking changes if done correctly.

---

## 1. Current System Assessment

### 1.1 Database Schema Analysis

**Table:** `provider_offerings` (migration `001_original_schema.sql:48-92`)

**Total Columns:** 38 fields
- **Generic (11):** id, pubkey, offering_id, offer_name, description, product_page_url, currency, monthly_price, setup_fee, visibility, billing_interval
- **Hardware-Specific (27):** product_type, virtualization_type, stock_status, processor_brand, processor_amount, processor_cores, processor_speed, processor_name, memory_error_correction, memory_type, memory_amount, hdd_amount, total_hdd_capacity, ssd_amount, total_ssd_capacity, unmetered_bandwidth, uplink_speed, traffic, datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude, control_panel, gpu_name, min_contract_hours, max_contract_hours, payment_methods, features, operating_systems, created_at_ns

**Strengths:**
- Simple single-table design
- Easy to query with standard SQL
- No JOINs needed for listings
- SQLite-friendly (embedded database)

**Weaknesses:**
- **Sparse Data:** Most columns NULL for any offering (~70% empty fields)
- **No Type Safety:** Can't enforce "colocation MUST have rack_units" at DB level
- **Poor Scalability:** Adding GPU specs means 10+ new columns affecting all rows
- **Schema Confusion:** "Which fields apply to validators?" requires documentation/comments
- **Maintenance Burden:** Every new type = migration + 15-20 new columns

### 1.2 CSV Workflow Analysis

**CSV Header Definition:** Hardcoded in 2 places
- `api/src/openapi/offerings.rs:89-128` - Template generation
- `api/src/openapi/providers.rs:568-607` - Export
- `api/src/database/offerings.rs:682-803` - Import parsing

**CSV Column Count:** 38 columns (matches DB schema)

**Provider Workflow:**
1. Download template → 38-column CSV with 2 example rows
2. Edit in Excel/Google Sheets → Fill only relevant columns
3. Upload via UI → Drag-drop or file select
4. Import → Upsert mode updates existing by `offering_id`

**User Experience Issues:**
- **Cognitive Overload:** Providers see 38 columns but only need 15-20 for their type
- **Error-Prone:** Easy to fill wrong columns (e.g., `ssd_amount` for colocation)
- **No Validation:** CSV accepts any column combination, errors only at import
- **Mixed Portfolios:** Provider with hardware + colocation = single 60+ column CSV

### 1.3 Data Models & Type System

**Rust Struct:** `api/src/database/offerings.rs:7-62`

```rust
pub struct Offering {
    pub id: Option<i64>,
    pub pubkey: String,
    pub offering_id: String,
    pub offer_name: String,
    // ... 34 more fields, mostly Option<T>
}
```

**Type Safety Issues:**
- All fields optional → No compile-time guarantees
- No enum for `product_type` → String-based, typos possible
- No validation of field combinations → Can create invalid offerings
- TypeScript generation → `website/src/lib/types/generated/` receives same loose structure

**Validation:** Application-layer only
- Required fields: `offering_id`, `offer_name` (line 194-199)
- No type-specific rules enforced

### 1.4 API Layer

**Endpoints:**
- `GET /offerings/template` - Returns hardcoded CSV headers
- `GET /providers/:pubkey/offerings/export` - Exports provider's offerings as CSV
- `POST /providers/:pubkey/offerings/import?upsert=true` - Imports CSV

**CSV Parsing:** `api/src/database/offerings.rs:681-804`
- Fixed column index mapping: `get_str(0)` = offering_id, `get_str(1)` = offer_name, etc.
- No dynamic column detection
- Fails if column count < 38

**Weaknesses:**
- **Brittle Parsing:** Column order matters, can't reorder
- **No Versioning:** CSV format changes break old exports
- **No Type Detection:** Parser treats all offerings identically

### 1.5 UI Components

**Provider Dashboard:** `website/src/routes/dashboard/offerings/+page.svelte`
- Grid view of offerings with stock/visibility toggles
- CSV editor dialog (`OfferingsEditor.svelte`) with drag-drop
- Quick edit dialog for individual fields

**Marketplace:** `website/src/routes/dashboard/marketplace/+page.svelte`
- Filter by type: All, Compute, Storage, Network (hardcoded buttons)
- Card view showing specs: `formatSpecs()` hardcoded to CPU/RAM/Storage

**UI Limitations:**
- **Hardcoded Type Filters:** Adding "Colocation" requires code change
- **Generic Spec Display:** Can't show rack_units or validator commission
- **No Type-Specific Forms:** All offerings edited via CSV, no guided UI

---

### Recommended approach: Hybrid Typed Table

**Approach:** Unified table with common fields + type-discriminated optional groups + JSON for edge cases

**Schema Example (draft, not reviewed):**
```sql
CREATE TABLE provider_offerings (
    -- Identity & ownership (5 columns)
    id INTEGER PRIMARY KEY,
    pubkey BLOB NOT NULL,
    offering_id TEXT NOT NULL UNIQUE,

    -- Common to all types (15 columns)
    offer_name TEXT NOT NULL,
    description TEXT,
    product_type TEXT NOT NULL, -- 'hardware', 'colocation', 'saas', 'gpu', 'validator'
    monthly_price REAL NOT NULL,
    currency TEXT NOT NULL,
    visibility TEXT NOT NULL,
    stock_status TEXT NOT NULL,
    datacenter_country TEXT NOT NULL,
    datacenter_city TEXT NOT NULL,
    datacenter_latitude REAL,
    datacenter_longitude REAL,
    created_at_ns INTEGER NOT NULL,

    -- Hardware/GPU specs (nullable when not hardware/gpu) (12 columns)
    processor_cores INTEGER,
    memory_amount TEXT,
    storage_type TEXT, -- 'ssd' | 'hdd' | 'nvme'
    storage_amount_gb INTEGER,
    gpu_model TEXT,
    gpu_count INTEGER,
    uplink_speed_gbps INTEGER,

    -- Colocation specs (nullable when not colocation) (5 columns)
    rack_units INTEGER,
    power_watts INTEGER,
    network_ports INTEGER,

    -- Validator specs (nullable when not validator) (4 columns)
    blockchain TEXT,
    commission_percent REAL,
    uptime_percent REAL,

    -- Extensibility (2 columns)
    additional_specs TEXT, -- JSON for future types
    features TEXT -- Comma-separated tags
);

-- Validation via CHECK constraints
CREATE TRIGGER validate_hardware_offering
BEFORE INSERT ON provider_offerings
WHEN NEW.product_type IN ('hardware', 'gpu')
BEGIN
    SELECT RAISE(FAIL, 'Hardware offerings must specify processor_cores')
    WHERE NEW.processor_cores IS NULL;
END;
```

**Pros:**
- **Balanced Sparseness:** ~50 columns total (not 100+), ~30% NULL per row
- **Type-Specific Validation:** CHECK constraints/triggers enforce rules
- **Simple Queries:** Single table, optional WHERE clauses
- **Type-Specific CSVs:** Generate 20-column CSV per type programmatically
- **Mixed Portfolios Supported:** One CSV, rows differ by type
- **Future-Proof:** JSON column for new types before migration
- **Developer-Friendly:** Clear column grouping, self-documenting

**Cons:**
- **Moderate Sparseness:** Still ~30% NULL, but acceptable
- **Some Validation in App:** CHECK constraints limited, need app logic
- **Migration Required:** Add ~15 new columns (manageable)

**Balance of simplicity, type safety, and extensibility.**

**Provider UX Flow:**
1. Select offering type → Hardware | Colocation | SaaS | GPU | Validator
2. Download type-specific template → SV
3. Edit in spreadsheet → Only see relevant columns
4. Upload → Parser detects type from CSV or provider selects
5. Import → Validates type-specific rules

---

## Principles

- ✅ **YAGNI:** Implement only what's needed for the 5 target types
- ✅ **DRY:** Extract common patterns, avoid duplication
- ✅ **Tests First:** Write failing test, implement minimal code to pass
- ✅ **Atomic Commits:** Each step = working code + tests + commit
- ✅ **Update This Spec:** Mark steps complete, add learnings

---

## Target Resource Types

1. **Hardware** (existing) - VPS, Dedicated Servers, Cloud Instances
2. **Colocation** - Physical rack space rental
3. **SaaS** - Managed software/applications
4. **GPU** - GPU-accelerated compute
5. **Validator** - Blockchain validation services

---

## Phase 1: Database Foundation (Day 1)

### Step 1.1: Define Type-Specific Columns

**Status:** ⬜ Not Started

**Objective:** Document exact columns needed per type (no implementation yet)

**Tasks:**
1. Create column mapping table below
2. Validate with stakeholders if needed
3. Commit this spec update

**Column Mapping:**

```markdown
## Common Columns (All Types) - 17 total
- id, pubkey, offering_id, offer_name, description, product_page_url
- currency, monthly_price, setup_fee, visibility, stock_status, billing_interval
- datacenter_country, datacenter_city, datacenter_latitude, datacenter_longitude
- created_at_ns

## Hardware (existing) - Keep as-is - 16 specific
- processor_brand, processor_amount, processor_cores, processor_speed, processor_name
- memory_error_correction, memory_type, memory_amount
- hdd_amount, total_hdd_capacity, ssd_amount, total_ssd_capacity
- unmetered_bandwidth, uplink_speed, traffic, virtualization_type
- control_panel, min_contract_hours, max_contract_hours
- payment_methods, features, operating_systems

## Colocation (NEW) - 4 specific
- rack_units INTEGER (required)
- power_watts INTEGER (required)
- network_ports INTEGER (required)
- bandwidth_gbps INTEGER (optional)

## SaaS (NEW) - 5 specific
- software_name TEXT (required)
- software_version TEXT (optional)
- user_quota INTEGER (optional)
- storage_quota_gb INTEGER (optional)
- managed_support BOOLEAN (default FALSE)

## GPU (NEW) - Reuse existing + 2 new
- Reuse: processor_cores, memory_amount, uplink_speed
- gpu_model TEXT (required, rename from gpu_name)
- gpu_count INTEGER (required)

## Validator (NEW) - 4 specific
- blockchain TEXT (required)
- commission_percent REAL (required, 0-100)
- uptime_sla_percent REAL (optional, 0-100)
- minimum_stake TEXT (optional)

## Extensibility
- additional_specs TEXT (JSON for future types)
```

**Commit Message:**
```
docs: define type-specific columns for marketplace extension

- Map columns for 5 resource types
- Identify 15 new columns to add
- Keep existing hardware columns as-is
```

**Completion Checklist:**
- [ ] Column mapping documented
- [ ] No column conflicts identified
- [ ] Spec committed

---

### Step 1.2: Create Database Migration

**Status:** ⬜ Not Started

**Objective:** Add new columns to `provider_offerings` table

**File:** `api/migrations/006_marketplace_extension.sql`

**Implementation:**
```sql
-- Add colocation columns
ALTER TABLE provider_offerings ADD COLUMN rack_units INTEGER;
ALTER TABLE provider_offerings ADD COLUMN power_watts INTEGER;
ALTER TABLE provider_offerings ADD COLUMN network_ports INTEGER;
ALTER TABLE provider_offerings ADD COLUMN bandwidth_gbps INTEGER;

-- Add SaaS columns
ALTER TABLE provider_offerings ADD COLUMN software_name TEXT;
ALTER TABLE provider_offerings ADD COLUMN software_version TEXT;
ALTER TABLE provider_offerings ADD COLUMN user_quota INTEGER;
ALTER TABLE provider_offerings ADD COLUMN storage_quota_gb INTEGER;
ALTER TABLE provider_offerings ADD COLUMN managed_support BOOLEAN DEFAULT FALSE;

-- Add GPU columns
ALTER TABLE provider_offerings ADD COLUMN gpu_model TEXT;
ALTER TABLE provider_offerings ADD COLUMN gpu_count INTEGER;

-- Add validator columns
ALTER TABLE provider_offerings ADD COLUMN blockchain TEXT;
ALTER TABLE provider_offerings ADD COLUMN commission_percent REAL;
ALTER TABLE provider_offerings ADD COLUMN uptime_sla_percent REAL;
ALTER TABLE provider_offerings ADD COLUMN minimum_stake TEXT;

-- Add extensibility column
ALTER TABLE provider_offerings ADD COLUMN additional_specs TEXT;

-- Add example colocation offering
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description,
    currency, monthly_price, setup_fee, visibility, product_type,
    stock_status, billing_interval,
    datacenter_country, datacenter_city,
    rack_units, power_watts, network_ports,
    created_at_ns
) VALUES (
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'colo-basic-001',
    'Quarter Rack Colocation',
    '10U rack space with 1kW power and 1Gbps network',
    'USD', 299.99, 99.00, 'example', 'colocation',
    'in_stock', 'monthly',
    'US', 'New York',
    10, 1000, 1,
    1609459200000000000
);

-- Add example validator offering
INSERT INTO provider_offerings (
    pubkey, offering_id, offer_name, description,
    currency, monthly_price, setup_fee, visibility, product_type,
    stock_status, billing_interval,
    datacenter_country, datacenter_city,
    blockchain, commission_percent, uptime_sla_percent,
    created_at_ns
) VALUES (
    x'6578616d706c652d6f66666572696e672d70726f76696465722d6964656e746966696572',
    'val-icp-001',
    'ICP Validator Node',
    'High-uptime Internet Computer validator with 5% commission',
    'USD', 149.99, 0.00, 'example', 'validator',
    'in_stock', 'monthly',
    'DE', 'Frankfurt',
    'Internet Computer', 5.0, 99.9,
    1609459200000000000
);
```

**Testing:**
```bash
# Run migration
cd api
DATABASE_URL="sqlite:./test.db" sqlx migrate run

# Verify columns exist
sqlite3 test.db "PRAGMA table_info(provider_offerings);" | grep -E "(rack_units|blockchain|gpu_model)"

# Verify example offerings
sqlite3 test.db "SELECT offering_id, product_type FROM provider_offerings WHERE product_type IN ('colocation', 'validator');"
```

**Commit Message:**
```
feat(db): add columns for colocation, saas, gpu, validator types

- Add 15 new columns to provider_offerings
- Add example colocation offering
- Add example validator offering
- Backwards compatible (existing offerings unaffected)
```

**Completion Checklist:**
- [ ] Migration file created
- [ ] Migration runs successfully on fresh DB
- [ ] Existing hardware offerings still queryable
- [ ] Example offerings inserted
- [ ] Committed with passing tests

---

### Step 1.3: Update Rust Offering Struct

**Status:** ⬜ Not Started

**Objective:** Add new fields to `Offering` struct

**File:** `api/src/database/offerings.rs`

**Implementation:**
```rust
// In Offering struct (line 7-62), add new fields:

// Colocation fields
#[ts(type = "number | undefined")]
#[oai(skip_serializing_if_is_none)]
pub rack_units: Option<i64>,
#[ts(type = "number | undefined")]
#[oai(skip_serializing_if_is_none)]
pub power_watts: Option<i64>,
#[ts(type = "number | undefined")]
#[oai(skip_serializing_if_is_none)]
pub network_ports: Option<i64>,
#[ts(type = "number | undefined")]
#[oai(skip_serializing_if_is_none)]
pub bandwidth_gbps: Option<i64>,

// SaaS fields
pub software_name: Option<String>,
pub software_version: Option<String>,
#[ts(type = "number | undefined")]
#[oai(skip_serializing_if_is_none)]
pub user_quota: Option<i64>,
#[ts(type = "number | undefined")]
#[oai(skip_serializing_if_is_none)]
pub storage_quota_gb: Option<i64>,
pub managed_support: bool,

// GPU fields
pub gpu_model: Option<String>,
#[ts(type = "number | undefined")]
#[oai(skip_serializing_if_is_none)]
pub gpu_count: Option<i64>,

// Validator fields
pub blockchain: Option<String>,
pub commission_percent: Option<f64>,
pub uptime_sla_percent: Option<f64>,
pub minimum_stake: Option<String>,

// Extensibility
pub additional_specs: Option<String>,
```

**Update all SQL queries** to include new columns:
- `search_offerings()` - line 80
- `get_provider_offerings()` - line 118
- `get_offering()` - line 137
- `get_example_offerings()` - line 159
- `create_offering()` - line 266
- `update_offering()` - line 408
- `parse_csv_record()` - line 682

**YAGNI Note:** Don't add validation yet. Just make fields queryable.

**Testing:**
```bash
# Regenerate TypeScript types
cd api
cargo build --release

# Verify TypeScript types generated
cat ../website/src/lib/types/generated/Offering.ts | grep -E "(rack_units|blockchain|gpu_model)"

# Run existing tests (should still pass)
cargo test offerings
```

**Commit Message:**
```
feat(api): add new fields to Offering struct

- Add colocation, saas, gpu, validator fields
- Update all SQL queries to include new columns
- Regenerate TypeScript types
- All existing tests pass (backwards compatible)
```

**Completion Checklist:**
- [ ] 15 new fields added to struct
- [ ] All SQL queries updated
- [ ] TypeScript types regenerated
- [ ] `cargo test` passes
- [ ] Committed

---

## Phase 2: Type-Specific CSV Templates (Day 2)

### Step 2.1: Extract CSV Column Definitions

**Status:** ⬜ Not Started

**Objective:** DRY up hardcoded CSV headers, create reusable column lists

**File:** `api/src/csv_schema.rs` (NEW)

**Implementation:**
```rust
//! CSV schema definitions for type-specific offerings

/// Common columns for all offering types
pub const COMMON_COLUMNS: &[&str] = &[
    "offering_id",
    "offer_name",
    "description",
    "product_page_url",
    "currency",
    "monthly_price",
    "setup_fee",
    "visibility",
    "product_type",
    "billing_interval",
    "stock_status",
    "datacenter_country",
    "datacenter_city",
    "datacenter_latitude",
    "datacenter_longitude",
];

/// Hardware-specific columns
pub const HARDWARE_COLUMNS: &[&str] = &[
    "processor_brand",
    "processor_amount",
    "processor_cores",
    "processor_speed",
    "processor_name",
    "memory_error_correction",
    "memory_type",
    "memory_amount",
    "hdd_amount",
    "total_hdd_capacity",
    "ssd_amount",
    "total_ssd_capacity",
    "unmetered_bandwidth",
    "uplink_speed",
    "traffic",
    "virtualization_type",
    "control_panel",
    "min_contract_hours",
    "max_contract_hours",
    "payment_methods",
    "features",
    "operating_systems",
];

/// Colocation-specific columns
pub const COLOCATION_COLUMNS: &[&str] = &[
    "rack_units",
    "power_watts",
    "network_ports",
    "bandwidth_gbps",
    "features",
];

/// SaaS-specific columns
pub const SAAS_COLUMNS: &[&str] = &[
    "software_name",
    "software_version",
    "user_quota",
    "storage_quota_gb",
    "managed_support",
    "features",
];

/// GPU-specific columns
pub const GPU_COLUMNS: &[&str] = &[
    "processor_cores",
    "memory_amount",
    "gpu_model",
    "gpu_count",
    "uplink_speed",
    "features",
    "operating_systems",
];

/// Validator-specific columns
pub const VALIDATOR_COLUMNS: &[&str] = &[
    "blockchain",
    "commission_percent",
    "uptime_sla_percent",
    "minimum_stake",
    "features",
];

/// Get columns for a specific product type
pub fn columns_for_type(product_type: &str) -> Vec<&'static str> {
    let mut cols = COMMON_COLUMNS.to_vec();

    match product_type {
        "hardware" => cols.extend_from_slice(HARDWARE_COLUMNS),
        "colocation" => cols.extend_from_slice(COLOCATION_COLUMNS),
        "saas" => cols.extend_from_slice(SAAS_COLUMNS),
        "gpu" => cols.extend_from_slice(GPU_COLUMNS),
        "validator" => cols.extend_from_slice(VALIDATOR_COLUMNS),
        _ => {
            // Unknown type: return all columns
            cols.extend_from_slice(HARDWARE_COLUMNS);
            cols.extend_from_slice(COLOCATION_COLUMNS);
            cols.extend_from_slice(SAAS_COLUMNS);
            cols.extend_from_slice(GPU_COLUMNS);
            cols.extend_from_slice(VALIDATOR_COLUMNS);
            cols.dedup();
        }
    }

    cols
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_columns() {
        let cols = columns_for_type("hardware");
        assert!(cols.contains(&"offering_id")); // common
        assert!(cols.contains(&"processor_cores")); // hardware
        assert!(!cols.contains(&"rack_units")); // not colocation
    }

    #[test]
    fn test_colocation_columns() {
        let cols = columns_for_type("colocation");
        assert!(cols.contains(&"offering_id")); // common
        assert!(cols.contains(&"rack_units")); // colocation
        assert!(!cols.contains(&"processor_cores")); // not hardware
    }
}
```

**Update `api/src/lib.rs`:**
```rust
pub mod csv_schema;
```

**Testing:**
```bash
cargo test csv_schema
```

**Commit Message:**
```
refactor(api): extract CSV column definitions

- Create csv_schema module with type-specific columns
- DRY up hardcoded headers
- Add tests for column selection
```

**Completion Checklist:**
- [ ] csv_schema.rs created
- [ ] Tests pass
- [ ] Committed

---

### Step 2.2: Implement Type-Specific Template Endpoint

**Status:** ⬜ Not Started

**Objective:** Add `GET /offerings/template/:type` endpoint

**File:** `api/src/openapi/offerings.rs`

**Implementation:**
```rust
use crate::csv_schema;

// Add new endpoint after get_offerings_csv_template (line 206)

/// Get CSV template for specific offering type
///
/// Returns a type-specific CSV template with example offerings
#[oai(
    path = "/offerings/template/:product_type",
    method = "get",
    tag = "ApiTags::Offerings"
)]
async fn get_typed_csv_template(
    &self,
    db: Data<&Arc<Database>>,
    product_type: Path<String>,
) -> poem_openapi::payload::PlainText<String> {
    let product_type = product_type.0.as_str();
    let columns = csv_schema::columns_for_type(product_type);

    let mut csv_writer = csv::Writer::from_writer(vec![]);

    // Write header
    if csv_writer.write_record(&columns).is_err() {
        return poem_openapi::payload::PlainText("Error generating CSV".to_string());
    }

    // Get example offerings of this type
    if let Ok(all_examples) = db.get_example_offerings().await {
        let typed_examples: Vec<_> = all_examples
            .into_iter()
            .filter(|o| o.product_type == product_type)
            .collect();

        for offering in typed_examples {
            let row = build_csv_row(&offering, &columns);
            let _ = csv_writer.write_record(row);
        }
    }

    match csv_writer.into_inner() {
        Ok(csv_data) => {
            poem_openapi::payload::PlainText(String::from_utf8_lossy(&csv_data).to_string())
        }
        Err(e) => poem_openapi::payload::PlainText(format!("CSV generation error: {}", e)),
    }
}

/// Build CSV row from offering based on column list
fn build_csv_row(offering: &crate::database::offerings::Offering, columns: &[&str]) -> Vec<String> {
    columns
        .iter()
        .map(|col| get_offering_field(offering, col))
        .collect()
}

/// Get field value from offering by column name
fn get_offering_field(offering: &crate::database::offerings::Offering, col: &str) -> String {
    match col {
        "offering_id" => offering.offering_id.clone(),
        "offer_name" => offering.offer_name.clone(),
        "description" => offering.description.clone().unwrap_or_default(),
        "product_page_url" => offering.product_page_url.clone().unwrap_or_default(),
        "currency" => offering.currency.clone(),
        "monthly_price" => offering.monthly_price.to_string(),
        "setup_fee" => offering.setup_fee.to_string(),
        "visibility" => offering.visibility.clone(),
        "product_type" => offering.product_type.clone(),
        "billing_interval" => offering.billing_interval.clone(),
        "stock_status" => offering.stock_status.clone(),
        "datacenter_country" => offering.datacenter_country.clone(),
        "datacenter_city" => offering.datacenter_city.clone(),
        "datacenter_latitude" => offering.datacenter_latitude.map(|v| v.to_string()).unwrap_or_default(),
        "datacenter_longitude" => offering.datacenter_longitude.map(|v| v.to_string()).unwrap_or_default(),

        // Hardware
        "processor_brand" => offering.processor_brand.clone().unwrap_or_default(),
        "processor_amount" => offering.processor_amount.map(|v| v.to_string()).unwrap_or_default(),
        "processor_cores" => offering.processor_cores.map(|v| v.to_string()).unwrap_or_default(),
        "processor_speed" => offering.processor_speed.clone().unwrap_or_default(),
        "processor_name" => offering.processor_name.clone().unwrap_or_default(),
        "memory_error_correction" => offering.memory_error_correction.clone().unwrap_or_default(),
        "memory_type" => offering.memory_type.clone().unwrap_or_default(),
        "memory_amount" => offering.memory_amount.clone().unwrap_or_default(),
        "hdd_amount" => offering.hdd_amount.map(|v| v.to_string()).unwrap_or_default(),
        "total_hdd_capacity" => offering.total_hdd_capacity.clone().unwrap_or_default(),
        "ssd_amount" => offering.ssd_amount.map(|v| v.to_string()).unwrap_or_default(),
        "total_ssd_capacity" => offering.total_ssd_capacity.clone().unwrap_or_default(),
        "unmetered_bandwidth" => offering.unmetered_bandwidth.to_string(),
        "uplink_speed" => offering.uplink_speed.clone().unwrap_or_default(),
        "traffic" => offering.traffic.map(|v| v.to_string()).unwrap_or_default(),
        "virtualization_type" => offering.virtualization_type.clone().unwrap_or_default(),
        "control_panel" => offering.control_panel.clone().unwrap_or_default(),
        "min_contract_hours" => offering.min_contract_hours.map(|v| v.to_string()).unwrap_or_default(),
        "max_contract_hours" => offering.max_contract_hours.map(|v| v.to_string()).unwrap_or_default(),
        "payment_methods" => offering.payment_methods.clone().unwrap_or_default(),
        "features" => offering.features.clone().unwrap_or_default(),
        "operating_systems" => offering.operating_systems.clone().unwrap_or_default(),

        // Colocation
        "rack_units" => offering.rack_units.map(|v| v.to_string()).unwrap_or_default(),
        "power_watts" => offering.power_watts.map(|v| v.to_string()).unwrap_or_default(),
        "network_ports" => offering.network_ports.map(|v| v.to_string()).unwrap_or_default(),
        "bandwidth_gbps" => offering.bandwidth_gbps.map(|v| v.to_string()).unwrap_or_default(),

        // SaaS
        "software_name" => offering.software_name.clone().unwrap_or_default(),
        "software_version" => offering.software_version.clone().unwrap_or_default(),
        "user_quota" => offering.user_quota.map(|v| v.to_string()).unwrap_or_default(),
        "storage_quota_gb" => offering.storage_quota_gb.map(|v| v.to_string()).unwrap_or_default(),
        "managed_support" => offering.managed_support.to_string(),

        // GPU
        "gpu_model" => offering.gpu_model.clone().unwrap_or_default(),
        "gpu_count" => offering.gpu_count.map(|v| v.to_string()).unwrap_or_default(),

        // Validator
        "blockchain" => offering.blockchain.clone().unwrap_or_default(),
        "commission_percent" => offering.commission_percent.map(|v| v.to_string()).unwrap_or_default(),
        "uptime_sla_percent" => offering.uptime_sla_percent.map(|v| v.to_string()).unwrap_or_default(),
        "minimum_stake" => offering.minimum_stake.clone().unwrap_or_default(),

        _ => String::new(),
    }
}
```

**Testing:**
```bash
# Build and run API
cargo build --release

# Test endpoints (once API is running)
curl http://localhost:3000/api/v1/offerings/template/hardware
curl http://localhost:3000/api/v1/offerings/template/colocation
curl http://localhost:3000/api/v1/offerings/template/validator

# Verify column counts
curl http://localhost:3000/api/v1/offerings/template/colocation | head -1 | tr ',' '\n' | wc -l
# Should be ~20 columns, not 50+
```

**Commit Message:**
```
feat(api): add type-specific CSV template endpoint

- Add GET /offerings/template/:type
- Returns only relevant columns per type
- Includes example offerings for each type
- Hardware: 37 cols, Colocation: 20 cols, Validator: 19 cols
```

**Completion Checklist:**
- [ ] Endpoint implemented
- [ ] Templates return correct columns
- [ ] Example offerings included
- [ ] API tests pass
- [ ] Committed

---

### Step 2.3: Update Export to Use Column Schema

**Status:** ⬜ Not Started

**Objective:** Refactor export endpoint to use shared column logic

**File:** `api/src/openapi/providers.rs`

**Implementation:**
```rust
// Update export_provider_offerings_csv (line 548)
// Replace hardcoded header with dynamic column selection

async fn export_provider_offerings_csv(
    &self,
    db: Data<&Arc<Database>>,
    auth: ApiAuthenticatedUser,
    pubkey: Path<String>,
    // NEW: Optional type filter
    product_type: poem_openapi::param::Query<Option<String>>,
) -> poem_openapi::payload::PlainText<String> {
    let pubkey_bytes = match decode_pubkey(&pubkey.0) {
        Ok(pk) => pk,
        Err(_) => return poem_openapi::payload::PlainText("Invalid pubkey format".to_string()),
    };

    if check_authorization(&pubkey_bytes, &auth).is_err() {
        return poem_openapi::payload::PlainText("Unauthorized".to_string());
    }

    match db.get_provider_offerings(&pubkey_bytes).await {
        Ok(mut offerings) => {
            // Filter by type if requested
            if let Some(ref ptype) = product_type.0 {
                offerings.retain(|o| &o.product_type == ptype);
            }

            // Determine columns based on offerings
            let columns = if let Some(ptype) = product_type.0 {
                crate::csv_schema::columns_for_type(&ptype)
            } else {
                // Mixed types: use all columns
                crate::csv_schema::columns_for_type("")
            };

            let mut csv_writer = csv::Writer::from_writer(vec![]);

            // Write header
            let _ = csv_writer.write_record(&columns);

            // Write data rows
            for offering in offerings {
                let row = crate::openapi::offerings::build_csv_row(&offering, &columns);
                let _ = csv_writer.write_record(row);
            }

            match csv_writer.into_inner() {
                Ok(csv_data) => poem_openapi::payload::PlainText(
                    String::from_utf8_lossy(&csv_data).to_string(),
                ),
                Err(e) => {
                    poem_openapi::payload::PlainText(format!("CSV generation error: {}", e))
                }
            }
        }
        Err(e) => poem_openapi::payload::PlainText(format!("Error: {}", e)),
    }
}
```

**Make `build_csv_row` public** in `offerings.rs`:
```rust
pub fn build_csv_row(...) -> Vec<String> { ... }
pub fn get_offering_field(...) -> String { ... }
```

**Testing:**
```bash
# Export all offerings (mixed CSV)
curl -H "Authorization: ..." http://localhost:3000/api/v1/providers/{pubkey}/offerings/export

# Export only colocation (type-specific CSV)
curl -H "Authorization: ..." http://localhost:3000/api/v1/providers/{pubkey}/offerings/export?product_type=colocation
```

**Commit Message:**
```
refactor(api): use shared CSV schema for export

- Export endpoint now uses csv_schema module
- Add optional product_type filter to export
- DRY: share build_csv_row between template and export
```

**Completion Checklist:**
- [ ] Export uses shared schema
- [ ] Type filtering works
- [ ] Tests pass
- [ ] Committed

---

## Phase 3: Dynamic CSV Import (Day 3)

### Step 3.1: Implement Dynamic CSV Parser

**Status:** ⬜ Not Started

**Objective:** Parse CSV by header names, not fixed column indices

**File:** `api/src/database/offerings.rs`

**Implementation:**
```rust
// Replace parse_csv_record (line 682) with header-based parsing

/// Parse a single CSV record into Offering using header mapping
fn parse_csv_record_dynamic(
    record: &csv::StringRecord,
    headers: &csv::StringRecord,
) -> Result<Offering, String> {
    // Create header -> index map
    let header_map: std::collections::HashMap<&str, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h, i))
        .collect();

    // Helper to get value by header name
    let get_by_header = |name: &str| -> Option<&str> {
        header_map.get(name).and_then(|&idx| record.get(idx))
    };

    let get_str = |name: &str| -> String {
        get_by_header(name).unwrap_or("").to_string()
    };

    let get_opt_str = |name: &str| -> Option<String> {
        get_by_header(name).and_then(|v| {
            let trimmed = v.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
    };

    let get_opt_i64 = |name: &str| -> Option<i64> {
        get_by_header(name).and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<i64>().ok()
            }
        })
    };

    let get_opt_f64 = |name: &str| -> Option<f64> {
        get_by_header(name).and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<f64>().ok()
            }
        })
    };

    let get_f64 = |name: &str| -> Result<f64, String> {
        get_by_header(name)
            .ok_or_else(|| format!("Missing column {}", name))?
            .trim()
            .parse::<f64>()
            .map_err(|_| format!("Invalid number in column {}", name))
    };

    let get_bool = |name: &str| -> bool {
        get_by_header(name)
            .map(|s| {
                let lower = s.trim().to_lowercase();
                lower == "true" || lower == "1" || lower == "yes"
            })
            .unwrap_or(false)
    };

    // Required fields validation
    let offering_id = get_str("offering_id");
    let offer_name = get_str("offer_name");

    if offering_id.trim().is_empty() {
        return Err("offering_id is required".to_string());
    }
    if offer_name.trim().is_empty() {
        return Err("offer_name is required".to_string());
    }

    Ok(Offering {
        id: None,
        pubkey: String::new(),
        offering_id,
        offer_name,
        description: get_opt_str("description"),
        product_page_url: get_opt_str("product_page_url"),
        currency: get_str("currency"),
        monthly_price: get_f64("monthly_price")?,
        setup_fee: get_f64("setup_fee").unwrap_or(0.0),
        visibility: get_str("visibility"),
        product_type: get_str("product_type"),
        billing_interval: get_str("billing_interval"),
        stock_status: get_str("stock_status"),
        datacenter_country: get_str("datacenter_country"),
        datacenter_city: get_str("datacenter_city"),
        datacenter_latitude: get_opt_f64("datacenter_latitude"),
        datacenter_longitude: get_opt_f64("datacenter_longitude"),

        // Hardware fields
        processor_brand: get_opt_str("processor_brand"),
        processor_amount: get_opt_i64("processor_amount"),
        processor_cores: get_opt_i64("processor_cores"),
        processor_speed: get_opt_str("processor_speed"),
        processor_name: get_opt_str("processor_name"),
        memory_error_correction: get_opt_str("memory_error_correction"),
        memory_type: get_opt_str("memory_type"),
        memory_amount: get_opt_str("memory_amount"),
        hdd_amount: get_opt_i64("hdd_amount"),
        total_hdd_capacity: get_opt_str("total_hdd_capacity"),
        ssd_amount: get_opt_i64("ssd_amount"),
        total_ssd_capacity: get_opt_str("total_ssd_capacity"),
        unmetered_bandwidth: get_bool("unmetered_bandwidth"),
        uplink_speed: get_opt_str("uplink_speed"),
        traffic: get_opt_i64("traffic"),
        virtualization_type: get_opt_str("virtualization_type"),
        control_panel: get_opt_str("control_panel"),
        gpu_name: get_opt_str("gpu_name"), // Legacy, keep for backwards compat
        min_contract_hours: get_opt_i64("min_contract_hours"),
        max_contract_hours: get_opt_i64("max_contract_hours"),
        payment_methods: get_opt_str("payment_methods"),
        features: get_opt_str("features"),
        operating_systems: get_opt_str("operating_systems"),

        // Colocation fields
        rack_units: get_opt_i64("rack_units"),
        power_watts: get_opt_i64("power_watts"),
        network_ports: get_opt_i64("network_ports"),
        bandwidth_gbps: get_opt_i64("bandwidth_gbps"),

        // SaaS fields
        software_name: get_opt_str("software_name"),
        software_version: get_opt_str("software_version"),
        user_quota: get_opt_i64("user_quota"),
        storage_quota_gb: get_opt_i64("storage_quota_gb"),
        managed_support: get_bool("managed_support"),

        // GPU fields
        gpu_model: get_opt_str("gpu_model"),
        gpu_count: get_opt_i64("gpu_count"),

        // Validator fields
        blockchain: get_opt_str("blockchain"),
        commission_percent: get_opt_f64("commission_percent"),
        uptime_sla_percent: get_opt_f64("uptime_sla_percent"),
        minimum_stake: get_opt_str("minimum_stake"),

        // Extensibility
        additional_specs: get_opt_str("additional_specs"),
    })
}
```

**Update `import_offerings_csv` to use new parser:**
```rust
pub async fn import_offerings_csv(
    &self,
    pubkey: &[u8],
    csv_data: &str,
    upsert: bool,
) -> Result<(usize, Vec<(usize, String)>)> {
    let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
    let mut success_count = 0;
    let mut errors = Vec::new();

    // Get headers
    let headers = match reader.headers() {
        Ok(h) => h.clone(),
        Err(e) => return Err(anyhow::anyhow!("Failed to read CSV headers: {}", e)),
    };

    for (row_idx, result) in reader.records().enumerate() {
        let row_number = row_idx + 2; // +2 because row 1 is header, 0-indexed

        match result {
            Ok(record) => {
                match Self::parse_csv_record_dynamic(&record, &headers) {
                    Ok(params) => {
                        let result: Result<()> = if upsert {
                            // ... existing upsert logic ...
                        } else {
                            self.create_offering(pubkey, params).await.map(|_| ())
                        };

                        match result {
                            Ok(_) => success_count += 1,
                            Err(e) => errors.push((row_number, e.to_string())),
                        }
                    }
                    Err(e) => errors.push((row_number, e)),
                }
            }
            Err(e) => errors.push((row_number, format!("CSV parse error: {}", e))),
        }
    }

    Ok((success_count, errors))
}
```

**Testing:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hardware_csv() {
        let headers = csv::StringRecord::from(vec![
            "offering_id", "offer_name", "product_type", "monthly_price",
            "currency", "processor_cores", "memory_amount"
        ]);
        let record = csv::StringRecord::from(vec![
            "hw-001", "Test Server", "hardware", "29.99", "USD", "4", "8GB"
        ]);

        let offering = parse_csv_record_dynamic(&record, &headers).unwrap();
        assert_eq!(offering.offering_id, "hw-001");
        assert_eq!(offering.processor_cores, Some(4));
        assert_eq!(offering.rack_units, None); // Not in CSV
    }

    #[test]
    fn test_parse_colocation_csv() {
        let headers = csv::StringRecord::from(vec![
            "offering_id", "offer_name", "product_type", "monthly_price",
            "currency", "rack_units", "power_watts"
        ]);
        let record = csv::StringRecord::from(vec![
            "colo-001", "Rack Space", "colocation", "299.99", "USD", "10", "1000"
        ]);

        let offering = parse_csv_record_dynamic(&record, &headers).unwrap();
        assert_eq!(offering.rack_units, Some(10));
        assert_eq!(offering.processor_cores, None); // Not in CSV
    }
}
```

**Commit Message:**
```
feat(api): implement dynamic header-based CSV parser

- Parse CSV by header names, not fixed indices
- Support variable column order
- Handle missing optional columns gracefully
- Add tests for hardware and colocation CSVs
```

**Completion Checklist:**
- [ ] Dynamic parser implemented
- [ ] Import uses new parser
- [ ] Tests pass for all types
- [ ] Backwards compatible with old CSVs
- [ ] Committed

---

### Step 3.2: Add Type-Specific Validation

**Status:** ⬜ Not Started

**Objective:** Validate required fields per type

**File:** `api/src/database/offerings.rs`

**Implementation:**
```rust
/// Validate offering based on product type
fn validate_offering(offering: &Offering) -> Result<(), String> {
    match offering.product_type.as_str() {
        "hardware" => {
            if offering.processor_cores.is_none() {
                return Err("Hardware offerings must specify processor_cores".to_string());
            }
            if offering.memory_amount.is_none() {
                return Err("Hardware offerings must specify memory_amount".to_string());
            }
        }
        "colocation" => {
            if offering.rack_units.is_none() {
                return Err("Colocation offerings must specify rack_units".to_string());
            }
            if offering.power_watts.is_none() {
                return Err("Colocation offerings must specify power_watts".to_string());
            }
        }
        "saas" => {
            if offering.software_name.is_none() {
                return Err("SaaS offerings must specify software_name".to_string());
            }
        }
        "gpu" => {
            if offering.gpu_model.is_none() {
                return Err("GPU offerings must specify gpu_model".to_string());
            }
            if offering.gpu_count.is_none() {
                return Err("GPU offerings must specify gpu_count".to_string());
            }
        }
        "validator" => {
            if offering.blockchain.is_none() {
                return Err("Validator offerings must specify blockchain".to_string());
            }
            if offering.commission_percent.is_none() {
                return Err("Validator offerings must specify commission_percent".to_string());
            }
        }
        _ => {
            // Unknown type: allow for extensibility
        }
    }

    Ok(())
}
```

**Call validation in `create_offering`:**
```rust
pub async fn create_offering(&self, pubkey: &[u8], params: Offering) -> Result<i64> {
    // Existing validations...

    // Type-specific validation
    validate_offering(&params)?;

    // ... rest of function
}
```

**Testing:**
```rust
#[test]
fn test_validate_colocation_missing_required() {
    let offering = Offering {
        product_type: "colocation".to_string(),
        rack_units: None, // Missing!
        power_watts: Some(1000),
        // ... other fields
    };

    let result = validate_offering(&offering);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("rack_units"));
}

#[test]
fn test_validate_colocation_valid() {
    let offering = Offering {
        product_type: "colocation".to_string(),
        rack_units: Some(10),
        power_watts: Some(1000),
        // ... other fields
    };

    assert!(validate_offering(&offering).is_ok());
}
```

**Commit Message:**
```
feat(api): add type-specific validation

- Validate required fields per product type
- Clear error messages for missing fields
- Extensible for unknown types
- Tests cover all 5 types
```

**Completion Checklist:**
- [ ] Validation function implemented
- [ ] All types validated
- [ ] Tests pass
- [ ] Committed

---

## Phase 4: Frontend Integration (Days 4-5)

### Step 4.1: Update Frontend API Client

**Status:** ⬜ Not Started

**Objective:** Add methods to fetch type-specific templates

**File:** `website/src/lib/services/api.ts`

**Implementation:**
```typescript
// Add new function
export async function fetchCSVTemplateForType(productType: string): Promise<string> {
	const response = await fetch(`${API_BASE_URL}/offerings/template/${productType}`);
	if (!response.ok) {
		throw new Error(`Failed to fetch ${productType} template: ${response.statusText}`);
	}
	return await response.text();
}

// Update existing fetchCSVTemplate to use 'hardware' by default
export async function fetchCSVTemplate(): Promise<string> {
	return fetchCSVTemplateForType('hardware');
}

// Add type list constant
export const PRODUCT_TYPES = [
	{ value: 'hardware', label: 'Hardware (VPS/Dedicated)', icon: '💻' },
	{ value: 'colocation', label: 'Colocation (Rack Space)', icon: '🏢' },
	{ value: 'saas', label: 'SaaS (Managed Software)', icon: '☁️' },
	{ value: 'gpu', label: 'GPU Compute', icon: '🎮' },
	{ value: 'validator', label: 'Validator (Blockchain)', icon: '⛓️' }
] as const;

export type ProductType = typeof PRODUCT_TYPES[number]['value'];
```

**Testing:**
```bash
cd website
npm run check  # TypeScript type checking
```

**Commit Message:**
```
feat(web): add type-specific template API methods

- Add fetchCSVTemplateForType()
- Define PRODUCT_TYPES constant
- Export ProductType type
```

**Completion Checklist:**
- [ ] API methods added
- [ ] Types exported
- [ ] TypeScript checks pass
- [ ] Committed

---

### Step 4.2: Add Type Selector to Offerings Dashboard

**Status:** ⬜ Not Started

**Objective:** Let providers choose offering type before downloading template

**File:** `website/src/routes/dashboard/offerings/+page.svelte`

**Implementation:**
```svelte
<script lang="ts">
	import { PRODUCT_TYPES, fetchCSVTemplateForType, type ProductType } from '$lib/services/api';

	// Add state for type selection
	let selectedType = $state<ProductType>('hardware');
	let showTypeSelector = $state(false);

	// Update openEditor to use selected type
	async function openEditor() {
		try {
			if (!currentIdentity?.identity || !currentIdentity?.publicKeyBytes) {
				error = 'Please authenticate to edit offerings';
				return;
			}

			if (offerings.length > 0) {
				// Export existing offerings
				const pubkeyHex = hexEncode(currentIdentity.publicKeyBytes);
				const path = `/api/v1/providers/${pubkeyHex}/offerings/export`;
				const signed = await signRequest(currentIdentity.identity, 'GET', path);
				editorCsvContent = await exportProviderOfferingsCSV(
					currentIdentity.publicKeyBytes,
					signed.headers
				);
			} else {
				// Show type selector for new providers
				showTypeSelector = true;
				return;
			}

			showEditorDialog = true;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load CSV';
			console.error('Error loading CSV:', e);
		}
	}

	async function handleTypeSelected(type: ProductType) {
		try {
			editorCsvContent = await fetchCSVTemplateForType(type);
			showTypeSelector = false;
			showEditorDialog = true;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to load template';
		}
	}

	function handleTypeSelectorClose() {
		showTypeSelector = false;
	}
</script>

<!-- Add type selector modal before OfferingsEditor -->
{#if showTypeSelector}
	<div class="fixed inset-0 bg-black/50 backdrop-blur-sm flex items-center justify-center z-50">
		<div class="bg-gray-900 rounded-xl p-8 max-w-2xl w-full mx-4 border border-white/20">
			<h2 class="text-2xl font-bold text-white mb-4">What type of offerings do you provide?</h2>
			<p class="text-white/60 mb-6">Choose the type that best matches your services</p>

			<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
				{#each PRODUCT_TYPES as { value, label, icon }}
					<button
						onclick={() => handleTypeSelected(value)}
						class="p-6 bg-white/10 hover:bg-white/20 rounded-lg border border-white/20 hover:border-blue-400 transition-all text-left group"
					>
						<div class="text-4xl mb-3">{icon}</div>
						<div class="text-white font-semibold mb-1 group-hover:text-blue-400">{label}</div>
						<div class="text-white/60 text-sm">
							{#if value === 'hardware'}
								Virtual Private Servers, Dedicated Servers, Cloud Instances
							{:else if value === 'colocation'}
								Physical rack space, power, and network connectivity
							{:else if value === 'saas'}
								Managed software applications and platforms
							{:else if value === 'gpu'}
								GPU-accelerated compute for AI/ML workloads
							{:else if value === 'validator'}
								Blockchain validation and staking services
							{/if}
						</div>
					</button>
				{/each}
			</div>

			<button
				onclick={handleTypeSelectorClose}
				class="mt-6 px-4 py-2 bg-white/10 rounded-lg hover:bg-white/20 transition-all w-full"
			>
				Cancel
			</button>
		</div>
	</div>
{/if}
```

**Commit Message:**
```
feat(web): add offering type selector to dashboard

- Show type selector for new providers
- Download type-specific CSV template
- Visual type selection with icons
```

**Completion Checklist:**
- [ ] Type selector modal added
- [ ] Template download per type works
- [ ] UI tested in browser
- [ ] Committed

---

### Step 4.3: Update Marketplace Filters

**Status:** ⬜ Not Started

**Objective:** Add filter buttons for new types

**File:** `website/src/routes/dashboard/marketplace/+page.svelte`

**Implementation:**
```svelte
<script lang="ts">
	import { PRODUCT_TYPES } from '$lib/services/api';

	// Update type to use ProductType
	let selectedType = $state<string>("all");
</script>

<!-- Replace hardcoded filter buttons (line 164-201) -->
<div class="flex gap-2 flex-wrap">
	<button
		onclick={() => (selectedType = "all")}
		class="px-4 py-3 rounded-lg font-medium transition-all {selectedType === 'all'
			? 'bg-blue-600 text-white'
			: 'bg-white/10 text-white/70 hover:bg-white/20'}"
	>
		All
	</button>
	{#each PRODUCT_TYPES as { value, label, icon }}
		<button
			onclick={() => (selectedType = value)}
			class="px-4 py-3 rounded-lg font-medium transition-all {selectedType === value
				? 'bg-blue-600 text-white'
				: 'bg-white/10 text-white/70 hover:bg-white/20'}"
		>
			{icon} {label.split(' ')[0]}
		</button>
	{/each}
</div>
```

**Commit Message:**
```
feat(web): add filters for all offering types in marketplace

- Replace hardcoded filters with PRODUCT_TYPES
- Add icons to filter buttons
- DRY: reuse type definitions
```

**Completion Checklist:**
- [ ] Filters updated
- [ ] All types show in marketplace
- [ ] Filtering works
- [ ] Committed

---

### Step 4.4: Add Type-Specific Spec Display

**Status:** ⬜ Not Started

**Objective:** Show relevant specs per offering type in marketplace

**File:** `website/src/routes/dashboard/marketplace/+page.svelte`

**Implementation:**
```svelte
<script lang="ts">
	// Replace formatSpecs function (line 92-113)
	function formatSpecs(offering: Offering): string {
		const specs: string[] = [];

		switch (offering.product_type) {
			case 'hardware':
				if (offering.processor_cores) specs.push(`${offering.processor_cores} vCPU`);
				if (offering.memory_amount) specs.push(`${offering.memory_amount} RAM`);
				if (offering.total_ssd_capacity) {
					specs.push(`${offering.total_ssd_capacity} SSD`);
				} else if (offering.total_hdd_capacity) {
					specs.push(`${offering.total_hdd_capacity} HDD`);
				}
				break;

			case 'colocation':
				if (offering.rack_units) specs.push(`${offering.rack_units}U Rack`);
				if (offering.power_watts) specs.push(`${offering.power_watts}W Power`);
				if (offering.network_ports) specs.push(`${offering.network_ports} Ports`);
				if (offering.bandwidth_gbps) specs.push(`${offering.bandwidth_gbps} Gbps`);
				break;

			case 'saas':
				if (offering.software_name) specs.push(offering.software_name);
				if (offering.software_version) specs.push(`v${offering.software_version}`);
				if (offering.user_quota) specs.push(`${offering.user_quota} users`);
				if (offering.storage_quota_gb) specs.push(`${offering.storage_quota_gb}GB storage`);
				if (offering.managed_support) specs.push('Managed Support');
				break;

			case 'gpu':
				if (offering.gpu_model) specs.push(offering.gpu_model);
				if (offering.gpu_count) specs.push(`${offering.gpu_count}x GPU`);
				if (offering.processor_cores) specs.push(`${offering.processor_cores} vCPU`);
				if (offering.memory_amount) specs.push(`${offering.memory_amount} RAM`);
				break;

			case 'validator':
				if (offering.blockchain) specs.push(offering.blockchain);
				if (offering.commission_percent !== undefined) {
					specs.push(`${offering.commission_percent}% commission`);
				}
				if (offering.uptime_sla_percent !== undefined) {
					specs.push(`${offering.uptime_sla_percent}% uptime`);
				}
				break;
		}

		if (offering.datacenter_country) {
			specs.push(`${offering.datacenter_city}, ${offering.datacenter_country}`);
		}

		return specs.length > 0 ? specs.join(' • ') : offering.description || 'No details available';
	}

	// Update getTypeIcon to handle new types (line 77-83)
	function getTypeIcon(productType: string) {
		const typeConfig = PRODUCT_TYPES.find(t => t.value === productType);
		return typeConfig?.icon || '📦';
	}
</script>
```

**Commit Message:**
```
feat(web): add type-specific spec formatting in marketplace

- Show colocation rack units, power, ports
- Show validator blockchain and commission
- Show GPU model and count
- Show SaaS software name and quotas
- DRY: use PRODUCT_TYPES for icons
```

**Completion Checklist:**
- [ ] Spec display updated for all types
- [ ] Each type shows relevant info
- [ ] Tested with example offerings
- [ ] Committed

---

### Step 4.5: Update TypeScript Generated Types

**Status:** ⬜ Not Started

**Objective:** Regenerate types from Rust, verify frontend builds

**Tasks:**
1. Rebuild API (generates TypeScript types)
2. Verify new fields in `Offering.ts`
3. Fix any TypeScript errors
4. Run frontend build

**Commands:**
```bash
# Rebuild API (regenerates TS types)
cd api
cargo build --release

# Verify types generated
cat ../website/src/lib/types/generated/Offering.ts | grep -E "(rack_units|blockchain|gpu_model)"

# Check for TypeScript errors
cd ../website
npm run check

# Run full build
npm run build
```

**Commit Message:**
```
build: regenerate TypeScript types from Rust

- Update Offering interface with new fields
- Fix TypeScript errors
- Verify frontend builds successfully
```

**Completion Checklist:**
- [ ] TypeScript types regenerated
- [ ] No TypeScript errors
- [ ] Frontend builds
- [ ] Committed

---

## Phase 5: Testing & Documentation (Day 5)

### Step 5.1: Write Integration Tests

**Status:** ⬜ Not Started

**Objective:** Test CSV import/export for each type

**File:** `api/src/database/offerings/tests.rs` (NEW)

**Implementation:**
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    async fn setup_test_db() -> Database {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!("./migrations").run(&db.pool).await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_colocation_csv_roundtrip() {
        let db = setup_test_db().await;
        let pubkey = vec![1u8; 32];

        let csv = "offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,rack_units,power_watts,network_ports\n\
                   colo-001,Test Colo,colocation,299.99,USD,US,NYC,public,monthly,in_stock,10,1000,2";

        let (success, errors) = db.import_offerings_csv(&pubkey, csv, false).await.unwrap();

        assert_eq!(success, 1);
        assert_eq!(errors.len(), 0);

        let offerings = db.get_provider_offerings(&pubkey).await.unwrap();
        assert_eq!(offerings.len(), 1);
        assert_eq!(offerings[0].rack_units, Some(10));
        assert_eq!(offerings[0].power_watts, Some(1000));
    }

    #[tokio::test]
    async fn test_validator_csv_roundtrip() {
        let db = setup_test_db().await;
        let pubkey = vec![2u8; 32];

        let csv = "offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,blockchain,commission_percent\n\
                   val-001,ICP Validator,validator,149.99,USD,DE,Frankfurt,public,monthly,in_stock,Internet Computer,5.0";

        let (success, errors) = db.import_offerings_csv(&pubkey, csv, false).await.unwrap();

        assert_eq!(success, 1);
        assert_eq!(errors.len(), 0);

        let offerings = db.get_provider_offerings(&pubkey).await.unwrap();
        assert_eq!(offerings[0].blockchain, Some("Internet Computer".to_string()));
        assert_eq!(offerings[0].commission_percent, Some(5.0));
    }

    #[tokio::test]
    async fn test_validation_colocation_missing_rack_units() {
        let db = setup_test_db().await;
        let pubkey = vec![3u8; 32];

        let csv = "offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,power_watts\n\
                   colo-bad,Bad Colo,colocation,299.99,USD,US,NYC,public,monthly,in_stock,1000";

        let (success, errors) = db.import_offerings_csv(&pubkey, csv, false).await.unwrap();

        assert_eq!(success, 0);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].1.contains("rack_units"));
    }
}
```

**Testing:**
```bash
cargo test integration_tests
```

**Commit Message:**
```
test(api): add CSV roundtrip tests for all types

- Test colocation import/export
- Test validator import/export
- Test validation errors
- Cover all 5 product types
```

**Completion Checklist:**
- [ ] Tests for all types
- [ ] Tests pass
- [ ] Committed

---

### Step 5.2: Write E2E Frontend Tests

**Status:** ⬜ Not Started

**Objective:** Test type selector and CSV workflow in browser

**File:** `website/tests/e2e/offerings-types.spec.ts` (NEW)

**Implementation:**
```typescript
import { test, expect } from '@playwright/test';

test.describe('Offering Type Selection', () => {
	test('should show type selector for new providers', async ({ page }) => {
		// Navigate to offerings dashboard
		await page.goto('/dashboard/offerings');

		// Click "Edit Offerings" button
		await page.click('text=Edit Offerings');

		// Should see type selector modal
		await expect(page.locator('text=What type of offerings do you provide?')).toBeVisible();

		// Should see all 5 types
		await expect(page.locator('text=Hardware')).toBeVisible();
		await expect(page.locator('text=Colocation')).toBeVisible();
		await expect(page.locator('text=SaaS')).toBeVisible();
		await expect(page.locator('text=GPU')).toBeVisible();
		await expect(page.locator('text=Validator')).toBeVisible();
	});

	test('should download colocation template', async ({ page }) => {
		await page.goto('/dashboard/offerings');
		await page.click('text=Edit Offerings');

		// Click colocation type
		await page.click('text=Colocation');

		// Should open CSV editor
		await expect(page.locator('text=Edit Offerings CSV')).toBeVisible();

		// CSV should have colocation columns
		const csvContent = await page.locator('textarea').inputValue();
		expect(csvContent).toContain('rack_units');
		expect(csvContent).toContain('power_watts');
		expect(csvContent).not.toContain('processor_cores'); // Not hardware
	});
});

test.describe('Marketplace Type Filters', () => {
	test('should filter by colocation', async ({ page }) => {
		await page.goto('/dashboard/marketplace');

		// Click colocation filter
		await page.click('text=Colocation');

		// Should only show colocation offerings
		// (assumes test data exists)
		const cards = page.locator('[data-product-type]');
		await expect(cards).toHaveCount(1); // Adjust based on test data
	});
});
```

**Testing:**
```bash
cd website
npm run test:e2e
```

**Commit Message:**
```
test(web): add e2e tests for offering types

- Test type selector workflow
- Test CSV template download
- Test marketplace filtering
```

**Completion Checklist:**
- [ ] E2E tests written
- [ ] Tests pass
- [ ] Committed

---

### Step 5.3: Update Documentation

**Status:** ⬜ Not Started

**Objective:** Document CSV format for each type

**File:** `docs/CSV_FORMATS.md` (NEW)

**Implementation:**
```markdown
# CSV Formats for Offering Types

This document describes the CSV format for each resource type in the marketplace.

## Common Columns (All Types)

These columns are required for all offering types:

- `offering_id` - Unique identifier for this offering (alphanumeric, no spaces)
- `offer_name` - Display name for the offering
- `product_type` - Type of offering: `hardware`, `colocation`, `saas`, `gpu`, or `validator`
- `monthly_price` - Monthly price (numeric)
- `currency` - Currency code (e.g., USD, EUR)
- `datacenter_country` - Country code (e.g., US, DE)
- `datacenter_city` - City name
- `visibility` - `public` or `private`
- `billing_interval` - `monthly` or `yearly`
- `stock_status` - `in_stock`, `out_of_stock`, or `discontinued`

Optional common columns:
- `description` - Detailed description
- `product_page_url` - Link to offering details
- `setup_fee` - One-time setup cost (default: 0)
- `datacenter_latitude` - Latitude coordinate
- `datacenter_longitude` - Longitude coordinate

## Hardware Offerings

**Template:** Download from `/api/v1/offerings/template/hardware`

**Type-specific columns:**
- `processor_cores` (required) - Number of CPU cores
- `memory_amount` (required) - RAM amount (e.g., "8GB", "16GB")
- `storage_type` - `ssd`, `hdd`, or `nvme`
- `storage_amount_gb` - Storage capacity in GB
- `processor_brand` - CPU manufacturer (Intel, AMD)
- `virtualization_type` - `kvm`, `vmware`, `xen`, etc.
- `uplink_speed` - Network speed (e.g., "1Gbps")
- `features` - Comma-separated list
- `operating_systems` - Comma-separated OS list

**Example:**
```csv
offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,processor_cores,memory_amount
hw-001,Basic VPS,hardware,29.99,USD,US,New York,public,monthly,in_stock,2,4GB
```

## Colocation Offerings

**Template:** Download from `/api/v1/offerings/template/colocation`

**Type-specific columns:**
- `rack_units` (required) - Rack space in U (e.g., 10, 21, 42)
- `power_watts` (required) - Power allocation in watts
- `network_ports` (required) - Number of network ports
- `bandwidth_gbps` - Bandwidth commitment in Gbps
- `features` - Comma-separated list (e.g., "24/7 Access,Security")

**Example:**
```csv
offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,rack_units,power_watts,network_ports
colo-001,Quarter Rack,colocation,299.99,USD,DE,Frankfurt,public,monthly,in_stock,10,1000,2
```

## SaaS Offerings

**Template:** Download from `/api/v1/offerings/template/saas`

**Type-specific columns:**
- `software_name` (required) - Name of the software
- `software_version` - Version number
- `user_quota` - Maximum number of users
- `storage_quota_gb` - Storage limit in GB
- `managed_support` - `true` or `false`
- `features` - Comma-separated features

**Example:**
```csv
offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,software_name,user_quota,managed_support
saas-001,WordPress Hosting,saas,19.99,USD,US,Chicago,public,monthly,in_stock,WordPress,5,true
```

## GPU Offerings

**Template:** Download from `/api/v1/offerings/template/gpu`

**Type-specific columns:**
- `gpu_model` (required) - GPU model (e.g., "NVIDIA A100")
- `gpu_count` (required) - Number of GPUs
- `processor_cores` - CPU cores
- `memory_amount` - System RAM
- `uplink_speed` - Network speed
- `features` - Comma-separated list

**Example:**
```csv
offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,gpu_model,gpu_count
gpu-001,A100 Instance,gpu,499.99,USD,US,Austin,public,monthly,in_stock,NVIDIA A100,1
```

## Validator Offerings

**Template:** Download from `/api/v1/offerings/template/validator`

**Type-specific columns:**
- `blockchain` (required) - Blockchain network (e.g., "Internet Computer")
- `commission_percent` (required) - Commission rate (0-100)
- `uptime_sla_percent` - Uptime guarantee (0-100)
- `minimum_stake` - Minimum staking amount
- `features` - Comma-separated features

**Example:**
```csv
offering_id,offer_name,product_type,monthly_price,currency,datacenter_country,datacenter_city,visibility,billing_interval,stock_status,blockchain,commission_percent,uptime_sla_percent
val-001,ICP Validator,validator,149.99,USD,CH,Zurich,public,monthly,in_stock,Internet Computer,5.0,99.9
```

## Importing CSVs

1. **Download template** for your offering type
2. **Fill in your data** (keep column headers)
3. **Upload via dashboard** or API
4. **Validation** will check required fields per type
5. **Upsert mode** updates existing offerings by `offering_id`

## API Endpoints

- `GET /api/v1/offerings/template/:type` - Download type-specific template
- `POST /api/v1/providers/:pubkey/offerings/import?upsert=true` - Import CSV
- `GET /api/v1/providers/:pubkey/offerings/export?product_type=:type` - Export CSV
```

**Commit Message:**
```
docs: add CSV format documentation for all types

- Document required fields per type
- Add examples for each type
- Explain import workflow
```

**Completion Checklist:**
- [ ] CSV_FORMATS.md created
- [ ] All 5 types documented
- [ ] Examples provided
- [ ] Committed

---

### Step 5.4: Update Implementation Spec Status

**Status:** ⬜ Not Started

**Objective:** Mark all steps complete, add final notes

**Tasks:**
1. Update status of all completed steps to ✅
2. Add "Lessons Learned" section below
3. Update top-level status to 🟢 Complete

**Commit Message:**
```
docs: mark marketplace extension implementation complete

- All phases completed
- Tests passing
- Documentation updated
```

**Completion Checklist:**
- [ ] All steps marked complete
- [ ] Lessons learned documented
- [ ] Committed

---

## Testing Checklist

### Backend Tests
- [ ] Unit tests for CSV parsing (all types)
- [ ] Unit tests for validation (all types)
- [ ] Integration tests for CSV roundtrip
- [ ] API endpoint tests

### Frontend Tests
- [ ] E2E test: Type selector workflow
- [ ] E2E test: CSV template download
- [ ] E2E test: Marketplace filtering
- [ ] Visual regression tests (optional)

### Manual Testing
- [ ] Download hardware template
- [ ] Download colocation template
- [ ] Download validator template
- [ ] Import valid CSV (each type)
- [ ] Import invalid CSV (missing required fields)
- [ ] Export existing offerings
- [ ] Filter marketplace by each type
- [ ] Create contract for each type

---

## Deployment Checklist

### Pre-Deployment
- [ ] All tests passing (`cargo make` clean)
- [ ] No TypeScript errors
- [ ] Frontend builds successfully
- [ ] Database migration tested on staging

### Deployment Steps
1. [ ] Backup production database
2. [ ] Run migration `006_marketplace_extension.sql`
3. [ ] Verify example offerings inserted
4. [ ] Deploy API
5. [ ] Deploy frontend
6. [ ] Smoke test each offering type

### Post-Deployment
- [ ] Monitor error logs
- [ ] Check CSV import success rate
- [ ] Verify marketplace loads
- [ ] Test one CSV import live

---

## Rollback Plan

If critical issues found:

1. **Frontend rollback:**
   ```bash
   git revert <frontend-commits>
   npm run build && npm run deploy
   ```

2. **Backend rollback:**
   ```bash
   git revert <backend-commits>
   cargo build --release && restart-api-service
   ```

3. **Database rollback:**
   - New columns can stay (NULL values harmless)
   - If needed: `ALTER TABLE provider_offerings DROP COLUMN <col>`

---

## Lessons Learned

*To be filled after implementation*

### What Went Well
-

### What Could Be Improved
-

### Unexpected Challenges
-

### Recommendations for Future
-

---

## Final Metrics

- **Total Commits:** TBD
- **Lines of Code Added:** TBD
- **Lines of Code Modified:** TBD
- **Tests Added:** TBD
- **Implementation Time:** TBD days
- **Bugs Found Post-Deploy:** TBD

---

**Last Updated:** TBD
**Implemented By:** TBD
**Reviewed By:** TBD
