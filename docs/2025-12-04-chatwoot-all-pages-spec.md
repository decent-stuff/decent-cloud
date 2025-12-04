# Chatwoot Widget on All Pages

**Status:** Complete

## Requirements

### Must-have
- [ ] Add VITE_CHATWOOT_WEBSITE_TOKEN and VITE_CHATWOOT_BASE_URL to deploy.py website build
- [ ] Move ChatwootWidget from dashboard layout to root layout (all pages)

### Nice-to-have
- None

## Steps

### Step 1: Add VITE_CHATWOOT_* to deploy.py
**Success:** deploy.py writes VITE_CHATWOOT_WEBSITE_TOKEN and VITE_CHATWOOT_BASE_URL to .env.local
**Status:** Pending

### Step 2: Move ChatwootWidget to root layout
**Success:** ChatwootWidget renders on all pages, removed from dashboard layout
**Status:** Pending

## Execution Log

### Step 1
- **Implementation:** Added CHATWOOT_WEBSITE_TOKEN and CHATWOOT_BASE_URL handling to deploy.py, updated cf/.env.example
- **Review:** Changes follow existing pattern for Stripe keys
- **Outcome:** Success

### Step 2
- **Implementation:** Moved ChatwootWidget from dashboard layout to root layout, fixed unused import in chatwoot-api.spec.ts
- **Review:** Clean removal, no duplication
- **Outcome:** Success

## Completion Summary
**Completed:** 2025-12-04 | **Agents:** 1/15 | **Steps:** 2/2
Changes: 5 files, +28/-11 lines, 0 new tests (existing tests cover functionality)
Requirements: 2/2 must-have, 0/0 nice-to-have
Tests pass ✓, cargo make clean ✓
Notes: Also fixed pre-existing unused import in chatwoot-api.spec.ts
