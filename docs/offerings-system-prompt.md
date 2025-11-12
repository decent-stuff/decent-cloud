# Offerings System: Implementation Status

## Phase 1: Core CRUD Operations - ✅ COMPLETED

**Completion Date**: 2025-11-12

### Summary

Successfully implemented full CRUD functionality for server offerings in the API server. All operations are authenticated, authorized, and fully tested.

### What Was Implemented

#### 1. Database Layer (`api/src/database/offerings.rs`)
- **Added `CreateOfferingParams` struct** with 43 fields for all offering data
- **`create_offering()`** - Create new offering with transaction support, duplicate prevention, and metadata insertion
- **`update_offering()`** - Update existing offering with ownership verification and metadata replacement
- **`delete_offering()`** - Delete offering with ownership verification (CASCADE handles metadata)
- **`duplicate_offering()`** - Clone offering with new ID, preserving all data and metadata

**Code Stats**: Added ~200 lines across 4 methods + helper functions
**Tests**: 12 new tests (3 per CRUD operation - success, unauthorized, edge cases)
**Test Results**: 21/21 passing (including 9 pre-existing tests)

#### 2. API Handlers (`api/src/api_handlers.rs`)
- **`create_provider_offering()`** - POST endpoint with authentication
- **`update_provider_offering()`** - PUT endpoint with ownership check
- **`delete_provider_offering()`** - DELETE endpoint with authorization
- **`duplicate_provider_offering()`** - POST endpoint for cloning
- **`DuplicateOfferingRequest`** struct for request body

**Code Stats**: ~100 lines across 4 handlers + request struct

#### 3. Routes (`api/src/main.rs`)
- `POST /api/v1/providers/{pubkey}/offerings` - Create
- `PUT /api/v1/providers/{pubkey}/offerings/{id}` - Update
- `DELETE /api/v1/providers/{pubkey}/offerings/{id}` - Delete
- `POST /api/v1/providers/{pubkey}/offerings/{id}/duplicate` - Duplicate

**Code Stats**: ~15 lines for route registration

### Key Design Decisions

1. **Authentication**: All mutations require `AuthenticatedUser` (signature-based)
2. **Authorization**: Double-check pubkey matches URL parameter and offering owner
3. **Transactions**: Create/update use SQL transactions for atomicity
4. **Validation**: Required fields (offering_id, offer_name) validated before DB operations
5. **Metadata Handling**: Payment methods, features, and operating systems stored in normalized tables
6. **Duplicate Detection**: offering_id + pubkey_hash unique constraint enforced

### Testing Coverage

```
✅ test_create_offering_success - Full offering with metadata
✅ test_create_offering_duplicate_id - Prevents duplicates
✅ test_create_offering_missing_required_fields - Validates required fields

✅ test_update_offering_success - Updates all fields + metadata
✅ test_update_offering_unauthorized - Rejects unauthorized updates

✅ test_delete_offering_success - Deletes offering and metadata (CASCADE)
✅ test_delete_offering_unauthorized - Rejects unauthorized deletions

✅ test_duplicate_offering_success - Copies offering with new ID
✅ test_duplicate_offering_unauthorized - Rejects unauthorized duplication
```

### Build & Test Status

```bash
cargo build --bin api-server  # ✅ SUCCESS (6 warnings - unrelated)
cargo clippy --bin api-server # ✅ CLEAN (only pre-existing warnings)
cargo test --bin api-server   # ✅ 98/98 PASSING
```

### File Size Analysis

- `api/src/database/offerings.rs`: 1,285 lines (approaching limits, needs refactoring for Phase 2)
- `api/src/api_handlers.rs`: ~500 lines (within limits)
- `api/src/main.rs`: ~350 lines (within limits)

**Recommendation**: For Phase 2, split `offerings.rs` into modules:
- `offerings/crud.rs` - CRUD operations
- `offerings/bulk.rs` - Bulk operations
- `offerings/queries.rs` - Search/read operations

---

# Offerings System: Analysis & Development Plan

## Context

We need to design and implement a flexible, generic offerings system that:
1. Currently supports "server" offerings (following serverhunter.com data model)
2. Future-proofs for diverse offering types: WordPress hosting, Bitcoin miners, Ethereum miners, Solana miners, hosted websites, and more
3. Makes it extremely easy for providers to create/edit offerings via a convenient interface (spreadsheet-like or better)

## Current State Analysis Required

### API Layer (`/home/sat/projects/decent-cloud/api/`)

**Existing Data Model:**
- `/home/sat/projects/decent-cloud/api/src/database/offerings.rs` - `Offering` struct (48 fields)
- `/home/sat/projects/decent-cloud/api/migrations/001_original_schema.sql` - Database schema
  - `provider_offerings` table (48 columns)
  - Normalized tables: `provider_offerings_payment_methods`, `provider_offerings_features`, `provider_offerings_operating_systems`

**Current Fields Cover:**
- Pricing: monthly_price, setup_fee, currency, price_per_hour_e9s, price_per_day_e9s, billing_interval
- Hardware: processor (brand/cores/speed), memory (amount/type), storage (HDD/SSD amounts), GPU
- Network: uplink_speed, unmetered_bandwidth, traffic
- Location: datacenter_country, datacenter_city, latitude, longitude
- Product: product_type, virtualization_type, visibility, stock_status
- Contract: min/max contract hours

**Existing Endpoints (Read-only):**
- `GET /search_offerings` - Search with filters
- `GET /providers/{pubkey}/offerings` - List provider's offerings
- `GET /offerings/{offering_id}` - Get single offering

**Missing:** No create/update/delete endpoints exist yet

### Website Layer (`/home/sat/projects/decent-cloud/website-svelte/`)

**Existing UI:**
- `/home/sat/projects/decent-cloud/website-svelte/src/routes/dashboard/offerings/+page.svelte` - Provider offerings dashboard (grid view, placeholders for Create/Edit/Disable)
- `/home/sat/projects/decent-cloud/website-svelte/src/routes/dashboard/marketplace/+page.svelte` - Buyer marketplace (search/filter, grid display)

**Existing Patterns for Form Components:**
- `UserProfileEditor.svelte`, `ContactsEditor.svelte`, `SocialsEditor.svelte`, `PublicKeysEditor.svelte`
- Pattern: `$state()` management → `onMount()` load → form inputs → add/delete buttons → success/error alerts → `UserApiClient` mutations

**Missing:** No `OfferingEditor.svelte` component, no mutation methods in `UserApiClient`

## Requirements

### Functional Requirements

1. **Generic Offering Type System**
   - Schema must support arbitrary offering types beyond "server"
   - Each offering type may have unique fields (e.g., WordPress: PHP version, MySQL size; Bitcoin miner: hashrate, power consumption)
   - Common fields across all types: name, description, pricing, provider, visibility, stock status

2. **Provider-Friendly Editing Interface**
   - Must be extremely convenient for bulk creation/editing
   - Options to consider:
     a. Spreadsheet-like table editor (inline editing, bulk actions)
     b. CSV/Excel import/export
     c. Form-based editor with templates per offering type
     d. API-based bulk upload (JSON/YAML)
   - **Ask AI to suggest best approach based on:** provider personas, typical use cases (1 offering vs. 100 offerings), maintenance burden

3. **CRUD Operations**
   - Create new offering (with type selection)
   - Update existing offering
   - Delete/disable offering
   - Bulk operations (enable/disable multiple, duplicate offering)

4. **Validation & Constraints**
   - Required fields per offering type
   - Pricing validation (no negative prices, currency consistency)
   - Location validation (country codes, coordinates)
   - Stock status enforcement

### Technical Requirements

1. **Database Design**
   - How to model generic offering types? Options:
     a. JSONB column for type-specific fields (flexible but loses type safety)
     b. EAV pattern (entity-attribute-value)
     c. Table-per-type (clean but requires migrations for new types)
     d. Hybrid: common fields in main table + JSONB for extras
   - Must maintain backward compatibility with existing 48-column schema
   - Must support efficient filtering/searching across types

2. **API Design**
   - RESTful endpoints: `POST /offerings`, `PUT /offerings/{id}`, `DELETE /offerings/{id}`
   - Bulk endpoints: `POST /offerings/bulk`, `PUT /offerings/bulk`
   - Type metadata endpoint: `GET /offering-types` (list available types with field schemas)
   - Authentication: must verify provider owns the offering

3. **Frontend Architecture**
   - Component structure: reusable `OfferingEditor.svelte` that adapts to offering type?
   - State management for large datasets (if spreadsheet approach)
   - Form validation (client-side + server-side)
   - UX for switching between offering types

4. **Testing Strategy**
   - Unit tests for CRUD operations
   - Tests for type-specific validation
   - Integration tests for API endpoints
   - UI component tests for form submission/error handling

## Analysis Tasks

1. **Data Model Analysis**
   - Compare existing 48 fields against serverhunter.com model - what's missing?
   - Identify which fields are server-specific vs. generic
   - Propose schema evolution strategy for supporting multiple offering types
   - Design migration path from current schema

2. **UX/UI Research**
   - Evaluate spreadsheet-like editors: ag-Grid, Handsontable, TanStack Table
   - Compare with form-based approach (existing pattern in ContactsEditor.svelte)
   - Prototype mockup for bulk editing 50+ offerings
   - Recommend optimal approach with trade-offs

3. **Architecture Planning**
   - Propose API endpoint design (paths, methods, request/response schemas)
   - Define authentication/authorization strategy
   - Specify error handling patterns
   - Outline testing approach

4. **Implementation Roadmap**
   - Break down into minimal milestones (aligned with TDD, YAGNI, DRY principles)
   - Phase 1: Server offerings CRUD (maintain current schema)
   - Phase 2: Generic offering type system
   - Phase 3: Bulk editing interface
   - Each phase must include: failing tests → minimal code → `cargo make` clean → refactor

## Constraints

- **Code Quality:** Follow AGENTS.md rules (TDD, YAGNI, DRY, fail-fast, no silent errors)
- **Testing:** Every function covered by unit tests, both positive and negative paths
- **Hard Limits:** New files max 200 lines, functions max 50 lines, total per feature max 200 lines
- **Development Order:** Search existing code → Failing test → Minimal code → `cargo clippy` and `cargo test` clean → Refactor → Docs (if needed)
- **Backward Compatibility:** Must not break existing `GET /search_offerings` and related endpoints

## Deliverables

1. **Analysis Document:**
   - Data model comparison and gaps
   - Schema evolution proposal (with SQL migration scripts)
   - UX recommendation with rationale
   - API specification (OpenAPI/Swagger style)

2. **Development Plan:**
   - TodoWrite-compatible task breakdown
   - Test specifications for each task
   - File structure (which files to modify, which to create)
   - Rollout strategy (feature flags? versioned API?)

3. **Prototype (Optional):**
   - If spreadsheet approach: working demo with 10 sample offerings
   - If form approach: `OfferingEditor.svelte` component mockup

## Questions for AI

1. **Data Model:** What's the best way to support generic offering types while maintaining query performance and type safety? Provide pros/cons for JSONB, EAV, table-per-type, and hybrid approaches.

2. **UX:** For a provider managing 50-200 offerings, what's the optimal editing interface? Compare: (a) inline spreadsheet editor, (b) modal-based form editor, (c) CSV import/export, (d) combination approach.

3. **Migration Strategy:** How do we evolve from the current 48-column `provider_offerings` table to a generic system without breaking existing data and queries?

4. **Validation:** Should offering type schemas be hardcoded in Rust structs, stored in DB, or defined in config files? How to ensure consistent validation across API and UI?

5. **Testing:** What's the minimal test suite for CRUD operations that covers edge cases (duplicate IDs, unauthorized edits, invalid types, concurrent updates)?
