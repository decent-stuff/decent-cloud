# Support Portal Access Reset

**Status:** Complete
**Date:** 2025-12-04

## Summary

Enable providers to reset their Chatwoot support portal password via our API. Migrate from Account API to Platform API for user management.

## Requirements

### Must-have
- [x] Providers can request support portal access reset via authenticated API
- [x] System generates secure password and updates Chatwoot user
- [x] Password is emailed to provider
- [x] Store Chatwoot user ID in database for future operations
- [x] Migrate agent creation from Account API to Platform API

### Nice-to-have
- [ ] Re-create agent if user ID not found (backward compatibility)

## Technical Design

### Current Flow (Account API) - DEPRECATED
```
Provider registers → create_agent(email, name) via /api/v1/accounts/{id}/agents
                   → Returns agent_id (NOT user_id)
                   → Cannot reset password
```

### New Flow (Platform API)
```
Provider registers → create_user(email, name, password) via /platform/api/v1/users
                   → Returns user_id
                   → Store user_id in accounts table
                   → Add user to account via /platform/api/v1/accounts/{id}/account_users

Password reset    → Generate secure password
                  → PATCH /platform/api/v1/users/{user_id} with new password
                  → Email password to provider
```

### New Environment Variable
```
CHATWOOT_PLATFORM_API_TOKEN - Token from Platform App (SuperAdmin → Platform Apps → New)
```

### Database Changes
New column in `accounts` table:
```sql
ALTER TABLE accounts ADD COLUMN chatwoot_user_id INTEGER;
```

### API Endpoint
```
POST /api/v1/chatwoot/support-access/reset
- Requires authentication (provider must be logged in)
- Returns: { success: true, data: "New password sent to your email address." }
```

## Steps

### Step 1: Database Migration
**Success:** Migration adds `chatwoot_user_id` column to accounts table
**Status:** Complete

### Step 2: Extend ChatwootClient with Platform API
**Success:** Client can create users, add to account, update password via Platform API
**Status:** Complete

### Step 3: Modify Agent Creation Flow
**Success:** `create_provider_agent` uses Platform API, stores user_id in DB
**Status:** Complete

### Step 4: Add Password Reset Endpoint
**Success:** `POST /chatwoot/support-access/reset` works with tests
**Status:** Complete

### Step 5: Update Environment Configuration
**Success:** `.env.example` documents new `CHATWOOT_PLATFORM_API_TOKEN`
**Status:** Complete

### Step 6: Clean Up Deprecated Code
**Success:** Remove unused Account API agent methods, update tests
**Status:** Complete

## Execution Log

### Step 1
- **Implementation:** Added migration `027_chatwoot_user_id.sql`, updated Account struct and queries
- **Review:** All SELECT queries updated to include new column
- **Outcome:** Success - migration applies, column accessible

### Step 2
- **Implementation:** Added `ChatwootPlatformClient` with `create_user`, `add_user_to_account`, `update_user_password`
- **Review:** Follows existing client patterns, proper error handling
- **Outcome:** Success - new client exported from module

### Step 3
- **Implementation:** Modified `create_provider_agent` to use Platform API, store user_id
- **Review:** Password generation meets Chatwoot requirements (16 chars, mixed case, numbers, special)
- **Outcome:** Success - agent creation stores user ID for future resets

### Step 4
- **Implementation:** Added `POST /chatwoot/support-access/reset` endpoint
- **Review:** Proper auth, error handling, email queueing
- **Outcome:** Success - endpoint generates password, updates Chatwoot, emails user

### Step 5
- **Implementation:** Updated `.env.example` with `CHATWOOT_PLATFORM_API_TOKEN` and setup instructions
- **Review:** Clear documentation with step numbers
- **Outcome:** Success - configuration documented

### Step 6
- **Implementation:** Removed `CreateAgentRequest`, `AgentResponse`, `create_agent` method
- **Review:** Added Platform API client tests, updated test_helpers with new migration
- **Outcome:** Success - 779 tests pass, no new clippy warnings

## Completion Summary
**Completed:** 2025-12-04 | **Agents:** 1 (self) | **Steps:** 6/6

**Changes:**
- Files: 9 modified, 2 new
- New: `027_chatwoot_user_id.sql`, spec doc
- Modified: `client.rs`, `integration.rs`, `mod.rs`, `tests.rs`, `accounts.rs`, `chatwoot.rs` (openapi), `test_helpers.rs`, `.env.example`

**Requirements:** 5/5 must-have complete, 0/1 nice-to-have (deferred)

**Tests:** 779 pass, cargo clippy clean (pre-existing warnings only)

**Notes:**
- Platform API requires one-time setup: Create Platform App in Chatwoot SuperAdmin console
- SSO feature was Enterprise-only, so we use direct password update approach
- Password emailed to user with instructions to change after login
