# E2E Harness Overhaul + UX Fix Plan (2026-07-22)

## Goal
Radically improve the Web UI e2e test harness (run in seconds, cover ALL user
flows) while fixing the functional/visual/UX issues found by live browser audit.

## Method
TDD: RED (write failing test) → GREEN (minimal fix) → REFACTOR → commit each unit.
No silent errors. DRY/KISS/YAGNI. Greenfield (no backward-compat baggage).

---

## Phase 1 — Critical functional fixes (blocks core product loop)

| ID | Issue | Root cause | Fix |
|----|-------|-----------|-----|
| C1 | Marketplace shows 0 offerings (10 exist) | `marketplace/+page.svelte:229-237` hides demo+offline by default; "Clear all filters" re-applies the same hide → dead-end empty state | Distinct empty-state when defaults hide everything; one-click reveal; don't re-hide on clear |
| C2 | Profile page crashes ("No account username found") | `account/profile/+page.svelte` identity race vs working `account/+page.svelte` | Align username derivation with overview page; guard UserProfileEditor |
| H1 | Billing Spending Alerts renders raw `not found` | endpoint `/spending-alert` missing (404); response body painted to DOM | Catch 404, hide section gracefully |
| H4 | Rentals 404 on `/contract-events` every load | endpoint missing; signed call fails silently | Handle 404 without console noise |
| H6 | All Rent buttons disabled (no online provider) | seed data has no online provider | Seed an online provider for dev/e2e so rent flow is exercisable |

## Phase 2 — High UX fixes

| ID | Issue | Fix |
|----|-------|-----|
| H2 | Transfers dead-end (no send/receive) | Add Send/Receive actions or clear "coming soon" messaging |
| H3 | Create offering step 2 Hetzner dead-end | Make "Connect a Hetzner account" a real link; add Skip path |
| H5 | Login has no discoverable registration path | Add "Create account" affordance routing to Generate New |

## Phase 3 — Medium UX

| ID | Issue | Fix |
|----|-------|-----|
| M1 | Dashboard shows misleading provider metrics to non-provider | Gate provider panels behind actual-provider check |
| M2 | Marketplace controls mislabeled ("Category:" holds regions) | Rename/split controls |
| M3 | Create offering placeholders-as-labels, unlabeled selects | Add real `<label>` elements |
| M4 | Email + seed banners on 19/22 pages | Dismissible, respect dismissal globally |
| M5 | Billing VAT country EU-only | Widen country list or clarify label |

## Phase 4 — E2E harness overhaul

### 4a. DRY refactors (do first — unblocks everything)
1. Extract `waitForAuthReady(page)` → `fixtures/test-account.ts`
2. Extract `deleteSavedOfferingsForUser()` → `seed-helpers.ts`
3. Extract parametrized `seedOffering()` → `seed-helpers.ts`
4. Reuse `signIn()` helper in signin-flow.spec.ts (5× inline dup)
5. Reuse `psqlArgs()`/`sql()` in test-admin-account.ts

### 4b. Speed wins
1. Drop `registerNewAccount` from chatwoot-api.spec.ts → seedAccountDirect
2. Purge networkidle from signin-flow (8) + recovery-flow (3)
3. Replace waitForTimeout in notification-bell (2000ms×2) + dashboard-overview (500ms)
4. Move API-only tests (provider-response-metrics, chatwoot-api) to integration project
5. Consolidate navigations: account-notifications (7→2), offerings-template (5→2)

### 4c. Dead test cleanup
1. Delete post-rental-welcome.spec.ts:21 (always-skips)
2. Rewrite :60/:79 against DB-seeded real contract id
3. Remove redundant admin-dashboard.spec.ts:44

### 4d. Coverage gap fills (key flows)
1. **Become-provider** (`/dashboard/offerings/create`) — full form flow
2. **Edit offering** (`/dashboard/offerings/[id]/edit`)
3. **Reputation detail + trust** (`/dashboard/reputation/[identifier]`, `/trust`)
4. **Provider authenticated requests** (accept/reject/batch)
5. **Rental creation end-to-end** (with online provider from Phase 1 H6)
6. **Email verification** (`/verify-email`)
7. **Landing page content** (`/` — hero, CTAs)
8. **Profile edit + persist** (`/dashboard/account/profile` — after C2 fix)
9. **Agents pricing** (`/agents/pricing`)
10. **Offline page** (`/offline`)

## Phase 5 — Documentation
- Update `repo/docs/OPEN_ISSUES.md` with new findings + resolutions
- Update `repo/AGENTS.md` with harness conventions
- Persist any remaining open items

## Execution order
Phase 1 (critical) → Phase 4a (DRY, unblocks test work) → Phase 4b-4c (speed/dead) →
Phase 4d (coverage) → Phase 2-3 (UX) → Phase 5 (docs).

Each unit: test (RED) → fix (GREEN) → commit.
