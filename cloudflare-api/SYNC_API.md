# Ledger Synchronization API

This document describes the ledger synchronization features added to the Cloudflare API.

## Overview

The CF API now includes a real ICP agent that can communicate with the Decent Cloud canister to import ledger data into the D1 database. This allows the CF service to act as a cache/replica of the canister data.

## Architecture

### Components

1. **ICP Agent** (`src/services/icp-agent.ts`)
   - Real connection to ICP canister using @dfinity/agent
   - Supports query calls to canister endpoints
   - Uses the `next_block_entries` endpoint for efficient data sync

2. **Ledger Import Service** (`src/services/ledger-import.ts`)
   - Orchestrates the import of ledger data from canister to D1
   - Processes different label types (ProvProfile, ProvOffering, etc.)
   - Maintains sync status and supports resumable imports

3. **Sync Routes** (`src/routes/sync.ts`)
   - RESTful API endpoints to trigger and monitor imports

## API Endpoints

### Import All Ledger Data

```bash
POST /api/sync/import
Content-Type: application/json

{
  "batchSize": 100  # Optional, default 100
}
```

Response:
```json
{
  "success": true,
  "data": {
    "message": "Ledger import completed",
    "stats": {
      "totalProcessed": 150,
      "usersCreated": 45,
      "profilesCreated": 40,
      "offeringsCreated": 65,
      "ledgerEntriesCreated": 150,
      "errors": []
    }
  }
}
```

### Import Specific Label

```bash
POST /api/sync/import/ProvProfile
Content-Type: application/json

{
  "batchSize": 50
}
```

Supported labels:
- `ProvRegister` - Provider registrations
- `UserRegister` - User registrations
- `ProvProfile` - Provider profiles
- `ProvOffering` - Provider offerings
- `ContractSignRequest` - Contract requests
- `RewardDistribution` - Reward distributions

### Get Sync Status

```bash
GET /api/sync/status
```

Response:
```json
{
  "success": true,
  "data": {
    "statuses": [
      {
        "tableName": "all",
        "lastSyncedBlockOffset": 150,
        "lastSyncedAt": "2025-11-03T12:34:56.789Z",
        "totalRecordsSynced": 150,
        "syncErrors": 0,
        "lastError": null
      }
    ],
    "totalTables": 1
  }
}
```

## Schema Verification

The DB schema in `migrations/0001_initial_schema.sql` matches the canister data structure:

### Matching Tables

| DB Table | Canister Label | Status |
|----------|----------------|--------|
| `dc_users` | `ProvRegister`, `UserRegister` | ✅ Matches |
| `provider_profiles` | `ProvProfile` | ✅ Matches |
| `provider_offerings` | `ProvOffering` | ✅ Matches |
| `ledger_entries` | All labels | ✅ Matches |
| `contract_signatures` | `ContractSignRequest` | 🚧 TODO |
| `reputation_changes` | `RewardDistribution` | 🚧 TODO |
| `token_transfers` | ICRC-1 transfers | 🚧 TODO |

### Key Differences

1. **Block Offset**: The `next_block_entries` endpoint doesn't provide block offsets directly. Currently stored as 0.
2. **Timestamps**: Using current timestamp instead of ICP timestamp for now.
3. **Signature Extraction**: Signatures are stored with the profile/offering data blob.

## Usage Example

1. **Start the local development server**:
   ```bash
   npm run dev
   ```

2. **Run migrations**:
   ```bash
   npm run d1:migrate:test
   ```

3. **Trigger a sync**:
   ```bash
   curl -X POST http://localhost:8787/api/sync/import \
     -H "Content-Type: application/json" \
     -d '{"batchSize": 100}'
   ```

4. **Check status**:
   ```bash
   curl http://localhost:8787/api/sync/status
   ```

## Configuration

Set the following environment variables in `wrangler.toml`:

```toml
[env.local.vars]
ENVIRONMENT = "development"
CANISTER_ID = "ggi4a-wyaaa-aaaai-actqq-cai"
FLUSH_INTERVAL_SECONDS = "5"
MAX_RETRY_ATTEMPTS = "3"
```

For production, use the mainnet canister ID and set `ENVIRONMENT = "production"`.

## Implementation Notes

### Resumable Imports

The import service tracks progress in the `sync_status` table. If an import is interrupted, it will resume from the last successfully processed offset.

### Error Handling

- Failed entry processing is logged but doesn't stop the import
- Errors are tracked in the sync_status table
- Use the `/api/sync/status` endpoint to monitor errors

### Performance

- Default batch size is 100 entries
- Imports run synchronously (blocking request)
- For large imports, consider implementing a queue or background job system

## Future Improvements

1. Add ICRC-3 `get_blocks` support for better block tracking
2. Implement contract and reward processing
3. Add incremental sync (only fetch new data since last sync)
4. Support background/scheduled sync jobs
5. Add webhook notifications for sync completion
