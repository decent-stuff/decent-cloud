# API Implementation TODO

## Current Status

✅ Basic poem server running with health endpoint
✅ Placeholder canister proxy endpoint structure
✅ Docker deployment configuration

## Canister Proxy Implementation

The API needs to proxy the following ICP canister methods (from `website/lib/cf-service.ts`):

### Provider Operations
- `provider_register_anonymous(pubkey_bytes, crypto_signature, caller_principal)` → ResultString
- `provider_update_profile_anonymous(pubkey_bytes, profile_serialized, crypto_signature, caller_principal)` → ResultString
- `provider_update_offering_anonymous(pubkey_bytes, offering_serialized, crypto_signature, caller_principal)` → ResultString
- `provider_list_checked_in()` → ResultString
- `provider_get_profile_by_pubkey_bytes(pubkey_bytes)` → string | null
- `provider_get_profile_by_principal(principal)` → string | null

### Offering Operations
- `offering_search(search_query)` → Array<{provider_pub_key: number[], offering_compressed: number[]}>

### Contract Operations
- `contract_sign_request_anonymous(pubkey_bytes, contract_info_serialized, crypto_signature, caller_principal)` → ResultString
- `contracts_list_pending(pubkey_bytes?)` → Array<[number[], number[]]>
- `contract_sign_reply_anonymous(pubkey_bytes, contract_reply_serialized, crypto_signature, caller_principal)` → ResultString

### User Operations
- `user_register_anonymous(pubkey_bytes, crypto_signature, caller_principal)` → ResultString

### Check-in Operations
- `get_check_in_nonce()` → number[]
- `provider_check_in_anonymous(pubkey_bytes, memo, nonce_crypto_signature, caller_principal)` → ResultString

### Common Operations
- `get_identity_reputation(pubkey_bytes)` → bigint (as string)
- `get_registration_fee()` → bigint (as string)

## Implementation Steps

1. **Add ICP dependencies** to `Cargo.toml`:
   ```toml
   ic-agent = "0.38"
   ic-utils = "0.38"
   candid = "0.10"
   ```

2. **Create canister module** (`src/canister.rs`):
   - Initialize ic-agent with canister ID from environment
   - Implement method call wrapper
   - Handle Result<T, E> → CFResponse<T> conversion

3. **Implement each canister method** in `src/main.rs`:
   - Parse args from JSON
   - Call canister method via agent
   - Return proper CFResponse format

4. **Add error handling**:
   - Network errors
   - Canister rejection errors
   - Serialization errors

5. **Add tests**:
   - Unit tests for arg parsing
   - Integration tests with local replica (optional)

## Response Format

All endpoints must return:
```json
{
  "success": true,
  "data": <result>
}
```

Or on error:
```json
{
  "success": false,
  "error": "error message"
}
```

## Environment Variables

Required:
- `CANISTER_ID`: ICP canister ID to proxy to
- `IC_NETWORK`: Network URL (e.g., "https://ic0.app" for mainnet)

Optional:
- `RUST_LOG`: Logging level
- `PORT`: API server port (default: 8080)
- `ENVIRONMENT`: deployment environment
