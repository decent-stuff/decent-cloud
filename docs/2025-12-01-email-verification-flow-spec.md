# Email Verification Flow
**Status:** In Progress

## Requirements

### Must-have
- [ ] OAuth users (Google) have email auto-verified (Google already verified ownership)
- [ ] `email_verified` field exposed in AccountWithKeys API response
- [ ] Frontend stores `emailVerified` and `email` in auth state
- [ ] Prominent banner in dashboard for unverified email users
- [ ] Resend verification email endpoint with 1-minute rate limit
- [ ] Resend button in verification banner
- [ ] Success page thanks user and mentions reputation improvement
- [ ] Unit tests for all backend changes
- [ ] E2E test for verification flow

### Nice-to-have
- [ ] Badge/indicator in sidebar showing verification status

## Steps

### Step 1: Backend - OAuth Auto-Verification
**Success:** OAuth accounts created/linked have `email_verified=1`. Unit tests pass.
**Status:** COMPLETE

Files:
- `api/src/oauth_simple.rs` - Set email_verified=1 in oauth_register
- `api/src/database/accounts.rs` - Update create_oauth_linked_account to set email_verified=1
- Add unit tests

### Step 2: Backend - Expose email_verified in API
**Success:** AccountWithKeys includes `emailVerified` and `email` fields. Unit tests pass.
**Status:** Pending

Files:
- `api/src/database/accounts.rs` - Add fields to AccountWithKeys struct
- Update get_account_with_keys and get_account_with_keys_by_public_key

### Step 3: Backend - Resend Verification Endpoint
**Success:** POST /api/v1/accounts/resend-verification works with 1-min rate limit. Unit tests pass.
**Status:** Pending

Files:
- `api/src/openapi/accounts.rs` - Add resend_verification_email endpoint
- `api/src/database/accounts.rs` - Add get_latest_verification_token_time function

### Step 4: Frontend - Auth Store Updates
**Success:** AccountInfo interface includes emailVerified and email. TypeScript compiles.
**Status:** Pending

Files:
- `website/src/lib/stores/auth.ts` - Add emailVerified, email to AccountInfo
- `website/src/lib/services/account-api.ts` - Add fields to AccountWithKeys interface
- `website/src/lib/services/account-api.ts` - Add resendVerificationEmail function

### Step 5: Frontend - Email Verification Banner
**Success:** Banner shows for unverified users with resend button. Visual is prominent.
**Status:** Pending

Files:
- `website/src/lib/components/EmailVerificationBanner.svelte` - New component
- `website/src/routes/dashboard/+layout.svelte` - Integrate banner

### Step 6: Frontend - Improved Verification Success Page
**Success:** Success page thanks user and mentions reputation improvement.
**Status:** Pending

Files:
- `website/src/routes/verify-email/+page.svelte` - Update success message

### Step 7: E2E Verification
**Success:** Manual/E2E test confirms full flow works.
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:**
  - Modified `create_oauth_linked_account` in `api/src/database/accounts.rs` to set `email_verified=1` in INSERT statement (line 705)
  - Added `set_email_verified` call in `oauth_simple.rs` when linking OAuth to existing account (line 269-276)
  - Added 2 unit tests: `test_oauth_account_creation_sets_email_verified` and `test_oauth_linking_to_existing_account_sets_email_verified`
- **Review:** Code follows DRY principle by reusing existing `set_email_verified` function. Minimal changes applied.
- **Verification:** All OAuth tests pass (6 tests total). New tests verify email_verified=1 for both new OAuth accounts and linked accounts.
- **Outcome:** SUCCESS - OAuth accounts now have email auto-verified. All tests pass.

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

### Step 7
- **Implementation:** (pending)
- **Review:** (pending)
- **Verification:** (pending)
- **Outcome:** (pending)

## Completion Summary
(To be filled in Phase 4)
