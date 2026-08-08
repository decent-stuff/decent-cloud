# E2E Test Flow Catalog

This file is the **single source of truth** mapping every user-facing flow in
Decent Cloud to its Playwright E2E coverage. Keep it in sync with the specs
under `tests/e2e/` — when you add a flow or a test, update this file (see
[Keeping this file current](#keeping-this-file-current)).

## How to use this file

- Each row maps one user **flow** to the spec + test name that covers it.
- **Status**
  - ✅ covered — a dedicated test asserts this flow end-to-end.
  - ⚠️ partial — only an entry-point / render / error branch is asserted; the
    full happy path is not exercised (often because it needs external services,
    complex seeding, or a provider/admin account the fixture can't provide).
  - ❌ gap — no test covers this flow.
- **Tags** column lists the flow's tags. `@smoke` marks a critical-path test
  included in the fast dev-loop tier (`npm run test:e2e:fast:smoke`, <35s).
- **Spec / Test** — the file under `tests/e2e/` and the `test(...)` title. A
  flow may map to several tests; only the most representative is listed (search
  the spec for the full set).
- Spec paths are relative to `tests/e2e/` (e.g. `signin-flow.spec.ts`).

### Tag legend

| Tag | Meaning |
|-----|---------|
| `@smoke` | Critical path; runs in the fast smoke tier (`test:e2e:fast:smoke`, <35s, ~26 tests). Pick only fast (<5s), reliable, low-seed tests. |
| `@auth` | Authentication: register, sign-in, sign-out, recover, verify, redirect. |
| `@marketplace` | Public browse: marketplace, search/filter/sort, offering detail, pricing, reputation, compare. |
| `@rental` | Tenant rental lifecycle: rent, pay, view, cancel, rentals list/detail. |
| `@provider` | Provider dashboard: become provider, create/edit offering, status/stock/visibility, requests, earnings, agent pools, SLA. |
| `@account` | Account: profile, devices/security, notifications, saved offerings, cloud accounts. |
| `@billing` | Billing: invoices, transfers, billing settings, payment flows. |
| `@admin` | Admin dashboard + access control. |

### Running a category / tag

```bash
cd website
npm run test:e2e:fast:smoke                 # ~32 critical-path tests, <35s (dev loop)
npm run test:e2e:fast                       # full suite (all specs)
```

> **Tags are doc-only except `@smoke`.** The category labels in this file
> (`@auth`, `@marketplace`, `@rental`, `@provider`, …) are documentation-only
> row markers — they are **not** present in test titles, so
> `--grep @rental` (or `@provider`/`@marketplace`/…) matches **zero** tests
> (verified). The only tag matched at runtime is **`@smoke`**, which lives in
> test titles (e.g. `test('@smoke ...')`).
>
> Run a **category by spec-file pattern** instead — specs are grouped one flow
> per file (or a small cluster), so a filename glob is the category selector:
>
> ```bash
> npm run test:e2e:fast -- rentals.spec.ts        # one spec file
> npm run test:e2e:fast -- "*rental*"             # every rental-related spec (filename glob)
> npm run test:e2e:fast -- "provider-*"           # every provider-dashboard spec
> npm run test:e2e:fast -- -g "should sign in"    # one test by title substring
> ```
>
> Adding per-test category tags (`@rental`, `@provider`, …) to titles so the
> categories become grep-able is a **documented future enhancement** — not done
> today (it would mean editing every spec title for a grep convenience, against
> KISS/YAGNI).

---

## Flow Index

Status legend: ✅ covered · ⚠️ partial · ❌ gap

### 1. Public (anonymous)

| Flow | Status | Tags | Spec | Test |
|------|--------|------|------|------|
| Landing page (`/`) renders | ✅ | `@smoke` `@marketplace` | `anonymous-browsing.spec.ts` | `@smoke landing page (/) renders title, hero, and marketplace stats` |
| Marketplace browse renders | ✅ | `@smoke` `@marketplace` | `anonymous-browsing.spec.ts` | `@smoke should allow anonymous user to view marketplace` |
| Marketplace search (DSL) | ✅ | `@marketplace` | `search-dsl.spec.ts` | `should filter offerings by GPU type checkbox` (+6 more) |
| Marketplace sort | ✅ | `@marketplace` | `marketplace-sort.spec.ts` | `desktop keeps the pill UI and exposes the <select> as an a11y alternative (#439)` |
| Marketplace default-hide / empty state | ✅ | `@marketplace` | `marketplace-empty-state.spec.ts` · `anonymous-browsing.spec.ts` | `offers a reveal action when all offerings are hidden by default` · `should hide demo offerings by default on marketplace` |
| Offering detail renders | ✅ | `@marketplace` | `rentable-offering-fixture.spec.ts` · `offline-provider-warning.spec.ts` | `seeded self_provisioned offering shows an enabled Rent Resource button` |
| Offering detail SLA card | ✅ | `@marketplace` | `offering-sla-empty-state.spec.ts` | `shows friendly empty state instead of empty gray bars...` |
| Offline-provider warning | ✅ | `@marketplace` | `offline-provider-warning.spec.ts` | `should disable Rent button and explain why when provider is offline` |
| Pricing (agents) | ✅ | `@marketplace` | `agents-pricing.spec.ts` · `agents.spec.ts` | `renders the single pricing tier with a price point and CTAs` |
| Reputation search | ✅ | `@marketplace` | `reputation.spec.ts` | `renders the Reputation heading and search box` |
| Reputation detail | ✅ | `@marketplace` | `reputation-detail.spec.ts` | `renders the reputation profile for a known account` |
| Reputation trust report | ✅ | `@marketplace` | `reputation-trust.spec.ts` | `resolves a known account and renders the TrustDashboard with its trust score` · `renders the "Account Not Found" error for an unknown identifier` — covers the `/dashboard/reputation/[identifier]/trust` sub-route (Trust Report): known-account TrustDashboard render + not-found branch. (Added 2026-08-02: this route was an undocumented gap — absent from both FLOWS.md and route-audit's dynamic table. The "No Trust Data Available" empty state is out of scope: the trust-metrics API returns a zero-valued object for any pubkey, so the empty state only fires on API failure, which can't be triggered deterministically without a forbidden first-party mock.) |
| Public user profile | ✅ | `@marketplace` | `user.spec.ts` · `profile-page.spec.ts` | `redirects to the reputation page preserving the identifier` |
| Provider public page | ✅ | `@marketplace` | `providers.spec.ts` | `renders "Provider Not Found" card for an unknown identifier` |
| Compare offerings | ✅ | `@smoke` `@marketplace` | `compare-share.spec.ts` | `@smoke copies canonical comparison URL and shows success feedback` (share URL @smoke) · `renders the side-by-side comparison table for two seeded offerings` (full view — full suite only; seeds 2 offerings, violates smoke low-seed rule) |
| 404 / error page | ✅ | `@marketplace` | `error-page.spec.ts` | `404 renders branded error page with navigation, not blank screen` |

### 2. Auth

| Flow | Status | Tags | Spec | Test |
|------|--------|------|------|------|
| Register (full flow) | ✅ | `@smoke` `@auth` | `registration-flow.spec.ts` | `@smoke should complete full registration flow with seed phrase` |
| Sign in with valid credentials | ✅ | `@smoke` `@auth` | `signin-flow.spec.ts` | `@smoke should sign in successfully with valid credentials` |
| Sign in: reject invalid seed | ✅ | `@auth` | `signin-flow.spec.ts` | `should reject invalid seed phrase` |
| Sign out | ✅ | `@smoke` `@auth` | `signin-flow.spec.ts` | `@smoke should sign out successfully` |
| Session persists after refresh | ✅ | `@auth` | `signin-flow.spec.ts` | `should maintain session after page refresh` |
| Recover account | ✅ | `@auth` | `recovery-flow.spec.ts` | `should complete recovery flow with token and surface API error for a fake token` — covers the full Continue → onComplete → handleSeedComplete → completeRecovery wiring plus the API-error surfacing path; the recover page uses `bg-danger/10` for its error div (distinct from the SeedPhraseStep's `bg-red-500/20` component-local errors). The success path (real DB-seeded token → completeRecovery → auto-login → success state with auto-redirect countdown) is covered by `success: a valid DB-seeded token completes recovery, auto-logs-in, and shows the auto-redirect countdown (#445)` |
| Verify email | ✅ | `@auth` | `verify-email.spec.ts` | `success: a valid DB-seeded token verifies the email and shows the success state` — success + both error branches; the token is seeded DB-side (no external email service). The success state auto-redirects to `/dashboard` after a countdown with a manual "Go now" link (#445), covered by `success: auto-redirect countdown appears and manual "Go now" link lands on /dashboard (#445)` |
| Auth capability + login default (#436) | ✅ | `@smoke` `@auth` | `auth-capabilities.spec.ts` | `capability endpoint returns a well-formed boolean` · `login page default surface matches the server capability` — the public `GET /api/v1/auth/capabilities` endpoint drives the frontend default; server env is the single source of truth. The spec reads the real capability then asserts the login default matches it (OAuth-on OR OAuth-off), so it stays green on any stack config |
| Redirect / returnUrl | ✅ | `@auth` | `signin-flow.spec.ts` · `registration-flow.spec.ts` | `should redirect to returnUrl after successful sign-in` |
| Login ↔ register CTA | ✅ | `@auth` | `login-registration-cta.spec.ts` | `Create account link jumps directly to seed backup (generate mode)` |
| Seed-phrase education + loss warning (UX-003) | ✅ | `@smoke` `@auth` | `seed-phrase-education.spec.ts` | `@smoke login chooser shows inline seed-phrase education` · `@smoke seed backup step warns the seed cannot be recovered if lost` · `@smoke recovery link on the login page uses seed-phrase-specific copy` — inline "what is a seed phrase?" note on the auth chooser (renders in both OAuth-on and OAuth-off layouts), a prominent permanent-loss warning on the backup step, and a seed-phrase-specific recovery link. Stays green on any capability config (reads the real `/api/v1/auth/capabilities`, never mocked). |
| First-login onboarding modal | ✅ | `@smoke` `@auth` | `first-login-onboarding.spec.ts` | `@smoke guides a new user through all onboarding steps once` |

### 3. Tenant Dashboard

| Flow | Status | Tags | Spec | Test |
|------|--------|------|------|------|
| Dashboard overview loads | ✅ | `@smoke` `@account` | `dashboard-overview.spec.ts` | `@smoke dashboard loads all sections via the single combined /provider/dashboard call` |
| Dashboard banner stack | ✅ | `@account` | `dashboard-banners.spec.ts` | `seed-phrase + unverified-email user sees BOTH banners simultaneously` |
| Role-based content gating | ✅ | `@account` | `dashboard-role-gating.spec.ts` | `new user does not see provider trust metrics or red flags` |
| Sidebar navigation | ✅ | `@smoke` `@account` | `anonymous-browsing.spec.ts` | `@smoke should show sidebar for anonymous users with all navigation items` |
| Browse marketplace (authed) | ✅ | `@smoke` `@marketplace` | (see Public — marketplace browse) | — |
| Rent an offering (dialog → contract) | ⚠️ | `@rental` | `rent-flow.spec.ts` | `rent an offering → contract appears on the rentals list with a Cancel button` — covered in the **full suite** only; excluded from smoke (6s + complex seeding) |
| Email verification gate on rent (F3) | ✅ | `@rental` `@account` | `rent-email-verification-gate.spec.ts` | `offering detail shows "Verify email to rent" for an unverified user` — serial spec (shared testAccount pubkey, middle test flips email_verified DB-side): unverified detail label, dialog Submit locked + notice, then verified detail label. Covers the inline surfacing of the rental create prerequisite (offering-detail button relabel + redirect, rentals empty-state note, RentalRequestDialog Submit gate). |
| View rentals list | ✅ | `@smoke` `@rental` | `rentals.spec.ts` | `@smoke empty state: fresh user sees onboarding steps and marketplace CTAs` |
| Rentals: populated state / tabs / search | ✅ | `@rental` | `rentals.spec.ts` | `populated state: shows contract cards with status tabs and counts` |
| Cancel a rental | ✅ | `@smoke` `@rental` | `rentals.spec.ts` · `rent-flow.spec.ts` | `@smoke action: Cancel a requested contract moves it to Cancelled tab` |
| Rental detail deep link | ✅ | `@rental` | `rentals.spec.ts` · `rent-flow.spec.ts` | `deep link: detail page at /dashboard/rentals/[id] loads` |
| Post-rental welcome banner | ✅ | `@rental` | `post-rental-welcome.spec.ts` | `shows the welcome banner when arriving with ?welcome=true` |
| Payment flows (Stripe-only) | ✅ | `@billing` `@rental` | `payment-flows.spec.ts` | `checkout.session.completed webhook flips payment_status to succeeded (the money path)` — UI rendering + the backend money path (signed webhook → payment_status flip + Stripe id recording) via STRIPE_WEBHOOK_SECRET on the warm stack. The hosted Stripe Checkout redirect itself stays out-of-harness (cross-origin); the webhook test closes the backend half. ICPay rail retired 2026-07-24. |
| Checkout cancel/success pages | ✅ | `@billing` | `checkout.spec.ts` | `renders the cancelled-payment page without a contract_id` |
| Save / unsave offerings | ✅ | `@account` `@marketplace` | `offering-detail-save.spec.ts` · `saved-offerings.spec.ts` | `bookmark toggle on offering detail page saves in a single click` |
| Edit profile | ✅ | `@smoke` `@account` | `profile-page.spec.ts` · `account-profile-edit.spec.ts` | `@smoke profile edit persists after save and reload` |
| Manage devices / security | ✅ | `@smoke` `@account` | `account-add-device.spec.ts` · `account-page.spec.ts` | `@smoke links a generated device key and raises the device count from 1 to 2` |
| Account overview / settings nav | ✅ | `@account` | `account-page.spec.ts` | `account page: overview renders correctly via direct URL` |
| Account error recovery | ✅ | `@account` | `account.spec.ts` | `shows error card with Retry and Logout when account fetch fails (#6)` |
| Billing settings (address/VAT) | ✅ | `@billing` `@account` | `billing-settings.spec.ts` | `billing settings: save billing address` (+spending alerts) |
| Invoices | ✅ | `@billing` | `invoices.spec.ts` | `populated state: shows invoice table with one row per invoiceable contract` |
| Transfers | ✅ | `@billing` | `transfers.spec.ts` | `populated state: shows sent and received transfers with direction icons` |
| Notifications (bell + channels) | ✅ | `@account` | `notification-bell.spec.ts` · `account-notifications.spec.ts` | `badge displays the correct unread count from the DB` |
| Cloud accounts | ✅ | `@account` | `cloud.spec.ts` | `populated state: a DB-seeded cloud account renders in the list` + `disconnect: the modal delete flow removes the cloud account` — empty state, Add-Account modal, populated list render, AND the modal-based signed-DELETE disconnect. No real Hetzner/Proxmox connection (cloud_accounts is a plain DB row seeded under the testAccount). |
| Keyboard shortcut (focus search) | ✅ | `@smoke` `@marketplace` | `keyboard-shortcuts.spec.ts` | `@smoke / focuses marketplace search input` |

### 4. Provider Dashboard

| Flow | Status | Tags | Spec | Test |
|------|--------|------|------|------|
| Become provider / setup wizard | ✅ | `@smoke` `@provider` | `provider-onboarding-submit.spec.ts` · `become-provider.spec.ts` | `submitting the Help Center form persists onboarding data across reload` (full submit) · `?step=3 deep-link renders the Help Center form without clicking through steps 1+2` (deep-link — the wizard now honors a `?step=N` query param on mount; a valid N wins over the persisted localStorage step, an invalid/absent N falls back to it) · `@smoke renders step 1, advances to step 2, and links Hetzner onboarding` (wizard render) |
| Create offering (full submit) | ✅ | `@provider` | `offering-create.spec.ts` | `create succeeds when the body omits pubkey (path-derived)` (real signed POST) — was blocked by #440 (backend `Offering.pubkey` rejected missing field); fixed in `ebebff02` via `#[oai(default)]` (handler overwrites from URL path). `monthly price is pre-filled with cost × 1.15 when a Hetzner server is selected` + `provider can override the suggested monthly price and the override is what gets submitted` cover the #442 monthly-price auto-suggest (catalog endpoint mocked — it requires a real Hetzner token unavailable in tests; create submit is real). CSV template download covered in `offerings-template.spec.ts`. |
| Edit offering | ✅ | `@provider` | `offering-edit.spec.ts` | `submit persists the change and redirects to the offerings list` |
| Offering edit: ownership guard (#5) | ✅ | `@smoke` `@provider` | `offering-edit-ownership.spec.ts` | `@smoke blocks a non-owner from the editable form (no Save button)` — `/dashboard/offerings/[id]/edit` redirects non-owners to the view-only route |
| Offering status badge (a11y) | ✅ | `@provider` | `offering-status-badge.spec.ts` | `tooltip becomes visible when the badge button receives focus (#15)` |
| Manage visibility | ✅ | `@provider` | `offerings-status-menus.spec.ts` | `visibility menu lists all states with descriptions and persists selection` |
| Manage stock status | ✅ | `@provider` | `offerings-status-menus.spec.ts` | `stock menu lists all states with descriptions and persists selection` |
| View requests (non-provider gate) | ✅ | `@provider` | `provider-requests-auth.spec.ts` | `shows the provider-setup-required banner for a non-provider account` |
| Accept / reject a request | ✅ | `@provider` | `provider-accept-reject.spec.ts` | `accept a contract request removes it from pending` (+reject, render, auto-accept toggle) — authenticated provider seeds contracts where it is the provider, then accepts/rejects via signed POST .../respond |
| Auto-accept toggle | ✅ | `@provider` | `provider-accept-reject.spec.ts` | `auto-accept toggle can be enabled` — flips the provider_profiles.auto_accept_rentals toggle and asserts the enabled state + banner |
| Provider sub-pages render | ✅ | `@provider` | `provider-pages-smoke.spec.ts` | `/dashboard/provider/* renders heading ... and its empty state` (analytics, feedback, password-resets, reseller, sla, ssh-key-rotations) |
| Agent pools | ✅ | `@provider` | `agent-pool-create.spec.ts` · `agent-pool-edit.spec.ts` · `provider-pages-smoke.spec.ts` | `creates an agent pool and lists it in the pool table` (create via UI form) · `rename persists in DB, list table, and detail page header` (signed PUT rename + `/dashboard/provider/agents/[pool_id]` detail-page render — was an untested gap; the detail page has no inline rename UI, so the PUT is exercised directly and the new name is asserted in the table UI, the DB, and the detail `<h1>`/breadcrumb) |
| Earnings | ✅ | `@provider` | `provider-earnings.spec.ts` · `provider-pages-smoke.spec.ts` | `shows the summed revenue, contract count, and contract rows for seeded provider contracts` |
| SLA metrics | ✅ | `@smoke` `@provider` | `provider-response-metrics.spec.ts` · `offering-sla-empty-state.spec.ts` | `@smoke GET /providers/:pubkey/response-metrics returns contract request SLA metrics` |
| Notification settings | ✅ | `@provider` | `notification-settings.spec.ts` | `notification settings: section, channels, save button, tier limits, and usage grid render correctly` |
| Password resets (provider view) | ⚠️ | `@provider` | `provider-pages-smoke.spec.ts` | `/dashboard/provider/password-resets renders heading ... and its empty state` — empty-state render only |

### 5. Admin

| Flow | Status | Tags | Spec | Test |
|------|--------|------|------|------|
| Admin dashboard renders (admin) | ✅ | `@admin` | `admin-dashboard.spec.ts` | `should show admin features when user is admin` |
| Access control: anonymous denied | ✅ | `@admin` | `admin-dashboard.spec.ts` | `should show access denied for anonymous users` |
| Access control: non-admin denied | ✅ | `@admin` | `admin-dashboard.spec.ts` | `should show access denied for non-admin users` |
| Admin sidebar link gating | ✅ | `@admin` | `admin-dashboard.spec.ts` | `should show Admin link in sidebar for admin users` |
| Failed-email error visibility | ✅ | `@admin` | `admin-dashboard.spec.ts` | `failed-email error is fully visible, not truncated (#11)` |
| Admin actions (account mutations) | ✅ | `@admin` | `admin-account-mutations.spec.ts` | `setEmailVerified flips the target account email_verified flag` · `setAdminStatus grants and then revokes admin privileges` · `deleteAccount removes a non-admin target and a re-fetch reports it gone` — real signed mutations via the admin handlers. (Send Test Email still ❌ — needs MAILCHANNELS_API_KEY.) |
| Admin: refund approval gate | ✅ | `@admin` | `admin-refund-requests.spec.ts` | `admin API lists pending refund requests with correct fields` · `admin UI shows pending request and decline works end-to-end` · `status filter shows auto_issued without action buttons` — refund requests DB-seeded (cancel→gate covered by Rust integration tests with stripe_client=None); UI decline tested fully e2e; approve path needs Stripe test mode or stripe_client=None. |

### 6. UX regression guards

Fast pins for the post-ICP UX cleanup (commits 25945664..2dd8e373). Each test
fails if its fix is reverted. All live in one spec (`ux-regression-guards.spec.ts`)
so the set is trivial to list and extend; the spec uses `baseTest`
(`@playwright/test`) for the anonymous assertions and `authTest`
(`fixtures/test-account`) for the single authenticated one (UX-004), so the
anonymous pages are not silently authenticated.

| Flow | Status | Tags | Spec | Test |
|------|--------|------|------|------|
| No fabricated provider data on hero (UX-001) | ✅ | `@smoke` | `ux-regression-guards.spec.ts` | `@smoke UX-001 homepage hero shows the "Anatomy of a Trust Score" graphic, not fabricated provider data` — asserts the educational graphic IS present and the fake-card signatures (`provider_alpha`, `87 Trust Score`, `1,247 Contracts`) are gone. A substring check for "Verified Provider" is intentionally omitted: the hero's typing animation legitimately spells "Verified Provider Track Records", so a substring assertion would false-positive; the fabricated handle + numbers fully cover the regression. |
| Validators route retired (UX-002) | ✅ | `@smoke` | `ux-regression-guards.spec.ts` | `@smoke UX-002 /dashboard/validators is retired (404) and no sidebar link remains` — `/dashboard/validators` returns HTTP 404; the sidebar has no Validators link or label. |
| Honest marketplace stats (UX-005) | ✅ | `@smoke` | `ux-regression-guards.spec.ts` | `@smoke UX-005 homepage stats grid omits dead ICP metrics (Validators / Transfers)` — the "Marketplace Statistics" grid contains neither "Active Validators" nor "Total Transfers". |
| Dashboard welcome card uses @username (UX-004) | ✅ | `@smoke` | `ux-regression-guards.spec.ts` | `@smoke UX-004 dashboard welcome card shows @username, not a raw principal` — authenticated `.card-accent` shows `@<username>` and no dashed textual principal (`xxxxx-xxxxx-xxxxx`). |
| Unauth sidebar hides "My Activity" (UX-008) | ✅ | `@smoke` | `ux-regression-guards.spec.ts` | `@smoke UX-008 unauthenticated sidebar hides "My Activity" (single Sign In CTA)` — the auth-gated section is absent for anonymous users; a Sign In CTA remains. |
| Login heading "Sign In or Create Account" (UX-013) | ✅ | `@smoke` | `ux-regression-guards.spec.ts` | `@smoke UX-013 login page heading reads "Sign In or Create Account"` — the AuthFlow h2 advertises account creation, not sign-in only. |
| Focus-visible outline 2px (UX-010) | ❌ | — | — | **gap** — a computed-style assertion (`outline-width: 2px` on a focused focusable element) is feasible but fragile across theme/UA defaults; not yet added. |
| Hero typing respects prefers-reduced-motion (UX-012) | ❌ | — | — | **gap** — testable via a `reducedMotion: 'reduce'` context asserting the blinking `_` cursor span is absent; not yet added. |

### Cross-cutting

| Flow | Status | Tags | Spec | Test |
|------|--------|------|------|------|
| Route audit (every public + authed route) | ✅ | — | `route-audit.spec.ts` | parametrized per route — catches 4xx/5xx, console errors, missing headings |
| Chatwoot identity / support-access API | ✅ | — | `chatwoot-api.spec.ts` | `GET /chatwoot/identity returns identity hash for authenticated user` |

---

## Smoke tier (`@smoke`)

The fast dev-loop tier. Run with `npm run test:e2e:fast:smoke` (~32 tests,
**<35s** against the warm stack). Selection rules:

- **Critical path only** — landing/anonymous browse, dashboard overview, sign-in,
  verify-email, onboarding, provider create + SLA,
  keyboard shortcuts, auth modal, UX regression guards (§6).
  (Full registration, sign-out, profile edit,
  add-device, and rent/cancel actions are full-suite-only — too slow for the loop.)
- **Fast** — each test <5s. Exclude anything that drives a slow multi-step flow
  or needs `networkidle`.
- **Low seed** — exclude tests that need complex DB seeding (e.g. the real
  rent-via-dialog flow in `rent-flow.spec.ts`). Prefer empty-state / render /
  single-row-seed tests.
- **Reliable** — no flaky parallel-DB races; deterministic SSR waits.

Current smoke membership (run `npx playwright test --list --grep @smoke`):

| # | Flow | Spec:Test |
|---|------|-----------|
| 1 | Landing renders | `anonymous-browsing.spec.ts` › `@smoke landing page (/) renders...` |
| 2 | Marketplace browse | `anonymous-browsing.spec.ts` › `@smoke ...view marketplace` |
| 3 | Auth modal on protected action | `anonymous-browsing.spec.ts` › `@smoke ...auth modal...rent resource` |
| 4 | Sidebar navigation | `anonymous-browsing.spec.ts` › `@smoke ...sidebar...navigation items` |
| 5 | User profile redirect | `user.spec.ts` › `@smoke redirects to the reputation page...` |
| 6 | Provider SLA metrics (API) | `provider-response-metrics.spec.ts` › `@smoke ...response-metrics returns contract request SLA metrics` |
| 7 | Provider SLA metrics (invalid pubkey) | `provider-response-metrics.spec.ts` › `@smoke ...error for invalid pubkey...` |
| 8 | Provider create wizard (render) | `become-provider.spec.ts` › `@smoke renders step 1, advances to step 2...` |
| 9 | Command palette trigger | `command-palette-trigger.spec.ts` › `@smoke sidebar shows a clickable command-palette trigger on desktop` |
| 10 | Command palette provider actions | `command-palette-trigger.spec.ts` › `@smoke authenticated palette lists provider actions...` |
| 11 | Compare share URL | `compare-share.spec.ts` › `@smoke copies canonical comparison URL...` |
| 12 | Offering detail breadcrumb | `offering-detail-save.spec.ts` › `@smoke breadcrumb root crumb matches its destination` |
| 13 | Offering edit: ownership guard (#5) | `offering-edit-ownership.spec.ts` › `@smoke blocks a non-owner from the editable form (no Save button)` |
| 14 | Dashboard overview loads | `dashboard-overview.spec.ts` › `@smoke dashboard loads all sections...` |
| 15 | First-login onboarding | `first-login-onboarding.spec.ts` › `@smoke guides a new user...` |
| 16 | Keyboard search shortcut | `keyboard-shortcuts.spec.ts` › `@smoke / focuses marketplace search input` |
| 17 | Keyboard help overlay | `keyboard-shortcuts.spec.ts` › `@smoke ? opens help overlay listing all shortcuts` |
| 18 | Rentals list (empty state) | `rentals.spec.ts` › `@smoke empty state...` |
| 19 | Invoices empty state | `invoices.spec.ts` › `@smoke empty state: fresh user sees FAQ and marketplace CTA` |
| 20 | Transfers empty state | `transfers.spec.ts` › `@smoke empty state: fresh user sees 0 balance and empty transfer list` |
| 21 | Sign in | `signin-flow.spec.ts` › `@smoke should sign in successfully...` |
| 22 | Auth capability endpoint (#436) | `auth-capabilities.spec.ts` › `capability endpoint returns a well-formed boolean @smoke` |
| 23 | Login default surface matches server (#436) | `auth-capabilities.spec.ts` › `login page default surface matches the server capability @smoke` |
| 24 | Verify-email missing token | `verify-email.spec.ts` › `@smoke shows a missing-token error...` |
| 25 | 404 error page | `error-page.spec.ts` › `@smoke 404 renders branded error page with navigation, not blank screen` |
| 26 | Checkout cancel page | `checkout.spec.ts` › `@smoke renders the cancelled-payment page without a contract_id` |
| 27 | UX-001 no fake provider data | `ux-regression-guards.spec.ts` › `@smoke UX-001 homepage hero shows the "Anatomy of a Trust Score"...` |
| 28 | UX-002 validators route retired | `ux-regression-guards.spec.ts` › `@smoke UX-002 /dashboard/validators is retired (404)...` |
| 29 | UX-005 honest homepage stats | `ux-regression-guards.spec.ts` › `@smoke UX-005 homepage stats grid omits dead ICP metrics...` |
| 30 | UX-008 unauth sidebar clean | `ux-regression-guards.spec.ts` › `@smoke UX-008 unauthenticated sidebar hides "My Activity"...` |
| 31 | UX-013 login heading | `ux-regression-guards.spec.ts` › `@smoke UX-013 login page heading reads "Sign In or Create Account"` |
| 32 | UX-004 dashboard @username | `ux-regression-guards.spec.ts` › `@smoke UX-004 dashboard welcome card shows @username, not a raw principal` |

> **Coverage note.** 13 of the 14 critical paths are covered. The remaining
> path — *rent an offering (dialog → real contract)* — is intentionally **not**
> in smoke: its only coverage (`rent-flow.spec.ts`) is >5s and needs complex DB
> seeding, violating the smoke selection rules. It is fully covered by the full
> suite (`npm run test:e2e:fast`).
>
> **2026-07-25 tuning.** 5 slow non-critical specs were demoted from `@smoke`
> (full registration, sign-out, add-device, profile-edit, rentals cancel-action)
> to bring the loop from ~51s/32 back to ~33s/26. They remain full-suite tests.
>
> **2026-08-02 speed pass.** The authed `page` fixture stopped pre-navigating to
> `/dashboard` — ~40 specs already `page.goto()` to their own target in the body,
> so the implicit landing was a wasted second page load per test. Each test now
> navigates exactly where it needs and gates on a page-specific element. Smoke
> dropped from ~40s to a reliable ~27s (6 clean runs at 26–28s), still 26/26.
>
> **2026-08-08 UX regression guards.** Added `ux-regression-guards.spec.ts`
> (6 `@smoke` tests pinning the post-ICP UX cleanup, UX-001/002/004/005/008/013).
> Smoke grew 26→32 and stayed at ~29s (still <35s): the 6 guards are almost all
> anonymous/SSR reads (~0.7–1s each) and parallelize into existing workers, so
> the marginal wall-clock cost was ~1.6s. UX-010 (2px focus outline) and UX-012
> (reduced-motion typing) are documented gaps in §6.

## Mock inventory

The mock policy (website/AGENTS.md): only the Stripe SDK and outbound external
HTTP may be mocked; never first-party API code. Error-path injection must be
done DB-side or as a documented exception. Every mock in the suite, classified:

| Mock | File | Boundary | Classification |
|------|------|----------|----------------|
| Stripe SDK (`loadStripe`) script | `fixtures/stripe-mock.ts` (used by `rent-flow.spec.ts`) | External (js.stripe.com) | ✅ sanctioned external-boundary mock (the canonical one) |
| Stripe Checkout redirect | `rent-flow.spec.ts` (`page.route('https://checkout.stripe.com/**')`) | External (checkout.stripe.com) | ✅ external-boundary; robustness guard, currently never fires (Stripe unconfigured) |
| Hetzner server catalog | `offering-create.spec.ts` (`page.route('**/api/v1/cloud-accounts/*/catalog')`) | External (proxied Hetzner API; needs a real Hetzner token unavailable in tests) | ✅ sanctioned external-boundary exception — the create submit + every other call hit the real API |
| Account-lookup 500 | `account.spec.ts` (patches `window.fetch` for `/api/v1/accounts?publicKey=`) | First-party | ⚠️ **documented exception** — the error-recovery card fires only on a thrown fetch (500/network), which can't be induced DB-side (a deleted row returns null → identity silently dropped, no card); scoped to the one URL |
| Registration create 500 | `registration-flow.spec.ts` (`page.route('**/api/v1/accounts')`) | First-party | ⚠️ **documented exception** — the wizard pre-validates username/email client-side so no real API error can be submitted; a 500 can't be induced without taking the API down; scoped to the one create URL |

No undocumented first-party mocks remain. The two first-party mocks above are
client error-UI tests for branches unreachable without fault injection; they
are scoped to a single URL and every other fetch hits the real API.

## Keeping this file current

This is a **living document**. When you change the suite:

1. **Added a test?** Find the flow it covers in the index. If the flow exists,
   update its Status/Spec/Test row. If it's a new flow, add a row under the
   right category and pick the appropriate tags.
2. **Added a flow with no test?** Add the row with Status ❌ and file a GitHub
   issue so the gap is tracked.
3. **Tagged something `@smoke`?** It must satisfy the smoke selection rules
   above (critical, <5s, low-seed, reliable). Add it to the smoke membership
   table and confirm `npm run test:e2e:fast:smoke` still finishes <35s.
4. **Removed/renamed a test?** Update every row that referenced it; re-check
   the flow's Status (it may drop from ✅ to ❌).
5. **Re-validate periodically** with:
   ```bash
   npx playwright test --list --grep @smoke   # confirm smoke membership
   npm run test:e2e:fast:smoke                # confirm <35s + green
   ```

### Adding a category tag

Tags live in test titles (e.g. `test('@smoke ...')`) and are matched with
Playwright's `--grep`. The category tags (`@auth`, `@marketplace`, …) in this
file are documentation-only labels for the index — they are not required in
test titles. Only `@smoke` is matched at runtime.
