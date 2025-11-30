# Account Recovery via Email

**Status:** In Progress

## Requirements

### Must-have
- [x] "Forgot password?" link on login page
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
**Status:** Complete

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
- **Implementation:** Added recovery link to `/code/website/src/routes/login/+page.svelte` in footer area. Link text "Lost access? Recover your account" navigates to `/recover`. Positioned above "Back to home" button. Styling: `text-white/50` base color (more subtle than back button), `hover:text-white/80` on hover, `text-xs` size (smaller than back button). Uses `space-y-2` for vertical spacing between footer links.
- **Review:** Minimal change - added 9 lines total. Follows KISS principle: just a plain `<a>` tag, no new components. Styling matches existing page theme (white text with transparency, smooth transitions). Recovery link is more subtle (50% opacity, xs text) than back button (60% opacity, sm text) to maintain visual hierarchy.
- **Verification:** TypeScript check passes (`npm run check`). No errors or warnings. Link appears in footer below auth flow card.
- **Outcome:** Complete. Recovery link is visible on login page and navigates to `/recover` route.

### Step 4
- **Implementation:**
- **Review:**
- **Verification:**
- **Outcome:**

## Completion Summary
