# 2026-08-09 — User balance/financial visibility

## STATUS: DONE (impl + targeted e2e verified)

## Context
User: "Ensure user balances are visible in the UI, and address any other outstanding issues."

## Problem analysis
decent-cloud has **NO prepaid-wallet / credit-balance concept** (ICP retired; Stripe
pay-per-contract is the sole payment rail). "User balances" therefore means the financial
summary a user needs: **SPENDING** (renter) and **EARNINGS** (provider).

### Current financial surfaces
| Surface | Route | Visible to | Status |
|---------|-------|-----------|--------|
| Total Spent + Spending Insights | `/dashboard` | tenant w/ ≥1 rental | OK |
| Earnings metric | `/dashboard` | provider w/ ≥1 offering | OK |
| Provider Earnings detail | `/dashboard/provider/earnings` | provider | OK |
| Spending Alerts config | `/dashboard/account/billing` | authenticated | OK |
| Rentals spending stats | `/dashboard/rentals` | tenant | OK |

### THE GAP
`detectUserRole` returns `'new'` for users with zero contracts AND zero offerings.
The dashboard's `{#if userRole === 'new'}` branch (`+page.svelte:363`) renders **only CTA
cards — ZERO financial information**. A new user cannot see that spending/earnings tracking
exists, nor their $0 starting position. This is the "balances not visible" gap.

Secondary: `spendingInsights` returns `null` when `rentals.length === 0` (`+page.svelte:63`),
hiding the Spending Insights card even for tenants whose only rentals are historical.

### Stale ICP leftovers (tech debt)
- `invoices/+page.svelte:168` — "Contracts are prepaid" (FALSE since Stripe; contracts are
  pay-per-contract, not prepaid).
- `website/src/lib/utils/metadata.js` — dead `InsufficientFunds` + `icrc1_balance_of` ICRC types.

## Plan

### 1. Financial summary always visible (primary fix)
- Extract `lifetimeSpentUsd` + `activeEarningsUsd` as `$derived` values (DRY — currently
  inlined + duplicated in tenant/provider branches).
- Add a compact 2-card financial summary to the `new`-user branch showing Spending $0.00 +
  Earnings $0.00 with links to detail pages. Sets expectations + makes the surface discoverable.

### 2. Clean stale ICP references
- Fix invoices "prepaid" comment → accurate pay-per-contract wording.
- Audit `metadata.js` ICRC leftovers; remove if truly dead (verify no consumers first).

### 3. E2E test
- Add test to `dashboard-overview.spec.ts` asserting the financial summary renders for the
  fresh (new-user) fixture account with $0.00 values.

### 4. Verify
- `cd website && npm run check` (svelte-check)
- `cd website && npm run test:e2e:fast -- dashboard-overview provider-earnings billing`
- clippy/nextest if any Rust touched (none expected — UI-only).

## Results
- svelte-check: 0 errors, 0 warnings.
- dashboard-overview (4) + search-dsl (8) + marketplace-empty-state (1): 13/13 passed (28.7s).
- Financial specs (provider-earnings, billing-settings, rentals, dashboard-role-gating): 19 passed.
- New e2e: `financial summary is visible for new users with $0.00 balances` passes.
- Bonus fix: `search-dsl.spec.ts` was leaking `e2edsl-*` offerings into
  `marketplace-empty-state` (afterAll used silent `.catch(()=>{})`). Added
  beforeAll stale-data cleanup + surfaced cleanup errors. Root cause of the
  only pre-existing suite failure.
- Dead ICP code (`website/src/lib/services/icp.ts`, `metadata.ts` ICRC types):
  DEFERRED — needs careful audit (ICP canister may still be used for identity).
  Filed for follow-up, not touched here.

## Baseline
- `main` @ `9ded9f85` (working tree clean; branch diverged 2 local / 3 remote — rebase later).
- Warm stack up: api http://localhost:59011, web http://localhost:59010.
