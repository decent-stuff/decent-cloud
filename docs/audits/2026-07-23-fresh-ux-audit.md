# Fresh UX + Visual Audit (No-Mock)

**Date:** 2026-07-23
**Scope:** Full SvelteKit frontend — anonymous + authenticated flows, mobile viewport.
**Stack:** warm stack UP at web:59010 / api:59011 (both healthy).
**Method:** `scripts/browser.js` (fresh Chromium per call) — `errs` on every page, `snap` for structure, `shot` + `zai-vision` for visual checks. No source code modified. Every zai-vision claim was verified against the DOM/source (zai-vision produced false positives, as documented in the 2026-07-23 plan Phase 5).
**Accounts:** freshly created `testuserx9h8pg` (returning user) + seeded rentable provider (`de14c7a0…`) via `dc-auth.js seed-ux-data`.

---

## Top-line summary

**No P0 / Critical.** The app is **launch-ready after a one-line fix (F1)**. The core tenant journey
(landing → marketplace → rent dialog) is fully functional and clean end-to-end. Anonymous browsing,
the rental flow, save/bookmark, and all empty states work. **19 of 20** internal links resolve; the
one broken link is a primary provider-onboarding CTA.

**Counts:** Critical = 0 · **High = 1** · **Medium = 1** · **Low = 1**.

**Re-verification of prior audits (2026-07-21 / 2026-07-23 Phase 5):** all confirmed resolved or
remain known/false-positive — NOT re-reported here:
- Prior #1 (`pt-18` invalid) — confirmed false positive (Tailwind v4 dynamic spacing; `pt-18`=72px, valid). `<main>` class still contains `pt-18`.
- Prior #8 (mobile sort desktop-only) — **FIXED.** Marketplace now has a `<select>` sort dropdown as the sole mobile sort affordance (`+page.svelte:1226-1229`); desktop pills are `hidden md:flex`. Verified via DOM: exactly one visible sort control on mobile.
- Prior #4 (offerings visibility cycle buttons) — filed as issue #437 (deferred); my test user has no offerings so the toggle does not render. Not re-reported.
- Prior #7 (banner stacking) — `<main>` now carries `pt-56 md:pt-36` stacked-banner padding (issue #438 track).
- Known dev noise (Lit dev-mode warning, Stripe.js-over-HTTP, Vite HMR `REQUESTFAILED`/`ERR_ABORTED` on `@vite`/`.svelte-kit` modules) — present, all dev-only, excluded from findings.

---

## Findings

### F1 — Dashboard "Get Started" provider-onboarding CTA links to a 404
**Severity:** High
**Page:** `/dashboard` (authenticated, empty "My Resources" section)
**Confidence:** 10/10
**Evidence:**
- `routes/dashboard/+page.svelte:787` — `href="/dashboard/provider"`
- `curl -o /dev/null -w "%{http_code}" http://localhost:59010/dashboard/provider` → **404**
- No `+page.svelte` exists at `routes/dashboard/provider/` (only sub-routes: `provider/agents`, `provider/support`, etc.)
- This is the **only** broken internal link: a scan of all 20 unique internal hrefs rendered across dashboard pages returns 200 for 19 and 404 for this one alone.

**Description:** The "My Resources → No resources yet" empty-state card has a prominent **Get Started**
button whose `href="/dashboard/provider"` resolves to a 404 ("Page not found"). This is the canonical
provider-onboarding CTA on the authenticated dashboard home — the first thing a new provider-to-be
clicks after signing in. Clicking it dumps them on the 404 page.

**Inconsistency:** The *same* "become a provider" intent is wired correctly elsewhere — the first-login
`WelcomeModal` uses `getActivationActionHref('provider')` → `/dashboard/provider/support` (valid), and
the landing-page "Become a Provider" → `/dashboard/provider/support` (valid). Only this dashboard
empty-state CTA uses the bare, non-existent `/dashboard/provider`.

**Mitigations (why High, not Critical):** (a) the 404 page is graceful — it offers "Back to home" and
"Browse marketplace" recovery links; (b) two other working paths to provider setup exist (WelcomeModal,
offerings page "Create Your First Offering" → `/dashboard/offerings/create`).

**Suggested fix:** One-line change — `href="/dashboard/provider/support"` (mirrors the WelcomeModal /
landing page), or `/dashboard/offerings/create` to drop the user straight into the create form.

---

### F2 — First-login onboarding modal reappears every session and mislabels a complete profile
**Severity:** Medium
**Page:** `/dashboard` (authenticated, on mount)
**Confidence:** 8/10
**Evidence:**
- `lib/components/WelcomeModal.svelte:18` — `let open = $state(browser ? !isOnboardingCompleted(sessionStorage) : false);`
- `lib/components/welcome-onboarding.ts:3` — `ONBOARDING_SESSION_KEY = 'first_login_onboarding_completed'` (read from **`sessionStorage`**)
- Modal heading is unconditional: `WelcomeModal.svelte:95` — `<h2 …>Complete your profile</h2>` regardless of `hasUsername`/`hasEmail` state.

**Description:** The "Complete your profile" onboarding modal's visibility is gated on
`sessionStorage['first_login_onboarding_completed']`, which is cleared whenever the browser/tab
session ends. Despite the key's "first login" name, the implementation makes it **first visit per
session**. A returning user who closes and reopens their browser sees the onboarding modal again on
every new session — even when their profile is already complete. Worse, the modal always reads
"Complete your profile" while simultaneously showing the user's username and email both in green
("complete"), so the heading is factually wrong for anyone who has already finished onboarding.

**User impact:** Every returning user is interrupted by a redundant, slightly misleading "Complete your
profile" modal each time they start a fresh session, requiring an extra click to dismiss before they
reach the dashboard.

**Suggested fix:** Either (a) gate visibility on a backend `profile_completed` flag (or `localStorage`)
for true first-login semantics, or (b) keep per-session behavior but make the heading conditional —
e.g. "Profile complete" / skip straight to step 2 when `hasUsername && hasEmail`.

---

### F3 — `/docs` and `/pricing` are not routes (404), and no general pricing page exists
**Severity:** Low
**Page:** `/docs`, `/pricing` (direct URL only)
**Confidence:** 7/10
**Evidence:**
- `curl http://localhost:59010/docs` → 404; `curl http://localhost:59010/pricing` → 404
- No `+page.svelte` at `routes/docs/` or `routes/pricing/`.
- Neither path is linked from any navigation (verified by scanning all rendered hrefs); only reachable by URL guessing.

**Description:** Neither path exists. `/docs/decent-cloud-whitepaper.pdf` (linked from the landing
footer) does resolve correctly (200, application/pdf), so the 404 is only for the bare `/docs` index.
There is a product pricing surface at `/agents/pricing` ("CHF 49 / month") but no general `/pricing`
page — a user looking for the core product's pricing who types `/pricing` lands on the 404. The 404
page itself is graceful (recovery links).

**Suggested fix:** Optional — add a redirect `/pricing` → `/agents/pricing` (and/or `/docs` → the
whitepaper or a docs index) to catch URL guesses; or leave as-is since nothing links here.

---

## Routes reviewed with no actionable findings

**Anonymous flow (0 console errors after filtering dev noise):**
- `/` (landing) — clean, AAA-contrast dark theme; zai-vision flagged nothing real.
- `/login` — Google + seed-phrase options, recovery + back links. Clean.
- `/dashboard/marketplace` — accessible to anonymous (Sign-In gate); default-hide "Show N offerings" reveal works; mobile sort works (prior #8 resolved).
- `/dashboard/reputation/<bogus>` — renders a proper "No Account Data" card with "Back to Marketplace" (not a hard error). Public, as intended.
- `/dashboard/reputation` (list) + `/dashboard/validators` — clean empty states ("Search Reputation", "No active validators found" + "Become a Validator").

**Authenticated flow (0 console errors after filtering dev noise on every page):**
- `/dashboard` — strong onboarding: "Ready to get started?" + Browse Marketplace / View Validators CTAs, Quick Actions, profile nudge. (See F1 for the broken CTA within it.)
- `/dashboard/rentals` — "No Rentals Yet" empty state with numbered steps + "Browse GPU Servers" CTA.
- `/dashboard/marketplace` — Rent flow verified end-to-end: enabled "Rent" button → "Rent Resource" dialog (Rental Duration, SSH key field w/ generate + help link, prorated-refund note, Cancel). Save/bookmark toggle ("Save" → "Saved") works and persists to `/dashboard/saved`.
- `/dashboard/marketplace/[id]` + `/compare` — render, no errors.
- `/dashboard/account`, `/account/profile`, `/account/billing`, `/account/notifications`, `/account/security`, `/account/subscription` — all well-built (Security: Devices/SSH Keys/2FA/API Tokens; Subscription: Free/Pro tiers; Billing: invoice form). No "undefined"/"NaN"/"loading…" stuck states.
- `/dashboard/cloud/resources`, `/cloud/accounts` ("Connect your Hetzner or Proxmox…" + Add Account), `/invoices` (educational "When will I see invoices?" empty state), `/transfers` (0.0000 balance + "No Transfers"), `/offerings` + `/offerings/create` (template presets, required-field markers, draft checkbox), `/saved`, `/provider/agents`, `/agents`, `/agents/pricing` — all clean.
- All 12 `dashboard/provider/*` sub-routes + all `account/*` sub-routes return 200.

**Mobile viewport (375×812):** landing, marketplace, dashboard all clean — no overflow, no overlap, primary actions tappable. Mobile sort confirmed working.

## What works well (launch-positive)
- Consistent empty states with educational copy + CTAs (rentals, invoices, saved, offerings, transfers).
- Graceful 404 page with recovery navigation.
- Verified false-positive resistance: every zai-vision visual claim was checked against the DOM (e.g. it falsely reported "two sort controls on mobile" — DOM confirms one).

## Verdict
**Launch-ready after fixing F1** (one-line `href` change). F2 is recommended polish for the
returning-user experience; F3 is optional. No blocking defects in the primary tenant journey.
