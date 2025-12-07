# Billing & Invoicing Implementation

**Status:** In Progress
**Priority:** HIGH - Required for payment system completion

## Requirements

### Must-have
- [x] Receipt emails sent after payment confirmation
- [x] Sequential receipt numbers (tax compliance)
- [x] PDF invoice generation on demand
- [x] Invoice metadata storage with sequential numbering
- [ ] Download invoice button in UI
- [ ] Stripe Tax integration for VAT
- [ ] Tax details on invoices

### Nice-to-have
- [ ] User billing settings (saved address, VAT ID)
- [ ] VAT ID validation via VIES API
- [ ] Invoice list in user dashboard

## PDF Generation Decision

**Chosen: Typst CLI** (invoked from API server)

### Alternatives Analyzed

| Tool | Type | Verdict | Reason |
|------|------|---------|--------|
| klirr | CLI | ❌ | Too opinionated, RON config, full invoicing system |
| tradedoc | Library | ❌ | 0% documentation, unclear API |
| xrechnung | Library | ❌ | XML only (German e-invoicing), no PDF |
| invogen | CLI | ❌ | Binary only, not embeddable as library |
| clinvoice | CLI | ❌ | Requires headless Chrome, unmaintained |
| genpdf | Library | ⚠️ | No tables, dormant 3+ years |
| **Typst CLI** | CLI | ✅ | Best output, JSON input, active, ~10ms |

### Implementation Approach

1. Create `api/templates/invoice.typ` with invoice layout
2. Pass invoice data as JSON via `--input` flag
3. Generate PDF on-demand: `typst compile --input data='{...}' invoice.typ output.pdf`
4. Cache PDF in database `invoices.pdf_blob`

## Steps

### Step 1: Receipt Email Infrastructure
**Success:** Receipt email sent after Stripe/ICPay payment success, sequential numbering works
**Status:** Pending

- Add migration for `receipt_sequence` table and contract columns
- Create `send_payment_receipt()` function
- Hook into Stripe webhook after payment success
- Hook into ICPay webhook after payment success
- Unit tests for receipt number generation

### Step 2: PDF Invoice Generation with Typst
**Success:** Invoice PDF generated on-demand via API endpoint, stored in database
**Status:** Pending

- Install Typst CLI in deployment environment
- Create `api/templates/invoice.typ` template
- Add migration for `invoices` table and `invoice_sequence`
- Implement `generate_invoice_pdf()` using Typst CLI
- Add `GET /api/v1/contracts/{id}/invoice` endpoint
- Add `GET /api/v1/contracts/{id}/invoice/metadata` endpoint
- Unit tests for invoice generation

### Step 3: Frontend Invoice Download
**Success:** User can download invoice PDF from contract detail page
**Status:** Pending

- Add "Download Invoice" button to contract detail page
- Handle PDF download response
- Show invoice generation status

### Step 4: Stripe Tax Integration
**Success:** Tax calculated at checkout, stored on contract, shown on invoice
**Status:** Pending

- Add tax columns to contract_sign_requests migration
- Update Stripe checkout session creation (enable automatic_tax)
- Extract tax info from Stripe webhook
- Display tax breakdown on invoice PDF
- ICPay: Show "Tax not included" disclaimer

### Step 5: User Billing Settings (Nice-to-have)
**Success:** User can save billing info, auto-populated on invoices
**Status:** Pending

- Add billing columns to user_profiles
- API endpoints for billing settings CRUD
- Frontend billing settings page
- VAT ID validation (Stripe or VIES API)

## Execution Log

### Step 1
- **Implementation:**
  - Created migration `038_receipt_tracking.sql`:
    - `receipt_sequence` table with single-row constraint for atomic numbering
    - Added `receipt_number` and `receipt_sent_at_ns` columns to `contract_sign_requests`
    - Created index on `receipt_number` for lookups
  - Created `api/src/receipts.rs` module:
    - `send_payment_receipt()` function - queues receipt email after payment
    - `get_next_receipt_number()` - atomically increments and returns receipt number using `UPDATE...RETURNING`
    - `update_contract_receipt_info()` - stores receipt number and timestamp on contract
    - Receipt email template with payment and contract details
  - Hooked into webhooks (`api/src/openapi/webhooks.rs`):
    - Stripe `payment_intent.succeeded` - sends receipt after auto-accepting contract
    - ICPay `payment.completed` - sends receipt after auto-accepting contract
  - Added unit tests in `api/src/receipts.rs`:
    - `test_get_next_receipt_number_sequential` - verifies sequential numbering (1, 2, 3...)
    - `test_get_next_receipt_number_atomic` - verifies atomicity with 10 concurrent requests (no duplicates, no gaps)
    - `test_update_contract_receipt_info` - verifies receipt metadata storage
  - Updated `api/src/database/test_helpers.rs` to include migration 038
  - Used `Arc<Database>` for concurrent test access (Database is not Clone)
  - Used `sqlx::query_as` instead of `sqlx::query!` macro in tests to avoid offline mode errors
- **Review:**
  - Code follows existing email queue patterns (EmailType::General)
  - Receipt numbering is atomic and sequential (SQLite UPDATE...RETURNING ensures no race conditions)
  - Non-blocking: webhook doesn't fail if receipt sending fails (logged as warning)
  - Minimal code: Extended existing webhook handlers, reused email queue system
  - DRY: No code duplication
- **Verification:**
  - Unit tests pass (verified via `cargo test --lib receipts`)
  - **BLOCKED:** Full `cargo make` blocked by pre-existing SQLX type inference errors in `api/src/database/reseller.rs` and `api/src/database/providers.rs` (from migration 037, unrelated to this change)
  - Documented blocking issue in TODO.md
- **Outcome:** Implementation complete, tests passing. Receipt email infrastructure ready for use. Blocked on pre-existing compilation errors in reseller/provider code

### Step 2
- **Implementation:**
  - Created Typst template `api/templates/invoice.typ`:
    - Uses `@preview/invoice-maker:1.1.0` package
    - Parses JSON from `sys.inputs.data`
    - Maps invoice data to invoice-maker format (biller, recipient, items)
  - Created migration `039_invoices.sql`:
    - `invoices` table with invoice_number, contract_id, seller/buyer details, amounts, pdf_blob
    - `invoice_sequence` table for atomic sequential numbering per year (INV-YYYY-NNNNNN)
    - Indexes on contract_id, invoice_number, created_at_ns
  - Created `api/src/invoices.rs` module (~480 lines):
    - `get_next_invoice_number()` - atomic sequential numbering with year rollover
    - `create_invoice()` - creates invoice record (idempotent per contract)
    - `generate_invoice_pdf()` - calls Typst CLI via `tokio::process::Command`
    - `get_invoice_pdf()` - returns cached PDF or generates on-demand
    - `get_invoice_metadata()` - returns invoice JSON
  - Created `api/src/openapi/invoices.rs` API endpoints:
    - `GET /api/v1/contracts/{id}/invoice` - returns PDF binary
    - `GET /api/v1/contracts/{id}/invoice/metadata` - returns invoice JSON
    - Auth: requester or provider only
  - Added 4 unit tests: sequential numbering, year rollover, invoice creation, metadata retrieval
- **Review:**
  - Uses existing patterns (sqlx::query for offline mode compatibility)
  - Typst CLI invocation is clean with proper error handling
  - Invoice numbers are atomic and sequential per year
  - PDF caching in database avoids regeneration
  - DRY: Reuses contract data, minimal code
- **Verification:**
  - All 4 invoice tests pass
  - `cargo make` passes cleanly
- **Outcome:** SUCCESS - PDF invoice generation with Typst working

### Step 3
- **Implementation:**
  - Added `downloadContractInvoice()` function to `website/src/lib/services/api.ts`:
    - Calls `GET /api/v1/contracts/{contractId}/invoice` with auth headers
    - Handles PDF binary response using `response.blob()`
    - Creates download link and triggers browser download
    - Follows existing pattern from `downloadOfferingsCSV()` (lines 490-503)
  - Updated `website/src/routes/dashboard/rentals/+page.svelte`:
    - Added "Download Invoice" button for contracts with `payment_status === 'succeeded'`
    - Button appears next to status badge and cancel button
    - Shows loading state while downloading (spinning indicator + "Downloading...")
    - Uses existing auth pattern (`signRequest()` with Ed25519 signature)
    - Error handling with error state display
  - Files changed:
    - `/code/website/src/lib/services/api.ts` (+30 lines)
    - `/code/website/src/routes/dashboard/rentals/+page.svelte` (+42 lines)
- **Review:**
  - Used existing download pattern (blob + URL.createObjectURL)
  - Followed existing auth pattern (signRequest + headers)
  - Minimal changes: extended existing components
  - No new files created
  - Button only shows for paid contracts (payment_status === 'succeeded')
  - Graceful error handling with user feedback
- **Verification:**
  - `npm run check` - 0 errors, 0 warnings
  - `npm run build` - SUCCESS (built in 15.90s)
- **Outcome:** SUCCESS - Invoice download button added to rentals page, follows all existing patterns, builds cleanly

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

## Completion Summary
(To be filled after implementation)

---

## Technical Details

### Receipt Email Template

```
Subject: Receipt for your Decent Cloud rental - #{receipt_number}

Receipt #{receipt_number}
Date: {date}

Thank you for your payment!

PAYMENT DETAILS
───────────────────────────────────
Amount Paid:     {amount} {currency}
Payment Method:  {payment_method}
Transaction ID:  {transaction_id}

CONTRACT DETAILS
───────────────────────────────────
Offering:        {offering_name}
Provider:        {provider_name}
Duration:        {duration_hours} hours
Start Date:      {start_date}
End Date:        {end_date}
Contract ID:     {contract_id}

View your contract: {contract_url}

───────────────────────────────────
This is a payment receipt, not a tax invoice.
For a tax invoice, visit your dashboard or contact support.

Decent Cloud
{legal_entity_details}
```

### Invoice PDF Layout (EU VAT Compliant)

```
┌─────────────────────────────────────────────────────────────────┐
│                           INVOICE                                │
│                                                                  │
│  Invoice Number: INV-2025-000123                                │
│  Invoice Date:   2025-12-07                                     │
│  Due Date:       Paid                                           │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  FROM                          TO                                │
│  ────                          ──                                │
│  Decent Cloud Ltd              {customer_name}                   │
│  {company_address}             {customer_address}                │
│  VAT: {our_vat_id}             VAT: {customer_vat_id}           │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│  DESCRIPTION                      QTY    UNIT PRICE    AMOUNT   │
│  ───────────────────────────────────────────────────────────────│
│  {offering_name}                                                 │
│  Provider: {provider_name}                                       │
│  Duration: {duration} hours                                      │
│  Period: {start_date} - {end_date}       1    {price}   {price} │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                                          Subtotal:    {subtotal} │
│                                          VAT ({rate}%): {vat}    │
│                                          ─────────────────────── │
│                                          TOTAL:       {total}    │
│                                                                  │
│  Payment Status: PAID                                            │
│  Payment Method: {payment_method}                                │
│  Transaction ID: {transaction_id}                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Database Migrations

**Receipt tracking:**
```sql
CREATE TABLE receipt_sequence (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    next_number INTEGER NOT NULL DEFAULT 1
);
INSERT INTO receipt_sequence (id, next_number) VALUES (1, 1);

ALTER TABLE contract_sign_requests ADD COLUMN receipt_number INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN receipt_sent_at_ns INTEGER;
```

**Invoice storage:**
```sql
CREATE TABLE invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL UNIQUE,
    invoice_number TEXT NOT NULL UNIQUE,
    invoice_date_ns INTEGER NOT NULL,
    seller_name TEXT NOT NULL,
    seller_address TEXT NOT NULL,
    seller_vat_id TEXT,
    buyer_name TEXT,
    buyer_address TEXT,
    buyer_vat_id TEXT,
    subtotal_e9s INTEGER NOT NULL,
    vat_rate_percent INTEGER NOT NULL DEFAULT 0,
    vat_amount_e9s INTEGER NOT NULL DEFAULT 0,
    total_e9s INTEGER NOT NULL,
    currency TEXT NOT NULL,
    pdf_blob BLOB,
    pdf_generated_at_ns INTEGER,
    created_at_ns INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);

CREATE TABLE invoice_sequence (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    year INTEGER NOT NULL,
    next_number INTEGER NOT NULL DEFAULT 1
);
```

**Tax columns on contracts:**
```sql
ALTER TABLE contract_sign_requests ADD COLUMN tax_amount_e9s INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN tax_rate_percent REAL;
ALTER TABLE contract_sign_requests ADD COLUMN tax_type TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN tax_jurisdiction TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN customer_tax_id TEXT;
ALTER TABLE contract_sign_requests ADD COLUMN reverse_charge INTEGER DEFAULT 0;
```

### Open Questions (Resolved)

1. **Legal entity:** Stub with "Decent Cloud Ltd" placeholder
2. **VAT registration:** Prepare for EU VAT but don't hardcode countries
3. **ICPay tax:** "Tax not included" disclaimer (per TODO.md)
4. **Invoice language:** English only
5. **Retention:** Store indefinitely in database
