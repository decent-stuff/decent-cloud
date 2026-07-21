# 2026-07-21 — Close #437 / #438 / #439 (UX audit follow-through)

**Scope.** Yesterday's UX audit (`docs/audits/2026-07-21-ux-audit.md`) shipped 12 inline
fixes and filed 3 issues (#437/#438/#439). Today: close all three, TDD-first, against the
warm stack. Then re-audit and refresh `OPEN_ISSUES.md`.

## Stack and gates

- Warm stack is UP (`bash scripts/dev-server.sh status`): api `:59011`, web `:59010`,
  `RATE_LIMIT_ENABLED=false`.
- `cd website && npm run test:e2e:fast:smoke` (4 tests, ~15 s) and `npm run check` are the
  per-unit gates. Full `npm run test:e2e:fast` (~135 tests, ~2.7 m) runs at the end.
- E2E fixtures: `tests/e2e/fixtures/test-account.ts` (fast-auth), `seed-helpers.ts` (DB-direct),
  `auth-helpers.ts` (UI sign-in). Mocks ONLY at the smallest external boundary.

## Issues

### #439 — Marketplace sort hidden on mobile (bug, small)

`repo/website/src/routes/dashboard/marketplace/+page.svelte:1185` wraps the sort pills in
`hidden md:flex`. Add a `<select>` mirror visible on mobile (`md:hidden`) and as an a11y
alternative on desktop. Single source of truth: bind to existing `sortField` / `sortDir`
state and reuse `syncFiltersToUrl()` (line 381). Options: Price ↑, Price ↓, Reliability ↓.

**Acceptance**
- Mobile (375 px) can change sort.
- Sort state syncs to URL on mobile and desktop (existing `syncFiltersToUrl` already handles
  it once the state setters fire).
- Desktop pill UI unchanged.
- E2E: mobile-viewport test asserts the dropdown is visible, changes sort, URL reflects it.

### #438 — Email banner preempts seed-phrase backup banner (bug, recovery risk)

`repo/website/src/routes/dashboard/+layout.svelte:58-103`. The `showSeedPhraseBackupBanner`
derived state includes `!showEmailVerificationBanner`, so a seed-phrase user with unverified
email never sees the backup warning.

**Fix**
- Drop the mutual-exclusion clause; render both banners independently in a stack.
- Each banner independently dismissable (seed banner already has `onDismiss`; email banner
  needs the same treatment — check `EmailVerificationBanner.svelte`).
- Fix `<main>` padding math (line 106): currently conditional on a single banner. With
  stacking, count visible banners (0/1/2) and apply matching `pt-N`. Tailwind v4 — any
  `pt-N` is valid (= N×0.25rem).

**Acceptance**
- Both banners render simultaneously when both conditions apply.
- Stacked layout usable at 375 px.
- E2E: simulate both conditions, assert both banners visible.

### #437 — Click-to-cycle visibility/stock buttons (enhancement, large)

`repo/website/src/routes/dashboard/offerings/+page.svelte:226-239` (`handleVisibilityToggle`,
cycles public→shared→private) and `:212-224` (`handleStockToggle`,
in_stock→out_of_stock→discontinued). Markup at `:792-809`. Title tooltips are invisible on
touch and don't preview the next state.

**Fix**
- New `src/lib/components/StateSelectMenu.svelte`: accessible dropdown built on `<details>`/
  `<summary>` (matches existing component style; see `QuickEditOfferingDialog.svelte` and the
  `<select>` users in `ContactsEditor`/`ExternalKeysEditor` for in-repo patterns). Keyboard:
  Enter/Space opens, Arrow navigates, Escape closes, Enter selects.
- Trigger button shows current state with its color; menu lists all states with one-line
  descriptions. Destructive transitions (public/shared → private; in_stock → discontinued)
  require a confirm step (inline "Are you sure? [Yes / Cancel]" inside the menu).
- Replace both handlers' callers in `+page.svelte:792-809`. Keep `updateOfferingField` as the
  single mutation path.

**States & descriptions**
- Visibility: `public` (green) "Visible to everyone on the marketplace";
  `shared` (blue) "Visible only to people with the link"; `private` (red) "Hidden from the
  marketplace".
- Stock: `in_stock` "Accepts new orders"; `out_of_stock` "Shown but not orderable";
  `discontinued` "Permanently unavailable".

**Acceptance**
- Dropdown/menu replaces click-to-cycle on both buttons.
- Each transition is explicit (no surprise).
- Destructive transition requires confirmation.
- Keyboard accessible.
- E2E: open menu, pick a state, assert offering updates; assert confirm step blocks a
  destructive transition until confirmed.
- `npm run check` clean.

## Out of scope this session

`#418` DA beta onboarding, `#427` Anthropic key proxy, `#416` DA usage dashboard, `#415` DA
subscription billing, all `deferred-post-launch`. #435 (SLA empty state) and #436 (seed-phrase
default) are small UX follow-ups filed after the audit — pull in only if the three above close
with time to spare.

## Sequencing

1. Plan doc (this file).
2. Three parallel subagents — one per issue — each TDD: failing E2E first, then implementation,
   then GREEN, then `npm run check`, then commit per unit, then `gh issue close` with comment.
3. Full `npm run test:e2e:fast` + `npm run check` against integrated tree.
4. Fresh UX audit; file or fix new findings.
5. Update `repo/docs/OPEN_ISSUES.md`; final commit.
