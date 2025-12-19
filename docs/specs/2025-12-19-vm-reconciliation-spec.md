# VM Reconciliation API Spec

## Overview

Replace the status-driven termination approach with a reconciliation-based approach where dc-agent reports running VMs and API decides which should be terminated.

## Current Behavior (Status-Driven)

```
dc-agent polls GET /providers/:pubkey/contracts/pending-termination
API returns contracts where status='cancelled' AND instance_details IS NOT NULL AND terminated_at_ns IS NULL
dc-agent terminates those VMs
```

**Problems:**
1. Only handles cancelled contracts, not expired ones
2. dc-agent must understand contract lifecycle states
3. No orphan VM detection
4. Tightly coupled to contract status model

## New Behavior (Reconciliation-Driven)

```
dc-agent: POST /providers/:pubkey/reconcile with list of running VMs
API: Returns which VMs to keep, terminate, or flag as unknown
```

**Benefits:**
1. Single source of truth - API decides based on ALL factors
2. Handles expired contracts (end_timestamp_ns < now)
3. Handles cancelled contracts
4. Handles payment failures (future)
5. Detects orphan VMs (running but no matching contract)
6. Stateless dc-agent - doesn't care WHY termination is needed
7. Multi-agent support - each reports its own VMs

## API Endpoint

### POST /api/v1/providers/:pubkey/reconcile

**Request:**
```json
{
  "runningInstances": [
    {
      "externalId": "vm-12345",
      "contractId": "abc123def456..."
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "keep": [
      {
        "externalId": "vm-12345",
        "contractId": "abc123def456...",
        "endsAt": 1734567890000000000
      }
    ],
    "terminate": [
      {
        "externalId": "vm-67890",
        "contractId": "def789abc123...",
        "reason": "expired"
      }
    ],
    "unknown": [
      {
        "externalId": "vm-orphan",
        "message": "No matching contract found"
      }
    ]
  }
}
```

**Termination Reasons:**
- `expired` - Contract end_timestamp_ns has passed
- `cancelled` - Contract was explicitly cancelled
- `payment_failed` - Payment did not complete (future)

## Database Changes

No schema changes needed. The reconciliation logic uses existing fields:
- `end_timestamp_ns` - For expiry detection
- `status` - For cancellation detection
- `provisioning_instance_details` - For matching running VMs to contracts

## dc-agent Changes

1. **Query running VMs from Proxmox** - New `list_vms()` method on provisioner
2. **Replace termination polling with reconciliation loop**
3. **Terminate VMs API says to terminate**
4. **Log warnings for unknown (orphan) VMs**

### New Provisioner Trait Method

```rust
/// List all running VMs managed by this agent
async fn list_running_instances(&self) -> Result<Vec<RunningInstance>>;

pub struct RunningInstance {
    pub external_id: String,
    pub contract_id: Option<String>,  // Extracted from VM name/tags if possible
}
```

### Reconciliation Loop

```rust
async fn reconcile(&self) -> Result<()> {
    // 1. Get running VMs from provisioner
    let running = provisioner.list_running_instances().await?;

    // 2. Call reconcile API
    let response = api_client.reconcile(&running).await?;

    // 3. Terminate what API says
    for vm in response.terminate {
        info!(external_id = %vm.external_id, reason = %vm.reason, "Terminating VM");
        provisioner.terminate(&vm.external_id).await?;
        api_client.report_terminated(&vm.contract_id).await?;
    }

    // 4. Warn about orphans
    for vm in response.unknown {
        warn!(external_id = %vm.external_id, "Orphan VM detected - no matching contract");
    }
}
```

## VM Naming Convention

To enable matching VMs to contracts, VMs are named with the contract ID:
- Proxmox: `dc-{contract_id_prefix}` (e.g., `dc-abc123de`)
- This allows extracting contract_id from running VM name

## Migration Path

1. Keep existing `pending-termination` endpoint temporarily (deprecated)
2. Add new `reconcile` endpoint
3. Update dc-agent to use reconciliation
4. Remove old endpoint after all agents updated

## Testing

1. **API tests:**
   - Reconcile with active contract - returns in `keep`
   - Reconcile with expired contract - returns in `terminate` with reason `expired`
   - Reconcile with cancelled contract - returns in `terminate` with reason `cancelled`
   - Reconcile with unknown VM - returns in `unknown`
   - Mixed scenarios

2. **dc-agent tests:**
   - Mock Proxmox VM listing
   - Mock reconcile API response
   - Verify termination calls
   - Verify warning for orphans

## Implementation Status

✅ **Completed (2025-12-19):**

1. `RunningInstance` struct and `list_running_instances()` trait method in `dc-agent/src/provisioner/mod.rs`
2. `ProxmoxProvisioner` implementation with `list_vms()` and VM filtering in `dc-agent/src/provisioner/proxmox.rs`
3. Reconcile API endpoint `POST /api/v1/providers/:pubkey/reconcile` in `api/src/openapi/providers.rs`
4. Database query for contracts needing termination (expired OR cancelled) in `api/src/database/contracts.rs`
5. dc-agent reconciliation loop replacing old termination polling in `dc-agent/src/main.rs`
6. API client `reconcile()` method in `dc-agent/src/api_client.rs`
7. Tests: 5 new Proxmox tests, API reconcile endpoint tests

**Files Changed:**
- `api/src/openapi/providers.rs` - New reconcile endpoint
- `api/src/database/contracts.rs` - Contract lookup queries
- `dc-agent/src/provisioner/mod.rs` - `RunningInstance` type and trait method
- `dc-agent/src/provisioner/proxmox.rs` - VM listing implementation
- `dc-agent/src/api_client.rs` - Reconcile API client
- `dc-agent/src/main.rs` - Reconciliation loop
- `dc-agent/src/provisioner/proxmox_tests.rs` - New tests

## Implementation Order

1. ✅ Add `list_running_instances()` to Provisioner trait
2. ✅ Implement for ProxmoxProvisioner
3. ✅ Add reconcile API endpoint
4. ✅ Add database query for contract lookup
5. ✅ Update dc-agent reconciliation loop
6. ✅ Add tests
7. Deprecate old endpoint (future cleanup)
