# Search DSL Implementation
**Status:** In Progress

## Requirements

### Must-have
- [ ] DSL parser supporting: `field:value`, `field:>=num`, `field:[min TO max]`
- [ ] Boolean operators: `AND`, `OR` (implicit AND between terms)
- [ ] Grouping with parentheses: `type:(gpu OR compute)`
- [ ] Negation: `!field:value` or `-field:value`
- [ ] SQL injection prevention via allowlisted field names
- [ ] Backend API endpoint accepting `q` query parameter
- [ ] Frontend search bar that sends DSL queries
- [ ] Unit tests for parser (positive + negative cases)
- [ ] Integration tests for API

### Nice-to-have
- [ ] Autocomplete suggestions for field names
- [ ] Query syntax help tooltip

## Searchable Fields (Allowlist)

| DSL Field | DB Column | Type | Operators |
|-----------|-----------|------|-----------|
| `name` | `offer_name` | text | exact, contains |
| `type` | `product_type` | text | exact, OR |
| `stock` | `stock_status` | text | exact |
| `price` | `monthly_price` | float | `>=`, `<=`, `[TO]` |
| `cores` | `processor_cores` | int | `>=`, `<=`, `[TO]` |
| `memory` | `memory_amount` | text | contains |
| `country` | `datacenter_country` | text | exact, OR |
| `city` | `datacenter_city` | text | exact |
| `gpu` | `gpu_name` | text | exact, contains |
| `gpu_count` | `gpu_count` | int | `>=`, `<=` |
| `gpu_memory` | `gpu_memory_gb` | int | `>=`, `<=` |
| `features` | `features` | csv | contains |
| `unmetered` | `unmetered_bandwidth` | bool | exact |
| `traffic` | `traffic` | int | `>=`, `<=` |
| `trust` | `trust_score` | int | `>=`, `<=` |

## DSL Syntax Examples

```
# Basic exact match
type:gpu

# Price range
price:[50 TO 200]
price:>=100
price:<=500

# Multiple values (OR)
type:(gpu OR compute)
country:(US OR DE OR NL)

# Boolean AND (implicit)
type:gpu price:<=100 country:US

# Negation
!stock:out_of_stock
-country:CN

# Combined complex query
type:(gpu OR compute) price:[50 TO 500] cores:>=8 !stock:out_of_stock country:(US OR DE)
```

## Steps

### Step 1: Create DSL Parser Module
**Success:** Parser correctly tokenizes and builds AST from DSL strings. All test cases pass.
**Status:** ✅ Completed

Files:
- `api/src/search/mod.rs` - module definition
- `api/src/search/parser.rs` - tokenizer + parser
- `api/src/search/types.rs` - AST types (Filter, Operator, etc.)
- `api/src/search/tests.rs` - unit tests

### Step 2: Create SQL Query Builder
**Success:** Builds safe parameterized SQL from parsed AST. No SQL injection possible.
**Status:** Pending

Files:
- `api/src/search/builder.rs` - SQL generation with bind values
- Extend tests in `api/src/search/tests.rs`

### Step 3: Integrate with Database Layer
**Success:** `search_offerings_dsl(query: &str)` works and returns correct results.
**Status:** Pending

Files:
- `api/src/database/offerings.rs` - add `search_offerings_dsl` method
- `api/src/database/offerings/tests.rs` - integration tests

### Step 4: Add API Endpoint
**Success:** `GET /offerings?q=type:gpu` returns filtered results.
**Status:** Pending

Files:
- `api/src/openapi/offerings.rs` - add `q` parameter to `search_offerings`

### Step 5: Frontend Search Bar
**Success:** Marketplace uses DSL query bar, queries work end-to-end.
**Status:** Pending

Files:
- `website/src/routes/dashboard/marketplace/+page.svelte` - replace search input
- `website/src/lib/services/api.ts` - add `q` parameter support

### Step 6: Final Review & Documentation
**Success:** All tests pass, `cargo make` clean, code reviewed.
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:** Created search module with 4 files:
  - `api/src/search/mod.rs` - Module exports (parse_dsl, Filter, Operator, Value)
  - `api/src/search/types.rs` - AST types (Filter, Operator, Value enums)
  - `api/src/search/parser.rs` - Tokenizer and recursive descent parser (328 lines)
  - `api/src/search/tests.rs` - 30 comprehensive unit tests

- **Review:** Code follows KISS, MINIMAL, DRY principles:
  - Tokenizer: Simple char-by-char scanner with peek/advance pattern
  - Parser: Recursive descent, returns Vec<Filter> directly
  - Types: Clean AST with Filter containing field, operator, values[], negated
  - No external dependencies beyond stdlib
  - Zero duplication, clear error messages

- **Verification:**
  - ✅ All 30 tests pass (tested in isolated cargo project)
  - ✅ No compiler warnings
  - ✅ Supports all required DSL syntax:
    - Basic: `field:value`
    - Operators: `:>=`, `:<=`, `:>`, `:<`
    - Range: `field:[min TO max]`
    - OR groups: `field:(value1 OR value2 OR value3)`
    - Negation: `!field:value` and `-field:value`
    - Implicit AND: `term1 term2 term3`
    - Explicit AND: `term1 AND term2`
    - Complex: `type:(gpu OR compute) price:[50 TO 500] cores:>=8 !stock:out_of_stock`
  - ✅ Error handling for invalid syntax
  - ✅ Case-insensitive keywords (AND, OR, TO, TRUE, FALSE)
  - ✅ Value type detection (Integer, Number, Boolean, String)

- **Outcome:** ✅ SUCCESS - Parser module complete and fully tested. Module is self-contained with #[cfg(test)] and NOT added to main.rs yet (as per Step 3).

### Step 2
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

### Step 3
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

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
(To be filled in Phase 4)
