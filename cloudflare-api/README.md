# Cloudflare API for Decent Cloud

A Cloudflare Workers service that provides a caching layer for Decent Cloud with D1 database and periodic synchronization to the ICP canister.

## Quick Start

```bash
cd cloudflare-api
npm install
npm run dev
```

The service will start on `http://localhost:8787`

## Architecture

```
Web Client → Cloudflare Worker → D1 (cache) → ICP Canister (periodic sync)
```

## Key Features

- **Binary Payload Compatibility**: All `Vec<u8>` data preserved exactly as in canister
- **Canister-Compatible Interface**: Same method signatures and response formats
- **D1 Caching Layer**: Immediate local storage with periodic flush to canister
- **Anonymous Access**: No dependency on `ic_cdk::caller()`

## API Endpoints

### Core Decent Cloud APIs
```
GET  /api/v1/health
POST /api/v1/canister/{method}
```

Available methods: `provider_register`, `provider_update_profile`, `provider_get_profile_by_pubkey_bytes`, `offering_search`, `user_register`, `get_registration_fee`, and more.

### Monitoring APIs
```
GET /api/v1/monitoring/health
GET /api/v1/monitoring/dashboard
```

## Testing

```bash
npm test                    # Run all tests
npm test -- canister       # Run canister compatibility tests
```

## Configuration

Edit `wrangler.toml`:
```toml
[vars]
ENVIRONMENT = "local"
CANISTER_ID = "ggi4a-wyaaa-aaaai-actqq-cai"
```

## Deployment

```bash
npm run deploy             # Deploy to Cloudflare Workers
```

## Database Schema

Core tables: `dc_users`, `provider_profiles`, `provider_offerings`, `ledger_entries`, `reputation_changes`, `token_transfers`, `contract_signatures`, `sync_status`.

## Migration Status

**Phase 1**: ✅ Dual-read architecture with D1 as read-through cache. Source of truth is in the ICP canister, but all data is cached in CF D1.
**Next**: Real ICP integration and automated sync. Source of truth is still in the ICP, but data is served and cached by CF D1. Data is periodically written back to the ICP canister.
