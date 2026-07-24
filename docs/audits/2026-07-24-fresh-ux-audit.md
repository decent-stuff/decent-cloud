# Fresh Read-Only UX Audit — 2026-07-24

**Scope:** Full read-only UX walk of the Decent Cloud web app (SvelteKit) against the live warm
stack (web `:59010`, api `:59011`, both healthy). No mocks, no code changes, no commits.

**Method:** Real browser automation via `scripts/browser.js` (snap/shot/errs/html/tour/click/fill).
Two personas walked: **first-time** (anonymous → register → onboarding → browse → rent dialog) and
**returning** (seed-authed provider+renter account → every dashboard route). Every route under
`website/src/routes/` was visited (anonymous + authed where relevant). Visual evidence captured via
zai-vision for the highest-impact finding.

**Test account:** `uxverify1784895820` (seeded as both renter and provider via
`scripts/dc-auth.js seed-ux-data`; 3 offerings #667/668/669, 1 agent pool online).

**Pre-flight:** `docs/OPEN_ISSUES.md` read in full. Findings below are **net-new** — they do not
duplicate any item in OPEN_ISSUES.md (neither the "Triaged as non-bugs" / "Deferred product
decisions" sections, nor the recently-closed fixes). Cross-checked against today's two sibling
audits (`2026-07-24-coverage-and-ux-flow.md` = E2E gaps + UX *optimization* suggestions;
`2026-07-24-code-robustness.md` = backend robustness) — no overlap.

## Summary

| Severity | Count |
|----------|-------|
| 🔴 critical | 0 |
| 🟠 high | 0 |
| 🟡 medium | 1 |
| 🟢 low | 4 |

Overall the app is in strong shape. Console was clean on every route (the only `REQUESTFAILED`
lines are the known Vite dev-server aborts from `browser.js`'s auth double-navigation dance — a
tooling artifact, not an app bug). Empty states are uniformly helpful with CTAs and no dead-ends.
Access control (admin page), error handling (verify-email / checkout missing-param pages,
nonexistent contract), and the rent dialog all behave correctly. The single medium finding is a
misleading data-presentation issue; the rest are cosmetic.

---

## 🟡 Findings

### [MEDIUM] Reputation report shows a red "Poor" uptime badge for providers with zero health checks
- Route: `/dashboard/reputation/[identifier]`
- Persona: returning (renter evaluating a provider)
- Repro:
  1. Sign in, open the marketplace, click a provider link → provider profile (`/dashboard/providers/[id]`).
  2. Click "Full reputation report" → `/dashboard/reputation/[id]`.
  3. Observe the "Provider Health (Last 30 Days)" card for a new/unmonitored provider.
- Evidence:
  - Reputation page renders `Uptime 0.0%` with a red **`Poor`** badge (`badge-danger`) while
    `Health Checks` shows `0 / 0 healthy` and `Contracts Monitored` is `0`.
  - Logic at `website/src/routes/dashboard/reputation/[identifier]/+page.svelte:470-478`:
    ```svelte
    {#if healthSummary.uptimePercent >= 99} … Excellent
    {:else if healthSummary.uptimePercent >= 95} … Good
    {:else if healthSummary.uptimePercent >= 90} … Fair
    {:else} <div class="badge badge-danger mt-2">Poor</div> {/if}
    ```
    There is no guard for `totalChecks === 0`, so absence-of-data (0.0% uptime) falls through to
    the red "Poor" branch.
  - Screenshot `/tmp/dc-rep-poor.png` analyzed via zai-vision: confirms a light-red badge reading
    "POOR" next to "0 / 0 healthy" — "misleading to display the POOR quality label when there are
    zero health checks… suggests a negative outcome from an evaluation that never took place."
  - Note the *provider profile* page (`/dashboard/providers/[id]`) does NOT have this bug — it
    shows `0.0%` with neutral subtext `0 checks`. Only the deeper reputation report is affected.
- Confidence (1-10) this is a real, correct-to-fix issue: **9**
  - This is the public report a renter reads when deciding whether to trust a provider. A red
    "Poor" label unfairly penalizes every brand-new provider (and any provider not yet enrolled in
    health monitoring). Spirit-identical to the already-fixed #435 (SLA chart showed misleading
    empty gray bars when `reports30d === 0`); same class of bug, different metric/page.
- Safety (1-10) that fixing won't break things: **9**
  - Add a `totalChecks === 0` (or `contractsMonitored === 0`) branch that renders a neutral
    `No data` / `Not monitored yet` badge instead of the percentage ladder. Existing e2e
    (`reputation-detail.spec.ts`) seeds its own account and can assert the empty state.
- Suggested fix sketch:
  ```svelte
    {#if healthSummary.totalChecks === 0}
      <div class="badge badge-neutral mt-2">No data yet</div>
    {:else if healthSummary.uptimePercent >= 99} … Excellent
    …
  ```

---

## 🟢 Findings (low / cosmetic)

### [LOW] Stale "© 2025" copyright year in the footer
- Route: all pages (footer is global) — e.g. `/`, `/agents/pricing`, `/dashboard/*`
- Persona: both
- Repro: Scroll to the footer of any page.
- Evidence: `website/src/lib/components/Footer.svelte:81` → `&copy; 2025 Decent Cloud &middot; …`.
  Hardcoded year; today is 2026-07-24. The landing stats and copy elsewhere read as a live 2026
  product, so the stale year is incongruous.
- Confidence: **10** (literal hardcoded string).
- Safety: **10** — either make it dynamic (`new Date().getFullYear()`) or bump to 2026.
- Suggested fix sketch: `&copy; {new Date().getFullYear()} Decent Cloud`.

### [LOW] `/dashboard/user/[identifier]` route is orphaned — no inbound links anywhere in the app
- Route: `/dashboard/user/[identifier]`
- Persona: returning
- Repro: The page exists and works (loads offerings + rentals-as-requester + rentals-as-provider),
  but no navigation element links to it.
- Evidence: `rg "dashboard/user"` across `website/src` (excluding the route's own directory)
  returns exactly one hit — the page's own self-redirect when a pubkey identifier resolves to a
  username (`+page.svelte:49`). No sidebar item, no card, no profile/reputation page points here.
  It is reachable only by direct URL entry. Functionally it is largely a subset of
  `/dashboard/providers/[id]` + `/dashboard/reputation/[id]`, both of which ARE linked.
- Confidence: **8** — it is genuinely unreachable via normal navigation; the small chance it is an
  intentional "shareable by URL" view is why this isn't higher.
- Safety: **7** — removing the route could break any external bookmarks/shares; safer to either
  (a) link it from somewhere meaningful, or (b) redirect it to the richer reputation page.
- Suggested fix sketch: Decide intent. If public user-activity view is wanted, link it (e.g. from
  the reputation report). If not, replace with a redirect to `/dashboard/reputation/[identifier]`.

### [LOW] Breadcrumb labels the root crumb "Dashboard" but links to `/dashboard/rentals` (My Rentals)
- Route: `/dashboard/marketplace/[id]` (offering detail)
- Persona: returning
- Repro: Open any offering detail page; look at the breadcrumb trail.
- Evidence: `website/src/routes/dashboard/marketplace/[id]/+page.svelte:340`:
  ```ts
  isAuthenticated ? { label: 'Dashboard', href: '/dashboard/rentals' } : { label: 'Home', href: '/' }
  ```
  The crumb reads "Dashboard" (conventionally `/dashboard`, the overview) but navigates to
  `/dashboard/rentals` ("My Rentals"). A user clicking "Dashboard" to get back to their overview
  lands on the rentals list instead.
- Confidence: **9** — label/destination mismatch is clear from the source.
- Safety: **9** — either relabel to "My Rentals" (matches destination) or repoint href to
  `/dashboard` (matches label).
- Suggested fix sketch: `{ label: 'My Rentals', href: '/dashboard/rentals' }` (cheapest; the
  next crumb is already "Marketplace").

### [LOW] Subscription tiers advertise "14-day free trial" but the only CTA is "Contact Sales"
- Route: `/dashboard/account/subscription`
- Persona: returning
- Repro: Open Account → Subscription; read the Pro and Enterprise plan cards.
- Evidence: Both Pro (`$29/mo`) and Enterprise (`$99/mo`) cards print "14-day free trial"
  immediately under the price, but each card's action button is "Contact Sales" — there is no
  "Start trial" / "Upgrade" affordance. The "Need Help?" block below says plans "can be upgraded
  or downgraded at any time," implying a self-serve path that the CTAs don't provide.
- Confidence: **7** — could be an intentional sales-led funnel; flagged so product can confirm
  intent. If trials are genuinely available, the copy/CTA disconnect will frustrate upgraders.
- Safety: **8** — if self-serve trials aren't built yet, remove the "14-day free trial" line; if
  they are, swap the CTA to a trial-start action.
- Suggested fix sketch: Align copy with intent — either drop the trial line, or wire the button to
  the Stripe trial checkout.

---

## Routes checked (all visited, console clean unless noted)

Anonymous: `/`, `/login` (+ registration seed/username steps via source), `/recover`, `/verify-email`
(no-token error path ✓), `/agents`, `/agents/pricing`, `/checkout/success` (no-session error ✓),
`/checkout/cancel`, `/offline`.

Authed (provider+renter account): `/dashboard`, `/dashboard/marketplace` (+detail `[id]`, +compare),
`/dashboard/offerings` (+create, +`[id]/edit`), `/dashboard/rentals` (+`[contract_id]` not-found ✓),
`/dashboard/saved`, `/dashboard/transfers`, `/dashboard/invoices`, `/dashboard/validators`,
`/dashboard/reputation` (+`[id]` report, +`[id]/trust`), `/dashboard/providers/[id]`,
`/dashboard/user/[id]`, `/dashboard/cloud/accounts`, `/dashboard/cloud/resources`,
`/dashboard/admin` (access-denied ✓), `/dashboard/provider/{support,earnings,analytics,sla,requests,
feedback,ssh-key-rotations,password-resets,agents,agents/[pool_id],reseller}`, full
`/dashboard/account/{,profile,security,subscription,billing,notifications}`.

Mobile (375×812): marketplace — clean, hamburger menu present, sort `<select>` present, no overlap
or horizontal-scroll (zai-vision confirmed; only nit was a slightly small banner close-button,
not worth a finding).

## Things explicitly checked and found NOT-broken (no finding)
- Rent dialog opens correctly; for own-offering shows "No Payment Required"; SSH-key required field
  + "Generate for me"; duration/OS/payment sections all render.
- Empty states on Rentals / Saved / Requests / Feedback / Transfers / Invoices / Cloud Accounts /
  Cloud Resources / Analytics / Earnings — all contextual with CTAs, no dead-ends.
- `/agents` "Start beta" form: endpoint `POST /api/v1/agents-waitlist` returns 200; success/error
  feedback wired (`status === 'ok'` / `Could not save signup: …`).
- Provider agent pool list + detail render with online/agents/contract counts.
- Whitepaper PDF link (`/docs/decent-cloud-whitepaper.pdf`) → 200 (not a dead link).
- Console errors across all routes: none beyond the known browser.js auth-dance Vite aborts.

## Honorable mentions (intentional, NOT reported as findings)
- Landing hero "provider_alpha / 87 Trust Score / 1,247 contracts / Updated 2m ago" card is a
  decorative marketing mockup (`HeroSection.svelte` comment says "card mockup"; `hidden lg:block`).
  The static "Updated 2m ago" next to the *real* "Marketplace Statistics" block below is slightly
  incongruous, but it is clearly decorative — left to product/design judgment, not filed.
