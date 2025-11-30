# Account Recovery via Email

**Status:** In Progress

## Requirements

### Must-have
- [ ] "Forgot password?" link on login page
- [ ] Recovery request form (enter email)
- [ ] `/recover` page to handle recovery token from email link
- [ ] Recovery completion flow (generate new seed phrase, add key)
- [ ] API client functions for recovery endpoints
- [ ] E2E test for recovery flow

### Nice-to-have
- [ ] Rate limiting feedback on excessive requests

## Steps

### Step 1: Add API client functions for recovery
**Success:** `requestRecovery(email)` and `completeRecovery(token, publicKeyHex)` functions work
**Status:** Complete

### Step 2: Create /recover page with full flow
**Success:** Page handles both request and completion flows, reuses SeedPhraseStep
**Status:** Complete

### Step 3: Add "Forgot password?" link to login page
**Success:** Link navigates to /recover, visible on login page
**Status:** Pending

### Step 4: E2E test for recovery flow
**Success:** Playwright test covers request → complete → login
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:** Added `requestRecovery(email)` and `completeRecovery(token, publicKeyHex)` functions to `/code/website/src/lib/services/account-api.ts`. Both functions follow existing patterns: POST to backend endpoints, handle ApiResponse wrapper, provide detailed error messages with HTTP status codes.
- **Review:** Functions are minimal and follow DRY - reuse error handling pattern from other functions in the file. Backend expects camelCase JSON (email, token, publicKey).
- **Verification:** TypeScript check passes (`npm run check`). No new dependencies required.
- **Outcome:** Complete. Two clean functions ready for use in recovery UI.

### Step 2
- **Implementation:** Created `/code/website/src/routes/recover/+page.svelte` with state machine handling both recovery flows. States: 'request' (email form), 'request-sent' (confirmation), 'generate-seed' (SeedPhraseStep in generate mode), 'processing', 'success' (redirect to login). Reuses SeedPhraseStep component with `initialMode="generate"` and `showModeChoice={false}`. Token detection via URL param `?token=xxx` triggers generate-seed state. Matches login page styling (dark gradient, card layout).
- **Review:** Page is minimal (224 lines total). Reuses existing components (SeedPhraseStep) and utilities (identityFromSeed, bytesToHex, requestRecovery, completeRecovery). No code duplication. State transitions are clear and follow KISS principle. Error handling provides specific messages.
- **Verification:** TypeScript check passes (`npm run check`). No new dependencies. File structure follows SvelteKit conventions. Renamed `state` variable to `currentState` to avoid TypeScript inference issues with Svelte 5 runes.
- **Outcome:** Complete. Page handles full recovery flow with clean UI matching existing design patterns.

### Step 3
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

### Step 4
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

## Completion Summary
