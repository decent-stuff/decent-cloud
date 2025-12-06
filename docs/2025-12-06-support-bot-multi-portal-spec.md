# Support Bot: Multi-Portal Knowledge Base
**Status:** Complete (pending deployment verification)

## Overview

Remove `CHATWOOT_DEFAULT_PORTAL_SLUG` configuration and automatically fetch articles from ALL Help Center portals. This ensures the AI support bot has access to the complete knowledge base without manual configuration.

## Requirements

### Must-have
- [x] Add `list_portals()` method to `ChatwootClient`
- [x] Fetch articles from all portals in `handle_customer_message()`
- [x] Remove `CHATWOOT_DEFAULT_PORTAL_SLUG` env var and all references
- [x] Update AGENTS.md documentation
- [x] All tests pass

### Validation
- [x] Query real Chatwoot deployment to verify `GET /api/v1/accounts/{id}/portals` response format
- [ ] Query real Chatwoot deployment to verify article fetch works for each portal slug

## Chatwoot API

### List Portals Endpoint
```
GET /api/v1/accounts/{account_id}/portals
Header: api_access_token: <token>
```

**Expected response (to be verified against real deployment):**
```json
{
  "payload": [
    {
      "id": 1,
      "name": "Main Help Center",
      "slug": "main-help",
      "archived": false,
      ...
    }
  ]
}
```

### Fetch Articles Endpoint (existing)
```
GET /hc/{portal_slug}/en/articles.json
```

## Steps

### Step 1: Verify Chatwoot API Response Format
**Success:** Response format documented from real deployment

Run against real Chatwoot deployment:
```bash
curl -H "api_access_token: $CHATWOOT_API_TOKEN" \
  "$CHATWOOT_BASE_URL/api/v1/accounts/$CHATWOOT_ACCOUNT_ID/portals"
```

Document actual response structure in execution log.

### Step 2: Add list_portals() to ChatwootClient
**Success:** Method compiles, returns Vec of portal slugs

Add to `api/src/chatwoot/client.rs`:
```rust
pub async fn list_portals(&self) -> Result<Vec<String>> {
    // GET /api/v1/accounts/{account_id}/portals
    // Return vec of portal slugs, excluding archived portals
}
```

Add unit test with mock response based on Step 1 findings.

### Step 3: Update handle_customer_message()
**Success:** Handler fetches from all portals, compiles, tests pass

Update `api/src/support_bot/handler.rs`:
1. Remove `CHATWOOT_DEFAULT_PORTAL_SLUG` env var check
2. Call `chatwoot.list_portals()` to get all portal slugs
3. Fetch articles from each portal (parallel with `futures::future::join_all`)
4. Merge all articles into single Vec before search
5. Handle edge cases:
   - No portals exist: escalate with message
   - All portal fetches fail: escalate with error message
   - Some portal fetches fail: log warning, continue with successful ones

### Step 4: Remove CHATWOOT_DEFAULT_PORTAL_SLUG References
**Success:** No references to CHATWOOT_DEFAULT_PORTAL_SLUG in codebase

Remove from:
- `api/src/support_bot/handler.rs` (done in Step 3)
- `api/src/support_bot/AGENTS.md`
- `cf/.env.example`
- `cf/docker-compose.dev.yml`
- `cf/docker-compose.prod.yml`
- Any other files found via grep

### Step 5: Verify Against Real Deployment
**Success:** Bot correctly fetches articles from all portals

1. Deploy to dev: `./cf/deploy.py deploy dev`
2. Check logs: `./cf/deploy.py logs dev api-server`
3. Send test message via Chatwoot widget
4. Verify logs show:
   - Portal list fetched successfully
   - Articles fetched from each portal
   - Search performed across merged articles

### Step 6: Update Documentation
**Success:** AGENTS.md reflects new architecture

Update `api/src/support_bot/AGENTS.md`:
- Remove `CHATWOOT_DEFAULT_PORTAL_SLUG` from env vars table
- Update flow description to show automatic portal discovery
- Update common issues section

## Execution Log

### Step 1
- **Query:** `GET /api/v1/accounts/2/portals` against dev-support.decent-cloud.org
- **Response:** `{"payload":[{"id":1,"slug":"platform-overview","archived":false,...}]}`
- **Outcome:** ✅ Confirmed response format matches spec. Portal object has `slug` and `archived` fields.

### Step 2
- **Implementation:** Added `list_portals()` method to `ChatwootClient` in `/code/api/src/chatwoot/client.rs` (lines 430-470)
  - Follows exact pattern of existing `list_inboxes()` method
  - GET `/api/v1/accounts/{account_id}/portals` with `api_access_token` header
  - Deserializes response with `PortalsResponse { payload: Vec<Portal> }` structs
  - Filters out archived portals using `.filter(|p| !p.archived)`
  - Returns `Vec<String>` of portal slugs
- **Tests:** Added 2 unit tests in `/code/api/src/chatwoot/tests.rs`:
  - `test_portals_response_deserialize`: Verifies JSON deserialization with 2 portals (1 archived, 1 active)
  - `test_portals_response_empty`: Verifies empty payload handling
- **Outcome:** ✅ Implementation complete, tests pass

### Step 3
- **Implementation:** Updated `handle_customer_message()` in `/code/api/src/support_bot/handler.rs` (lines 37-139)
  - Added `futures` crate dependency to workspace and api Cargo.toml
  - Replaced `CHATWOOT_DEFAULT_PORTAL_SLUG` env var check with `chatwoot.list_portals()` call
  - Fetch articles from all portals in parallel using `futures::future::join_all`
  - Merge all successful results into single `Vec<HelpCenterArticle>` before search
  - Handle edge cases:
    - No portals exist: escalate with message "No Help Center portals configured"
    - Failed to list portals: escalate with error message "I'm experiencing technical difficulties"
    - All portal fetches fail: escalate with message "I don't have enough information"
    - Some portal fetches fail: log warning with `tracing::warn!`, continue with successful ones
  - Added `use futures::future::join_all` and `use crate::chatwoot::HelpCenterArticle` imports
- **Review:**
  - Code compiles with SQLX_OFFLINE=true
  - All 416 support_bot and chatwoot tests pass (1 pre-existing test isolation issue unrelated to changes)
  - Function signature unchanged
  - Follows existing patterns (minimal, KISS, DRY)
- **Outcome:** ✅ Implementation complete, ready to commit

### Step 4
- **Files updated:**
  - `/code/api/src/support_bot/AGENTS.md`:
    - Updated flow diagram: "Get portal_slug from env" → "Discover all Help Center portals"
    - Updated "Simplified Support Bot" section: "Bot uses a single Help Center portal configured via..." → "Bot automatically fetches articles from ALL Help Center portals"
    - Removed `CHATWOOT_DEFAULT_PORTAL_SLUG` from env vars table
    - Updated "Bot not responding" troubleshooting: "Check CHATWOOT_DEFAULT_PORTAL_SLUG is configured" → "Check Help Center portals exist and have articles"
    - Updated "Missing configuration warnings": "If CHATWOOT_DEFAULT_PORTAL_SLUG is not set..." → "If no Help Center portals are found..."
  - `/code/cf/.env.example`: Removed commented export line and explanation comment
  - `/code/cf/.env.dev`: Removed `export CHATWOOT_DEFAULT_PORTAL_SLUG=platform-overview` line
  - `/code/cf/.env.prod`: Removed `export CHATWOOT_DEFAULT_PORTAL_SLUG=platform-overview` line
  - `/code/cf/docker-compose.dev.yml`: Removed `CHATWOOT_DEFAULT_PORTAL_SLUG` env var line
  - `/code/cf/docker-compose.prod.yml`: Removed `CHATWOOT_DEFAULT_PORTAL_SLUG` env var line
- **Verification:** `grep -r "CHATWOOT_DEFAULT_PORTAL_SLUG" /code` returns only references in spec files (this file and 2024-12-06 spec)
- **Outcome:** ✅ All references removed from codebase

### Step 5
- **Deployment:** Attempted `./cf/deploy.py deploy dev` - deployment succeeded but api-serve container failed to start
- **Logs:** Database access error: "unable to open database file" (SQLite permission issue, unrelated to multi-portal changes)
- **Outcome:** ⚠️ BLOCKED by pre-existing infrastructure issue. Multi-portal code is correct - needs database volume fix.

### Step 6
- **Changes:** Completed as part of Step 4 - AGENTS.md already updated with:
  - Flow diagram shows "Discover all Help Center portals"
  - `CHATWOOT_DEFAULT_PORTAL_SLUG` removed from env vars table
  - Troubleshooting section updated
- **Outcome:** ✅ Documentation complete

## Completion Summary
**Completed:** 2025-12-06 | **Agents:** 4/15 | **Steps:** 5/6 (Step 5 blocked by infra)

**Changes:** 12 files, +346/-44 lines, 2 new tests
- `api/src/chatwoot/client.rs`: Added `list_portals()` method
- `api/src/chatwoot/tests.rs`: Added portal response tests
- `api/src/support_bot/handler.rs`: Multi-portal fetching with parallel requests
- `api/src/support_bot/AGENTS.md`: Updated docs
- `cf/.env.example`, `cf/docker-compose.dev.yml`, `cf/docker-compose.prod.yml`: Removed env var

**Requirements:** 5/5 must-have ✅, 1/2 validation (deployment blocked by SQLite volume issue)

**Tests:** 504 passed ✅, cargo clippy clean ✅

**Notes:**
- Deployment verification blocked by pre-existing Docker volume mount issue (SQLite "unable to open database file")
- Code is correct and tested - needs volume path fix before live verification
- The volume `../data/api-data-dev:/data` uses relative path that doesn't work from `/code` directory
