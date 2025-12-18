# Account-Based User Identification Migration

**Date:** 2025-12-18
**Status:** In Progress
**Priority:** HIGH

## Problem Statement

The codebase has a dual-system problem where user identification is split between:

1. **Account system (new):** `accounts` table with usernames, linked to N public keys via `account_public_keys`
2. **Legacy system:** Everything else (contracts, offerings, providers, URLs) uses raw 32-byte pubkeys

**Impact:**
- URLs are ugly and unmemorable: `/dashboard/user/abc123...` vs `/dashboard/user/alice`
- Users cannot be found by username - must know their pubkey
- Contract history is per-key, not per-account (new device = fresh history)
- Provider profiles not linked to accounts
- Multi-device UX is broken

## Solution Overview

Link all entities to accounts instead of raw pubkeys. This enables:
- Username-based URLs
- Multi-device support (all keys under same account see same data)
- Proper account-centric data model

## Implementation Phases

### Phase 1: Database Schema Changes ✅

**Migration:** `050_account_based_identification.sql`

```sql
-- Add account_id to provider_profiles
ALTER TABLE provider_profiles ADD COLUMN account_id BLOB REFERENCES accounts(id);
CREATE INDEX idx_provider_profiles_account ON provider_profiles(account_id);

-- Add account_id to provider_offerings
ALTER TABLE provider_offerings ADD COLUMN account_id BLOB REFERENCES accounts(id);
CREATE INDEX idx_provider_offerings_account ON provider_offerings(account_id);

-- Add account_id columns to contracts
ALTER TABLE contract_sign_requests ADD COLUMN requester_account_id BLOB REFERENCES accounts(id);
ALTER TABLE contract_sign_requests ADD COLUMN provider_account_id BLOB REFERENCES accounts(id);
CREATE INDEX idx_contracts_requester_account ON contract_sign_requests(requester_account_id);
CREATE INDEX idx_contracts_provider_account ON contract_sign_requests(provider_account_id);
```

### Phase 2: Backend Changes

#### 2.1 Data Backfill Function (TODO)
- [ ] Add `Database::backfill_account_ids()` to run on startup
- [ ] For each pubkey without account_id:
  - Look up account via `get_account_id_by_public_key()`
  - If found: set account_id
  - If not found: auto-create account with generated username

Note: Implement when Phase 2.4 API endpoints are built.

#### 2.2 Auto-Create Accounts for Orphan Pubkeys ✅
- [x] Add `Database::ensure_account_for_pubkey()`
- [x] Generate username from pubkey prefix: `user_<first8chars>`
- [x] Create account with pubkey linked
- [ ] Called during backfill and when new providers/contracts are created (TODO)

#### 2.3 Update Database Queries (TODO)
Implement these when API endpoints (2.4) are built to avoid dead code:
- [ ] `providers.rs`: Add account-based lookups
  - `get_provider_profile_by_account_id()`
  - `get_provider_profile_by_username()`
  - `get_username_for_provider_pubkey()`
  - Update `update_provider_onboarding()` to set account_id
- [ ] `contracts.rs`: Add account-based lookups
  - `get_user_contracts_by_account_id()`
  - `get_provider_contracts_by_account_id()`
  - `get_user_contracts_by_username()`
  - `get_provider_contracts_by_username()`
  - Update `create_rental_request()` to set account_ids
- [ ] `offerings.rs`: Add account-based lookups
  - `get_offerings_by_account_id()`
  - `get_offerings_by_username()`

#### 2.4 API Endpoints (TODO)
- [ ] Add username-based endpoints:
  - `GET /api/v1/accounts/{username}` - Public profile
  - `GET /api/v1/accounts/{username}/offerings` - User's offerings
  - `GET /api/v1/accounts/{username}/contracts` - User's contracts (if public)
  - `GET /api/v1/accounts/{username}/reputation` - User's reputation

### Phase 3: Frontend Changes

#### 3.1 Update Route Structure
- [ ] Change `/dashboard/user/[pubkey]/` to `/dashboard/user/[username]/`
- [ ] Change `/dashboard/reputation/[pubkey]/` to `/dashboard/reputation/[username]/`
- [ ] Add short URL: `/u/[username]` → public profile

#### 3.2 Update API Calls
- [ ] `api.ts`: Add username-based API functions
- [ ] Update all components using pubkey in URLs to use username

#### 3.3 Update Link Generation
- [ ] `identity.ts`: Add `getUserProfileUrl(username)`
- [ ] Update all `href` attributes to use username-based URLs
- [ ] Update marketplace offering cards to link to username

### Phase 4: Generated Types

- [ ] Regenerate TypeScript types from Rust
- [ ] Update frontend to use new types

## Verification Checklist

After implementation, verify:

- [ ] `cargo clippy --tests` passes with no warnings
- [ ] `cargo nextest run` passes all tests
- [ ] Migration applies cleanly
- [ ] Existing data is backfilled correctly
- [ ] New providers get account_id set on registration
- [ ] New contracts get account_ids set on creation
- [ ] Username-based URLs work
- [ ] Old pubkey URLs redirect to username URLs (or show 404)
- [ ] Multi-device scenario works (same account, different keys, same data)

## Files to Modify

### Backend
- `api/migrations/050_account_based_identification.sql` ✅
- `api/src/database/providers.rs`
- `api/src/database/contracts.rs`
- `api/src/database/offerings.rs`
- `api/src/database/accounts.rs` (add ensure_account_for_pubkey)
- `api/src/openapi/accounts.rs` (add username endpoints)

### Frontend
- `website/src/routes/dashboard/user/[pubkey]/+page.svelte` → `[username]/`
- `website/src/routes/dashboard/reputation/[pubkey]/+page.svelte` → `[username]/`
- `website/src/lib/services/api.ts`
- `website/src/lib/utils/identity.ts`
- `website/src/lib/components/AccountOverview.svelte`
- All components generating user profile links

## Rollback Plan

If issues arise:
1. Drop the new columns (requires new migration)
2. Frontend routes still work if we keep pubkey support as fallback
3. API endpoints can be versioned to maintain backward compat

## Notes

- No legacy data to preserve (confirmed by user)
- No backward compatibility needed for URLs (no existing links to preserve)
- Auto-create accounts with generated usernames to avoid migration friction
