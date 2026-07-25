# Fresh UX Audit — 2026-07-25 (no-mock, real browser)

**Auditor:** Planner agent (read-only). **Stack:** warm — web `localhost:59010`, api `localhost:59011`.
**Method:** drove the REAL app with `scripts/browser.js` (anonymous paths) + a Playwright probe using the
canonical `testAccount` fixture auth pattern (`addInitScript` seed + `first_login_onboarding_completed`,
real seed-phrase auth, zero mocks) for all authed routes. Walked every route group: landing, marketplace,
validators, agents(+pricing), reputation, login/recover/verify-email, dashboard overview, rentals,
offerings (list/create/edit), all `provider/*`, all `account/*`, admin, transfers, invoices, cloud/*,
checkout success/cancel. Mobile viewport + keyboard affordances (`?`, `Ctrl+K`, `/`) verified.

## Summary

| Severity | Count |
|----------|-------|
| 🔴 Critical | 0 |
| 🟠 High | 2 |
| 🟡 Medium | 3 |
| **Total net-new** | **5** |

All five share a common root cause for four of them: **the 2026-07-24 ICPay retirement cleanup was
incomplete** — `RentalRequestDialog` and the admin payout subsystem were made Stripe-only, but numerous
user-facing surfaces still present **ICP** as the payment unit / revenue currency, directly contradicting
the shipped "Stripe is the sole rail" change. The fifth is an access-control UX gap on the offering edit
route.

No console/JS errors, no 4xx/5xx on legit routes, no dead links, no perpetual spinners, no AI slop / lorem
text, and no broken layouts were found. Keyboard affordances and mobile layout are healthy (see Positive
Signals). Items already shipped / triaged as non-bug / deferred in `docs/OPEN_ISSUES.md` were deliberately
NOT re-reported (L3 hero-vs-stats, H6 offline provider, #441 subscription CTA mismatch, #436 seed-phrase
discoverability, the dashboard-stats-to-renters design judgment, etc.).

---

## Findings

```
[severity 🟠] /dashboard/rentals  and  /dashboard/invoices
repro: authed — `node scripts/browser.js snap http://localhost:59010/dashboard/rentals --seed "<12-word-seed>"`
       (seed LAST); same for /dashboard/invoices. Empty-state / "How billing works" copy is SSR'd.
defect: Stale copy explicitly tells users they can pay with ICP, which is FALSE since the 2026-07-24 ICPay
        retirement (Stripe is the sole rail; RentalRequestDialog was already made Stripe-only).
          - /dashboard/rentals empty state:  "2. Rent & Pay — Pay with ICP or card"
          - /dashboard/invoices "How billing works" step 1: "You pay upfront when renting (ICP or card)"
          - /dashboard/invoices footer note: "ICP tokens and credit/debit cards via Stripe."
        This is shown to every new user on their first rentals page visit and on the invoices explainer —
        a factual falsehood on the core rent/billing flow.
file:line: website/src/routes/dashboard/rentals/+page.svelte:537 ;
          website/src/routes/dashboard/invoices/+page.svelte:146 and :175
confidence: 9/10
safe: 9/10  (copy-only change; the actual payment path already rejects ICP)
```

```
[severity 🟠] /dashboard/provider/earnings
repro: authed — probe snap of /dashboard/provider/earnings (any account; the header renders before the
       "Provider Setup Required" gate hides the metrics copy).
defect: The entire provider earnings surface is denominated in ICP, but providers are now paid in fiat via
        Stripe. A provider reading "Gross Revenue 0.00 ICP", table columns "Gross (ICP) / Net (ICP) /
        Revenue (ICP)", balance "X ICP", and a CSV export column `amount_icp` will reasonably believe they
        are paid in a cryptocurrency they can no longer use. Money display contradicts the retired rail.
file:line: website/src/routes/dashboard/provider/earnings/+page.svelte:237,243,249,259,309,311,335,341,424
          (and CSV header :89 `amount_icp`)
confidence: 8/10
safe: 7/10  (needs a decision on the display currency; the `*_e9s` DB columns are denomination-agnostic so
       relabeling to USD/CHF + adjusting the divisor is the likely fix, but verify against real Stripe data)
```

```
[severity 🟡] /dashboard/marketplace/[id]  (offering detail)  and  /dashboard/marketplace (filter chips)
repro: `node scripts/browser.js snap http://localhost:59010/dashboard/marketplace/5` (anonymous).
defect: The core purchase page shows price as "2.50 ICP / month" and estimate "2.5000 ICP" with NO USD
        equivalent. Code path: it only appends "≈ $X/mo" when `icpPriceUsd` is non-null
        (offering-detail +page.svelte:228-232, 612-616); when the ICP price feed returns null the page
        falls through to bare "ICP" pricing. Since users actually pay in USD via Stripe (RentalRequestDialog
        is Stripe-only), the displayed currency ≠ payable currency — misleading at the exact decision point.
        Also: marketplace price filter chips render as "Min price: X ICP" / "Max price: X ICP"
        (marketplace +page.svelte:742,745). Root data: offerings seeded/created with `currency: 'ICP'`;
        post-retirement the UI still trusts that field verbatim and depends on a deprecated ICP price feed.
file:line: website/src/routes/dashboard/marketplace/[id]/+page.svelte:204-232,577,610-616 ;
          website/src/routes/dashboard/marketplace/+page.svelte:633,742,745
confidence: 7/10  (manifests whenever icpPriceUsd is null / offering currency is ICP; verify production
              offerings are created with a fiat currency — if so this is dev-seed-only, lower priority)
safe: 7/10
```

```
[severity 🟡] /dashboard/provider/analytics , /dashboard (overview) , DashboardSection.svelte
repro: authed probe snaps of /dashboard/provider/analytics and /dashboard; both render revenue/volume
       metric labels referencing ICP.
defect: Residual ICP-denomination labels on revenue/volume metrics, part of the same incomplete-retirement
        cluster:
          - /dashboard/provider/analytics table header: "Revenue 30d (ICP)"
          - /dashboard overview metric subtexts: "ICP lifetime" and "ICP active"
          - DashboardSection.svelte shared metric label: "Total Volume (ICP)"
file:line: website/src/routes/dashboard/provider/analytics/+page.svelte:249 ;
          website/src/routes/dashboard/+page.svelte:405,527 ;
          website/src/lib/components/DashboardSection.svelte:41
confidence: 8/10
safe: 8/10  (label-only change once the display-currency decision from the earnings finding is made)
```

```
[severity 🟡] /dashboard/offerings/[id]/edit
repro: authed as a fresh NON-provider account — visit http://localhost:59010/dashboard/offerings/5/edit .
defect: The edit page loads the offering via the PUBLIC `getOffering(offeringDbId)` endpoint (line 326) with
        NO client-side ownership / provider-status guard. A non-owner, non-provider account lands on a full
        "Edit Offering" form pre-filled with another provider's offering data (name, description, price,
        currency, post-provision script) and a live "Save" button. The Save path IS server-protected
        (provider-scoped PUT at :262-310 rejects non-owners), so this is not a data-mutation hole — but the
        UX is misleading (an edit form that will always fail) and it bypasses the "Provider Setup Required"
        gate that the offerings LIST page enforces. Should redirect non-owners to view-only / show a
        "not your offering" state, matching how /dashboard/offerings gates non-providers.
file:line: website/src/routes/dashboard/offerings/[id]/edit/+page.svelte:319-352  (onMount load; no guard)
confidence: 8/10
safe: 7/10  (add an ownership check: compare offering.pubkey to currentIdentity before rendering the form,
       redirect otherwise; the data is public so no leak risk)
```

---

## Positive signals (deliberately verified, NOT defects)

- **No console/JS errors** on any of the ~35 routes walked (anonymous + authed); no 4xx/5xx on legit API
  routes; no `requestfailed` beyond Vite dev-module aborts (harness noise from the `--seed` double-nav).
- **No dead links**: landing footer (Whitepaper PDF 200, GitHub 200, Discord/Twitter 301 normal redirects);
  all in-app nav links resolve; `/dashboard/providers` bare path 404s by design (nothing links to it).
- **No perpetual spinners**: every page rendered real content within 1.5–3 s.
- **No AI slop / lorem / stub text**: copy throughout is specific and real.
- **Keyboard affordances all work**: `?` opens the keyboard-help overlay; `Ctrl+K` opens the command palette
  ("Search or navigate..."); `/` focuses the marketplace search input (`#marketplace-search`).
- **Mobile layout healthy**: marketplace shows the mobile `<select>` sort affordance (#439 confirmed), all
  filter/action buttons present at 375×812.
- **Graceful empty / error states**: rental-not-found ("Contract Not Found"), provider-not-found, checkout
  success without `session_id` ("Something Went Wrong"), checkout cancel, verify-email without token, and
  `/dashboard/admin` for a non-admin ("Access Denied") all render clear, helpful copy.
- **Agents waitlist form** submits with clear inline feedback ("Thanks - you are on the beta list. We will
  reach out within a week.") — no silent submit.

## Notes on tooling (not product defects)

- `scripts/browser.js --seed` does NOT fully authenticate against this stack: it omits the
  `first_login_onboarding_completed` localStorage flag the canonical `testAccount` fixture sets, so the
  auth store never propagates and `snap`/`wait` capture GUEST state on authed pages (Logout never appears).
  This is a tooling limitation already half-documented in `scripts/AGENTS.md` (eval --seed) and
  `docs/OPEN_ISSUES.md`. For this audit, authed routes were driven with a Playwright probe using the exact
  `testAccount` fixture pattern (`addInitScript` seed + onboarding flag + wait-for-Logout), which is the
  source-of-truth auth method the 300-test e2e suite uses. **Recommend**: backport the
  `first_login_onboarding_completed='true'` set into `browser.js`'s `authenticatePage()` so `--seed` works
  standalone for future audits.
- `node scripts/dc-auth.js create-user` printed a success JSON + seed/pubkey but did NOT persist the account
  (its UI registration flow failed silently — output began with "[SNAP] (snapshot unavailable)"). The account
  had to be inserted DB-direct via the `seedAccountDirect` pattern. Worth a separate look if reproducible.
