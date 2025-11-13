# Marketplace Rental Flow - Implementation Plan

## Overview
Transform the marketplace "Deploy Now" button into a complete resource rental system with request management, provider acceptance, provisioning workflow, and user notifications.

## Current State Analysis

### Existing Infrastructure
- **Offerings**: Provider resources with details (compute, storage, network)
- **Contracts**: Database tables for `contract_sign_requests` and `contract_sign_replies`
- **Contract Status**: Currently only "pending" status set during blockchain sync
- **Auth System**: Seed-based identity with public/private key pairs
- **API**: Handlers exist for viewing contracts but no creation endpoint for users

### Missing Pieces
1. User-initiated contract request API endpoint
2. Contract status state machine (requested → accepted → provisioned → active)
3. Provider acceptance/rejection workflow
4. Provisioning status tracking
5. User notifications system

## Proposed Status Flow

```
┌─────────────┐
│  requested  │  ← User clicks "Rent" button
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  pending    │  ← Awaiting provider response
└──────┬──────┘
       │
       ├─────YES────▶ ┌──────────┐
       │               │ accepted │  ← Provider approves
       │               └────┬─────┘
       │                    │
       │                    ▼
       │              ┌──────────────┐
       │              │ provisioning │  ← Provider working on setup
       │              └───────┬──────┘
       │                      │
       │                      ▼
       │              ┌──────────────┐
       │              │ provisioned  │  ← Resource ready
       │              └───────┬──────┘
       │                      │
       │                      ▼
       │              ┌──────────────┐
       │              │   active     │  ← User using resource
       │              └──────────────┘
       │
       └─────NO─────▶ ┌──────────┐
                       │ rejected │  ← Provider declines
                       └──────────┘
```

## Implementation Steps

### Phase 1: Backend - Contract Request API

**File**: `api/src/api_handlers.rs`

Add new endpoint:
```rust
#[handler]
pub async fn create_rental_request(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Json(req): Json<CreateRentalRequest>
) -> PoemResult<Json<ApiResponse<String>>>
```

**File**: `api/src/database/contracts.rs`

Add method:
```rust
pub async fn create_rental_request(
    &self,
    requester_pubkey: &[u8],
    offering_id: i64,
    params: RentalRequestParams
) -> Result<Vec<u8>>  // Returns contract_id
```

Fields needed:
- `requester_pubkey_hash` (from auth)
- `requester_ssh_pubkey` (from request or user profile)
- `requester_contact` (from request or user profile)
- `provider_pubkey_hash` (from offering)
- `offering_id`
- `payment_amount_e9s`
- `request_memo`
- Initial status: **"requested"**

### Phase 2: Provider Management API

**File**: `api/src/api_handlers.rs`

Add endpoints:
```rust
// Get pending requests for provider
#[handler]
pub async fn get_pending_rental_requests(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>
) -> PoemResult<Json<ApiResponse<Vec<Contract>>>>

// Accept/reject request
#[handler]
pub async fn respond_to_rental_request(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(contract_id): Path<String>,
    Json(req): Json<RentalResponseRequest>
) -> PoemResult<Json<ApiResponse<()>>>

// Update provisioning status
#[handler]
pub async fn update_provisioning_status(
    auth: AuthenticatedUser,
    db: Data<&Arc<Database>>,
    Path(contract_id): Path<String>,
    Json(req): Json<ProvisioningStatusRequest>
) -> PoemResult<Json<ApiResponse<()>>>
```

**File**: `api/src/database/contracts.rs`

Add methods:
```rust
pub async fn update_contract_status(
    &self,
    contract_id: &[u8],
    new_status: &str,
    updated_by_pubkey: &[u8]
) -> Result<()>

pub async fn add_provisioning_details(
    &self,
    contract_id: &[u8],
    instance_details: &str
) -> Result<()>
```

### Phase 3: Frontend - Marketplace Button Update

**File**: `website/src/routes/dashboard/marketplace/+page.svelte`

Changes:
1. Rename button: "Deploy Now" → **"Rent Resource"** (or "Request Access")
2. Add click handler
3. Show rental request dialog with:
   - Resource summary
   - SSH public key input (optional, from profile)
   - Contact method selection
   - Price confirmation
   - Submit button

```svelte
<button
    onclick={() => handleRentClick(offering)}
    class="..."
>
    🚀 Rent Resource
</button>
```

### Phase 4: Frontend - Rental Request Dialog

**New File**: `website/src/lib/components/RentalRequestDialog.svelte`

Features:
- Display offering details
- SSH key selector (from user's stored keys)
- Contact method selector (email, Matrix, etc.)
- Optional memo field
- Price display & confirmation
- Submit → calls API to create rental request

### Phase 5: Frontend - User Dashboard

**New File**: `website/src/routes/dashboard/rentals/+page.svelte`

User view:
- List of rental requests with status badges
- Status indicators:
  - 🟡 Requested
  - 🔵 Pending
  - 🟢 Accepted
  - ⚙️ Provisioning
  - ✅ Provisioned/Active
  - 🔴 Rejected
- Click to view details (credentials, connection info)

### Phase 6: Frontend - Provider Dashboard

**New File**: `website/src/routes/dashboard/provider/requests/+page.svelte`

Provider view:
- Pending requests list
- Accept/Reject buttons
- Status update form for provisioning workflow:
  - "Start Provisioning" (requested → provisioning)
  - "Mark as Provisioned" + instance details form
  - Instance details: IP address, credentials, connection instructions

### Phase 7: Database Schema Updates

**New Migration**: `api/migrations/XXX_rental_workflow.sql`

Add columns to `contract_sign_requests`:
```sql
ALTER TABLE contract_sign_requests ADD COLUMN status_updated_at_ns INTEGER;
ALTER TABLE contract_sign_requests ADD COLUMN status_updated_by BLOB;
```

Create provisioning details table:
```sql
CREATE TABLE IF NOT EXISTS contract_provisioning_details (
    contract_id BLOB PRIMARY KEY,
    instance_ip TEXT,
    instance_credentials TEXT,  -- encrypted?
    connection_instructions TEXT,
    provisioned_at_ns INTEGER NOT NULL,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);
```

Create status history table (optional but recommended):
```sql
CREATE TABLE IF NOT EXISTS contract_status_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    contract_id BLOB NOT NULL,
    old_status TEXT NOT NULL,
    new_status TEXT NOT NULL,
    changed_by BLOB NOT NULL,
    changed_at_ns INTEGER NOT NULL,
    change_memo TEXT,
    FOREIGN KEY (contract_id) REFERENCES contract_sign_requests(contract_id)
);
```

## Status Definitions

| Status         | Description            | Who can set     | Next states          |
|----------------|------------------------|-----------------|----------------------|
| `requested`    | User initiated request | User            | pending, rejected    |
| `pending`      | Waiting for provider   | System          | accepted, rejected   |
| `accepted`     | Provider approved      | Provider        | provisioning         |
| `rejected`     | Provider declined      | Provider        | (terminal)           |
| `provisioning` | Setup in progress      | Provider        | provisioned          |
| `provisioned`  | Resource ready         | Provider        | active               |
| `active`       | User is using resource | System/Provider | completed, cancelled |
| `completed`    | Rental ended normally  | System          | (terminal)           |
| `cancelled`    | Rental cancelled early | User/Provider   | (terminal)           |

## Additional Considerations

### Security
- Verify authorization: users can only request for themselves
- Verify authorization: providers can only respond to their own offerings
- Rate limiting on rental requests
- Input validation on all fields

### Notifications (Future Enhancement)
- Email/webhook when request is accepted
- Email/webhook when resource is provisioned
- In-app notification system

### Payment Integration (Future)
- Currently storing `payment_amount_e9s` but not enforcing
- Future: integrate with ICP ledger for actual payments
- Escrow mechanism for deposits

### Resource Lifecycle (Future)
- Auto-renewal
- Expiration handling
- Resource cleanup
- Contract extension requests

## Files to Create/Modify

### API (Rust)
- [ ] `api/src/api_handlers.rs` - Add 4 new endpoints
- [ ] `api/src/database/contracts.rs` - Add 4 new methods
- [ ] `api/migrations/XXX_rental_workflow.sql` - New migration
- [ ] `api/src/main.rs` - Register new routes

### Website (Svelte)
- [ ] `website/src/routes/dashboard/marketplace/+page.svelte` - Update button
- [ ] `website/src/lib/components/RentalRequestDialog.svelte` - New dialog
- [ ] `website/src/routes/dashboard/rentals/+page.svelte` - User rentals page
- [ ] `website/src/routes/dashboard/provider/requests/+page.svelte` - Provider requests page
- [ ] `website/src/lib/services/contracts-api.ts` - API client functions
- [ ] `website/src/lib/components/DashboardSidebar.svelte` - Add navigation links

## Testing Requirements

### Unit Tests
- Contract creation with validation
- Status transitions (valid/invalid)
- Authorization checks

### Integration Tests
- Full rental flow: request → accept → provision → active
- Rejection flow
- Multiple simultaneous requests

### Manual Testing
1. User requests resource
2. Provider sees pending request
3. Provider accepts request
4. Provider updates to "provisioning"
5. Provider marks as "provisioned" with details
6. User sees provisioned resource with connection info

## Future Enhancements
- Multi-resource bundles (e.g., compute + storage)
- Auto-scaling contracts
- SLA tracking
- Automated health checks
- Review/rating system after rental completion
