# Broad Review — 2026-08-11 (post PR #479 / `05849932`)

**Scope:** Comprehensive read+plan review of code, functional, and visual issues across the
decent-cloud app after the marketplace buy-flow (#0 priority) merged. **This is a PLAN only —
no fixes are implemented here.** Findings feed the next execution phase.

**Method:** Warm stack (api `:59011` 200, web `:59010` 200, pg `:5432` ok). Read of
`FLOWS.md`, `playwright.config.ts`, all `tests/e2e/*.spec.ts` catalogs, `OPEN_ISSUES.md`,
`PRODUCT-DIRECTION.md`, and the large Rust/Svelte files. UX driven for real (no mocks) via
`scripts/browser.js` + `dc-auth.js create-user` against the live stack; layouts verified with
the `zai-vision` MCP (the opencode model has no native vision).

**Branch:** `review/2026-08-11-broad-review` (from `main`). **Identity:** `andris-k85`.

---

## Summary verdict

The harness and product are in **strong shape** post-#479. The e2e catalog (`FLOWS.md`) is
the most disciplined test-coverage artifact in the repo, the route-audit spec tours every
route with real defect detection, the marketplace correctly shows ONE real Hetzner offering
(demos gone ✅), and the provider-start page is genuine 3-step technical onboarding (not a
support-profile stub ✅). The buy flow (#0) is end-to-end verified.

The highest-value problems are **two honesty violations** on the reputation surface (trust
scores shown for zero-track-record providers — directly contradicting `PRODUCT-DIRECTION.md`),
a **wallet-page auth-resolve flash**, and a **slow full suite (~11 min, not "seconds")** driven
by serial-mode sprawl (30 specs) + route-audit's per-route link-checking. None are P0; several
are P1.

---

## Findings (value ≥ 4)

Columns: `ID | Severity | value | effort | Finding (file:line) | Recommended fix | Confidence`.
Severity scale: P0 blocker · P1 high · P2 medium · P3 low. value/effort/confidence on 1–10 / S-M-L / 1–10.

### E2E track

| ID | Sev | val | eff | Finding (file:line) | Recommended fix | Conf |
|----|-----|-----|-----|---------------------|-----------------|------|
| E2E-1 | P1 | 8 | M | **Full suite ~11 min, not "seconds".** Root causes: (a) **30 of ~90 specs force `mode:'serial'`** (`rg -l "mode: 'serial'" tests/e2e/*.spec.ts`), serializing their tests onto one worker and crushing parallelism; (b) `route-audit.spec.ts` runs **~40 route tests** each with `test.setTimeout(60_000)` (`:638,:724,:739`) and does up to **25 HTTP link-checks per route** (`:530`) — even de-duped globally this is O(routes×first-seen-links) network I/O. `AGENTS.md:62` still claims "~200 tests in ~2.9 m" — stale (suite is now ~354 `test()` calls). | (1) Audit the 30 serial specs: move specs that use serial only defensively (not for shared-pubkey DB writes) back to parallel. (2) In route-audit, cap link-checks to a smaller sample or move broken-link checks to a separate once-per-suite test. (3) Update the `AGENTS.md` timing claim. Target full suite <5 min. | 8 |
| E2E-2 | P2 | 6 | S | **`FLOWS.md` smoke membership drift.** Doc states "~31 tests" / smoke table lists 31 rows (`:207`, `:224-256`); actual `@smoke` count is **34** (verified: `34 passed (29.6s)`). Recent additions (UX-004, command-palette, etc.) weren't all reflected in the table count line. | Reconcile the smoke table + the "~31/<35s" line with the actual 34 (run `npx playwright test --list --grep @smoke`). Pure doc fix. | 9 |
| E2E-3 | P2 | 6 | S | **Provider onboarding `/dashboard/provider/start` has no interactive-wizard e2e on this route.** The page is a static 3-step guide (agent → offering → support). The `become-provider`/`provider-onboarding-submit` specs cover the Help-Center support form, not the step-through on `/provider/start` itself, so the "does step 1 actually advance / does each CTA land correctly" path is render-only here. | Add a thin spec that visits `/dashboard/provider/start`, asserts the 3 numbered steps render in order, and that each CTA (`Read the installation guide`, `Create an offering`, `Open the support profile`) resolves to a 2xx target. | 7 |
| E2E-4 | P2 | 5 | M | **Wallet top-up Stripe Checkout redirect is uncovered on the client side.** `payment-flows.spec.ts` closes the backend webhook half only; the `window.location.href = checkoutUrl` redirect (`website/src/routes/dashboard/wallet/+page.svelte:92`) — the actual buyer-facing button→redirect — is not asserted (cross-origin, documented gap). | Assert the top-up button issues the signed POST and that the returned `checkoutUrl` is a `https://checkout.stripe.com/...` URL (intercept the `window.location` assignment or the POST response) — no real payment. | 6 |
| E2E-5 | P3 | 4 | S | **`provider/*` sub-pages (analytics, feedback, reseller, ssh-key-rotations, password-resets) are render/empty-state only** (`provider-pages-smoke.spec.ts`). FLOWS marks password-resets ⚠️; the others are ✅ but really only "renders heading + empty state". Acceptable for stub surfaces, but any future feature landing there has no behavioral coverage. | Leave as-is until those surfaces gain real features; flag in FLOWS as ⚠️ (render-only) rather than ✅ so the distinction is honest. | 8 |

### UX track (driven for real, no mocks)

| ID | Sev | val | eff | Finding (file:line) | Recommended fix | Conf |
|----|-----|-----|-----|---------------------|-----------------|------|
| UX-1 | P1 | 8 | S | **Reputation leaderboard shows trust scores for providers with ZERO completed contracts** — violates `PRODUCT-DIRECTION.md` "honest N/A score when there is no track record (never a green Reliable badge on empty data)". Live leaderboard renders: `uxprovidergfpgu4` trust **90** / completed **0**, `uxproviderkrjyjv` 90/0, `hetzner-reseller` 70/0 — only `probe46512698` honestly shows "—". Independently confirmed by `zai-vision` ("not honest… misleading and erodes trust"). | Gate the displayed trust score (and the leaderboard honesty filter) on `completed_contracts > 0`: show "—" when there is no track record. Verify the `total_contracts > 0` honesty gate (FLOWS §1) is actually applied — these rows prove it is NOT excluding zero-track providers. | 9 |
| UX-2 | P1 | 7 | S | **The same dishonest trust score (70) surfaces on the marketplace offering card** for `hetzner-reseller` (`/dashboard/marketplace` row badge), i.e. the buy-flow entry point shows a reliability number backed by zero completed rentals. Compounds UX-1 on the #0-priority surface. | Same fix as UX-1 (honest N/A in the offering-card badge when the provider has no completed contracts). | 8 |
| UX-3 | P2 | 6 | S | **Wallet page flashes "Login Required" for an authenticated user.** Two consecutive `browser.js --seed` runs (same seed that renders authed content on reputation/cloud/marketplace) showed the `AuthRequiredCard` ("🔑 Login Required") on `/dashboard/wallet`. Mechanism confirmed by `wallet-ui.spec.ts:31` `waitForWalletReady` (15s wait): the page gates the card on `isAuthenticated` (`+page.svelte:124`) which stays `false` during the async identity-derivation window after `onMount`; sibling pages don't flash because they're SSR'd or gated differently. A real user on a slow connection sees a confusing "Login Required" before the balance appears. | Defer the `AuthRequiredCard` until the auth store has actually settled (render the loading spinner, not the login-required card, while `currentIdentity` is unresolved); or gate on a tri-state (`loading | authed | unauthed`) like sibling pages. | 7 |
| UX-4 | P2 | 6 | S | **Leaderboard is polluted with test/seed provider accounts** (`uxprovidergfpgu4`, `uxproviderkrjyjv`, `probe46512698`) alongside the one real provider (`hetzner-reseller`). These are automation-seeded identities; on a public prod leaderboard they'd be noise that undermines "the catalog is the product". | Add a prod-only filter (e.g. exclude accounts created by the test seeder / without a real provider profile / display_name), or gate leaderboard rows on a real `provider_profile` + offerings. Coordinate with UX-1's honesty gate. | 6 |
| UX-5 | P3 | 4 | L | **`offerings/create/+page.svelte` is an 873-line single component** — the largest frontend file, mirroring the Rust large-file issue (#444) on the Svelte side. High churn risk for the create-offering flow (a #0-adjacent surface). | Extract sub-components (catalog picker, spec form, pricing form, SSH-key field) when next touched; track as the frontend analogue of #444. | 7 |

### TechDebt track

| ID | Sev | val | eff | Finding (file:line) | Recommended fix | Conf |
|----|-----|-----|-----|---------------------|-----------------|------|
| TD-1 | P2 | 7 | L | **Large Rust files still outstanding (issue #444).** `dc-agent/src/main.rs` **3681L** (provisioning loop, gateway, polling, setup all in one), `api/src/database/offerings.rs` **2981L** (grew from the 2944 cited in OPEN_ISSUES A3), `api/src/database/cloud_resources.rs` **2701L**, plus `api/src/openapi/providers.rs` **4082L** and `api/src/openapi/cloud.rs` 1821L. `accounts.rs` was already split (A3 done); these remain. | Continue the #444 split roadmap (`docs/plans/2026-07-25-large-file-splits-444.md`). `openapi/providers.rs` (4082L) is the most tractable next target (handler-group split like `accounts.rs`). dc-agent `main.rs` needs the concurrency refactor (A5) first to split cleanly. | 9 |
| TD-2 | P2 | 6 | M | **`let _ =` silent-error sweep is clean.** `rg` shows only 2 live sites (`api/src/bin/api-cli/gateway.rs:235,276`) — both intentional channel-close/recv on CLI shutdown, not Result-swallowing. `OPEN_ISSUES` A13 already audited this. **No production `let _ =` swallowing remains** — this is a positive finding, no action. | None (document as clean). | 9 |
| TD-3 | P3 | 4 | S | **No `TODO`/`FIXME`/`HACK` markers** in `api/src`, `dc-agent/src`, `common/src`, or `website/src/{routes,lib}` production code (rg returned empty). The Stripe-URL literal in `api/src/openapi/contracts.rs:1480,1487` is test-fixture data (`cs_test_abc`), not a hardcoded-URL violation (the `STRIPE_API_BASE` const rule is respected). | None (document as clean). | 9 |

### Alignment track (vs `docs/PRODUCT-DIRECTION.md`)

| ID | Sev | val | eff | Finding (file:line) | Recommended fix | Conf |
|----|-----|-----|-----|---------------------|-----------------|------|
| AL-1 | — | 9 | — | **POSITIVE: alignment infrastructure is in place.** `PRODUCT-DIRECTION.md` exists and is linked authoritatively from `AGENTS.md:8`. The marketplace correctly shows ONE real offering (`Hetzner CX23 Dev`, `hetzner-reseller`, $6.00, Nuremberg) — demo/synthetic offerings are GONE ✅, matching "drop the demo offerings". The provider-start page is genuine 3-step technical onboarding (install agent → list offering → support profile), matching "Become a Provider must mean real onboarding, not just a support-profile wizard" ✅. Cloud Accounts Add-modal is provider-agnostic (Hetzner/Proxmox) — single-common-API ✅. | None (keep it this way). | 9 |
| AL-2 | P1 | 8 | S | **Trust/reputation honesty FAILS the alignment checklist** ("Does it make trust/reputation more prominent and more honest?" → currently NO). See UX-1/UX-2: trust scores render with no track record. This is the single most important alignment gap. | Fix UX-1/UX-2 (honest N/A). | 9 |

---

## Coverage gap map (dashboard route → test status)

Legend: ✅ dedicated behavioral test · ⚠️ render/empty-state only (route-audit or smoke) · ❌ none.

| Route | Status | Covered by |
|-------|--------|-----------|
| `/dashboard` | ✅ | `dashboard-overview.spec.ts` (@smoke) |
| `/dashboard/account` (overview) | ✅ | `account-page.spec.ts` |
| `/dashboard/account/billing` | ✅ | `billing-settings.spec.ts` |
| `/dashboard/account/notifications` | ✅ | `account-notifications.spec.ts` |
| `/dashboard/account/profile` | ✅ | `profile-page.spec.ts`, `account-profile-edit.spec.ts` (@smoke) |
| `/dashboard/account/security` | ✅ | `account-add-device.spec.ts` (@smoke) |
| `/dashboard/admin` | ✅ | `admin-dashboard.spec.ts`, `admin-account-mutations.spec.ts` |
| `/dashboard/cloud/accounts` | ✅ | `cloud.spec.ts` (empty + populated + modal delete) |
| `/dashboard/cloud/resources` | ⚠️ | route-audit render only — no dedicated flow test |
| `/dashboard/invoices` | ✅ | `invoices.spec.ts` |
| `/dashboard/marketplace` | ✅ | `anonymous-browsing.spec.ts`, `search-dsl.spec.ts`, `marketplace-sort.spec.ts` |
| `/dashboard/marketplace/[id]` | ✅ | `rentable-offering-fixture.spec.ts`, `offering-detail-save.spec.ts` |
| `/dashboard/marketplace/compare` | ✅ | `compare-share.spec.ts` (@smoke share URL) |
| `/dashboard/offerings` | ✅ | `offerings-status-menus.spec.ts`, `offering-edit.spec.ts` |
| `/dashboard/offerings/[id]/edit` | ✅ | `offering-edit.spec.ts`, `offering-edit-ownership.spec.ts` (@smoke) |
| `/dashboard/offerings/create` | ✅ | `offering-create.spec.ts` (real signed POST) |
| `/dashboard/provider/agents` | ✅ | `agent-pool-create.spec.ts`, `agent-pool-edit.spec.ts` |
| `/dashboard/provider/agents/[pool_id]` | ✅ | `agent-pool-edit.spec.ts` (detail render + rename) |
| `/dashboard/provider/analytics` | ⚠️ | `provider-pages-smoke.spec.ts` (empty-state render) |
| `/dashboard/provider/earnings` | ✅ | `provider-earnings.spec.ts` |
| `/dashboard/provider/feedback` | ⚠️ | `provider-pages-smoke.spec.ts` (render) |
| `/dashboard/provider/password-resets` | ⚠️ | `provider-pages-smoke.spec.ts` (render) — FLOWS marks ⚠️ |
| `/dashboard/provider/requests` | ✅ | `provider-accept-reject.spec.ts`, `provider-requests-auth.spec.ts` |
| `/dashboard/provider/reseller` | ⚠️ | `provider-pages-smoke.spec.ts` (render) |
| `/dashboard/provider/sla` | ✅ | `provider-response-metrics.spec.ts` (@smoke), `offering-sla-empty-state.spec.ts` |
| `/dashboard/provider/ssh-key-rotations` | ⚠️ | `provider-pages-smoke.spec.ts` (render) |
| `/dashboard/provider/start` | ⚠️→✅ | `provider-start-cta.spec.ts`, `become-provider.spec.ts` (@smoke render) — **interactive step-through gap (E2E-3)** |
| `/dashboard/provider/support` | ✅ | `provider-onboarding-submit.spec.ts` (Help Center form) |
| `/dashboard/providers/[identifier]` | ✅ | `providers.spec.ts` |
| `/dashboard/rentals` | ✅ | `rentals.spec.ts` (@smoke empty state) |
| `/dashboard/rentals/[contract_id]` | ✅ | `rentals.spec.ts`, `rent-flow.spec.ts`, `rental-detail-cancel.spec.ts` |
| `/dashboard/reputation` | ✅ | `reputation-leaderboard.spec.ts`, `reputation.spec.ts` |
| `/dashboard/reputation/[identifier]` | ✅ | `reputation-detail.spec.ts` |
| `/dashboard/reputation/[identifier]/trust` | ✅ | `reputation-trust.spec.ts` |
| `/dashboard/saved` | ✅ | `saved-offerings.spec.ts` |
| `/dashboard/user/[identifier]` | ✅ | `user.spec.ts` (@smoke) |
| `/dashboard/wallet` | ✅ | `wallet-ui.spec.ts`, `wallet-api.spec.ts` — **Stripe-redirect client gap (E2E-4)** |

**Gaps the task specifically asked about:**
- Wallet top-up (Stripe Checkout → webhook credit): webhook half ✅ (`payment-flows.spec.ts`); client redirect ⚠️ (E2E-4).
- Profile/account settings: ✅ fully covered (profile, billing, notifications, security).
- Provider onboarding: ✅ support form; ⚠️ `/provider/start` interactive step-through (E2E-3).
- Cloud accounts management: ✅ add/list/delete covered (`cloud.spec.ts`).
- Reputation/leaderboard: ✅ covered — but the **data it renders is dishonest** (UX-1, a product bug, not a coverage gap).
- Search/filter DSL: ✅ `search-dsl.spec.ts` (8 tests).
- Keyboard shortcuts: ✅ `keyboard-shortcuts.spec.ts` (@smoke `/` + `?`).
- Email verification gate: ✅ `rent-email-verification-gate.spec.ts` + `verify-email.spec.ts`.

---

## Recommended execution order

**Phase 1 — S-effort, high value (do first):**
1. **UX-1 / UX-2 / AL-2** — honest trust score (gate on completed_contracts). Highest product value, smallest fix, directly fixes the alignment-checklist failure.
2. **UX-3** — wallet auth-resolve flash (tri-state gate).
3. **UX-4** — leaderboard test-account filtering (coordinate with UX-1).
4. **E2E-2** — reconcile `FLOWS.md` smoke count (doc-only).
5. **E2E-3** — `/provider/start` interactive step spec.
6. **E2E-5** — re-label render-only provider pages ⚠️ in FLOWS.

**Phase 2 — M-effort:**
7. **E2E-1** — serial-mode audit + route-audit link-check trimming (full-suite time).
8. **E2E-4** — wallet Stripe-redirect client assertion.
9. **TD-2/TD-3** — no action (documented clean; included for completeness).

**Phase 3 — L-effort (defer / coordinate):**
10. **TD-1** — large-file splits (continue #444; `openapi/providers.rs` 4082L next).
11. **UX-5** — `offerings/create` 873L component split (touch when next editing that flow).

---

## Low-confidence items (confidence < 6 — needs a human/poc before acting)

| ID | Finding | Why low-confidence | Next step to raise confidence |
|----|---------|--------------------|-------------------------------|
| LC-1 | The reputation honesty gate (`total_contracts > 0`) referenced in `FLOWS.md` §1 may be applied on a **different** column than the one rendered (the live leaderboard shows zero-completed rows, so either the gate is on `total_contracts` not `completed_contracts`, or it isn't applied at all). The DB schema join (pubkey lives in a separate `account_keys` table) made the exact SQL verification slow. | Couldn't pin the exact gate column from the live query in this pass. | Implementer: read `api/src/database/stats.rs` / the leaderboard query + the offering-card badge source, confirm which column gates the score, then apply the honest-N/A rule consistently. |
| LC-2 | Full-suite wall-clock "~11 min" is the task's stated figure; I verified smoke (29.6s) directly but did **not** run the full suite to time it (would consume the whole review budget). The 30-serial-spec + route-audit analysis explains *why* it's slow, but the exact minute count is unconfirmed here. | Did not run the full suite to clock it. | Run `time npm run test:e2e:fast` once and record the real number before Phase 2. |

