# E2E Coverage Gaps + UX Flow Optimization Audit

**Date:** 2026-07-24 · **Mode:** read-only analysis (no code/test changes, no commits)
**Stack:** warm (web `:59010`, api `:59011`), smoke tier = 19 tests in 12 files
**Sources of truth read in full:** `website/tests/e2e/FLOWS.md`, `docs/OPEN_ISSUES.md`,
plus the fixtures, specs, and `+page.svelte` files cited below. Every "closeable" verdict
below is grounded in real code, not assumed.

---

## TASK 1 — Coverage Gap Closure

### Verdict summary (all ⚠️/❌ rows in FLOWS.md)

| # | Flow (FLOWS.md row) | Status | Verdict | Conf. |
|---|---------------------|--------|---------|-------|
| 1 | Create offering — full submit (L123, ⚠️) | ⚠️ | **CLOSEABLE** (full-suite, serial) | 8/10 |
| 2 | Manage devices — add-device submit (L107, ⚠️) | ⚠️ | **CLOSEABLE** (smoke candidate) | 9/10 |
| 3 | Compare offerings — full multi-offering view (L70, ⚠️) | ⚠️ | **CLOSEABLE** (smoke candidate) | 9/10 |
| 4 | Agent pools — create (L132, ⚠️) | ⚠️ | **CLOSEABLE** (full-suite) | 8/10 |
| 5 | Provider earnings — populated state (L133, ⚠️) | ⚠️ | **CLOSEABLE** (full-suite, serial) | 7/10 |
| 6 | Become provider — onboarding submit (L122, ⚠️) | ⚠️ | **CLOSEABLE** (full-suite) | 7/10 |
| 7 | Password resets (provider) — populated (L136, ⚠️) | ⚠️ | CLOSEABLE, higher effort | 5/10 |
| 8 | Admin actions — account mgmt (L147, ❌) | ❌ | **CLOSEABLE** for account mutations (full-suite) | 8/10 |
| 9 | Verify email — success path (L83, ⚠️) | ⚠️ | NOT closeable (external email) | — |
| 10 | Payment flows — real checkout (L103, ⚠️) | ⚠️ | NOT closeable (external Stripe) | — |
| 11 | Cloud accounts — real connect (L115, ⚠️) | ⚠️ | NOT closeable (external Hetzner/Proxmox) | — |
| 12 | Rent offering in smoke (L97, note) | ⚠️ | Intentional exclusion — leave as-is | — |
| 13 | Admin — Send Test Email (part of L147) | ❌ | NOT closeable (needs MAILCHANNELS_API_KEY) | — |

**Net: 7 closeable gaps (rows 1–6 + 8).** Six can be closed without *any* new external
dependency; all reuse existing fixtures. Rows 9–11/13 genuinely require external services
and are correctly parked.

---

### Closeable gaps — detailed approaches

#### 1. Create offering — full submit (FLOWS.md L123, ⚠️)

**Why the comment "would create a real row" is no longer a blocker:** the
`offering-edit.spec.ts` suite already proves the seed-under-testAccount-pubkey + DB-cleanup
pattern is safe and reliable (`deleteOfferingsByProvider(pubkey)` in `finally`). Creating a
row via the real signed POST is the same shape, just the INSERT direction.

**Verified path:** `validateStep2` in `src/lib/utils/offering-wizard.ts:24-30` returns `null`
when `selectedAccountId` is empty — i.e. the wizard explicitly lets a non-Hetzner provider
advance ("You can still proceed without one", `offerings/create/+page.svelte:572`).
`handleSubmit` (`offerings/create/+page.svelte:182`) then signs `POST /providers/:pubkey/offerings`
with `provisioner_type=undefined` → a valid **manual / no-provisioner offering**. No external
service is touched.

**Approach:**
- Fixture: `test` (test-account). Serial mode (`test.describe.configure({ mode: 'serial' })`).
- `pubkey = pubkeyHexFromSeed(testAccount.seedPhrase)`.
- `goto('/dashboard/offerings/create')`; fill `#offer-name` + blur (auto-derives `#offering-id`);
  click "Next: Infrastructure" → assert step 2; click "Next: Pricing & Recipe" → assert step 3;
  fill `#monthly-price` with `12.50`; click "Create Offering".
- Assert: `expect(page).toHaveURL(/\/dashboard\/offerings\/?$/)` (redirect on success, line 277);
  then `sql("SELECT offer_name FROM provider_offerings WHERE pubkey = decode('${pubkey}','hex')")`
  contains the name.
- Cleanup: `deleteOfferingsByProvider(pubkey)` in `finally`.

**Tier:** full-suite only (signed POST + serial pubkey sharing). **Confidence: 8/10.**
Caveat: assumes the API accepts an offering with `provisioner_type=undefined`; the page
explicitly allows it, so this is very likely, but the handler should be confirmed on first run.

#### 2. Add device — submit (FLOWS.md L107, ⚠️)

**Verified path:** `AccountOverview.svelte:204` renders "+ Add Device" → opens
`AddDeviceModal.svelte`. The modal's `handleAddDevice` (line 35) calls `addAccountKey` — a
**signed API INSERT into `account_public_keys`** (`AddDeviceModal.svelte:44`). The "Generate
New" path generates a seed → derives pubkey → links it. Pure DB mutation; cleanup cascades
when the testAccount is deleted at fixture teardown (`test-account.ts:38`).

**Approach:**
- Fixture: `test`. `goto('/dashboard/account/security')`.
- Assert initial `text=1 key` (existing assertion in `account-page.spec.ts:208`).
- Click "+ Add Device" → modal → click "Generate New" → walk the seed reveal → optionally name
  the device → confirm link.
- Assert: `text=2 keys` visible, and the "Device Added!" success step
  (`AddDeviceModal.svelte:108`).
- Cleanup: automatic (account cascade). No extra helper needed.

**Tier:** **smoke candidate** (<5 s, low seed — the only DB write is the key INSERT that
cascades). **Confidence: 9/10.** The only friction is driving the `SeedPhraseStep` sub-flow,
which is already exercised by `registration-flow.spec.ts`, so selectors are known.

#### 3. Compare offerings — full multi-offering view (FLOWS.md L70, ⚠️)

**Verified path:** `marketplace/compare/+page.svelte:48` fetches each offering via
`getOffering(id)` and renders a side-by-side table with per-row "best value" `✓` winners
(`winners` derived at line 172, rendered e.g. at line 295). The current `compare-share.spec.ts`
only asserts URL canonicalization + clipboard — it never proves offerings actually load or the
table renders. Dev-DB demo offerings (IDs 1,2) are fragile (offline/hidden — OPEN_ISSUES H6).

**Approach (dev-DB-independent):**
- Seed two offerings under fresh random provider pubkeys with **different** prices so the price
  winner is deterministic:
  `seedRentableOffering({ name: 'Cheap A' })` and `seedRentableOffering({ name: 'Pricey B' })`
  — but set distinct monthly prices by extending the override or seeding via `seedOffering` with
  a custom price column. (Note: `seedOffering` hardcodes `monthly_price=25.0`; to make the winner
  deterministic either add a `monthlyPrice` override to `seedOffering` or assert only on
  rendering, not the winner.)
- `goto('/dashboard/marketplace/compare?ids=A,B')`.
- Assert: the comparison `<table>` renders, both offering names appear as links
  (`compare/+page.svelte:257`), the "Pricing" / "Compute" section headers are present.
- Cleanup: `deleteOfferingsByProvider(pubkeyA)` + `deleteOfferingsByProvider(pubkeyB)`.

**Tier:** **smoke candidate** (<5 s, two self-contained rows, no shared pubkey → no serial
mode needed). **Confidence: 9/10.** Cleanest of the partial gaps.

#### 4. Agent pools — create (FLOWS.md L132, ⚠️)

**Verified path (KEY):** the `agent_pools` table has `provider_pubkey … REFERENCES
provider_registrations(pubkey)` (`001_schema.sql:666`), so pool create *looks* like it needs a
registered provider first. **But** `Database::create_agent_pool` auto-creates the
`provider_registrations` row on demand:
```rust
// agent_pools.rs:205
INSERT INTO provider_registrations (pubkey, signature, created_at_ns)
  VALUES ($1, $1, $2) ON CONFLICT (pubkey) DO NOTHING
```
So the testAccount can create a pool with **zero pre-seeding**. The handler
`create_pool` (`openapi/providers.rs:3923`) only does `check_authorization` (pubkey == signer) —
no onboarding gate.

**Approach:**
- Fixture: `test`. `goto('/dashboard/provider/agents')`; wait for "+ New Pool" (existing
  assertion, `provider-pages-smoke.spec.ts:80`).
- Click "+ New Pool"; fill `#poolName` with `e2e-pool-<tag>`; leave defaults (location=europe,
  provisioner=proxmox); click "Create Pool".
- Assert: the success banner `Pool "e2e-pool-…" created` (`agents/+page.svelte:182`), and the
  pool appears in the `AgentPoolTable`.
- Cleanup: `sql("DELETE FROM agent_pools WHERE provider_pubkey = decode('${pubkey}','hex')")` +
  `DELETE FROM provider_registrations WHERE pubkey = decode('${pubkey}','hex')` (the
  auto-created row; not cascaded by account delete since pubkey is bytea, not an accounts FK).

**Tier:** full-suite (signed POST + explicit cleanup). **Confidence: 8/10.** Needs a small
new cleanup helper (delete-by-provider for agent_pools + provider_registrations).

#### 5. Provider earnings — populated state (FLOWS.md L133, ⚠️)

**Verified path:** `get_provider_stats` (`database/stats.rs:153`) computes:
- `total_revenue_e9s = SUM(payment_amount_e9s) FROM contract_sign_requests WHERE provider_pubkey = $1`
- `total_contracts` / `pending_contracts` / `offerings_count` from the same key.

The earnings page renders these in the "Revenue Overview" cards (`earnings/+page.svelte:233`)
and the contract table comes from `getUserActivity(pubkey).rentals_as_provider` (line 194).
**So seeding contracts where `provider_pubkey = testAccount pubkey` populates the real page.**

**Approach:**
- Seed 1–2 contracts via `seedContract({ requesterPubkeyHex: <random>, providerPubkeyHex: <testAccount pubkey>, paymentStatus: 'succeeded', paymentAmountE9s: 2_000_000_000, status: 'active' })`
  + optionally `seedOffering(testAccountPubkey, …)` for `offerings_count > 0`.
- `goto('/dashboard/provider/earnings')`; assert "Gross Revenue" shows a non-zero value
  (e.g. `2.00 ICP`, `earnings/+page.svelte:237`), "Total Contracts" > 0, and the contract
  table has rows (`sortedContracts.length > 0`).
- Cleanup: needs a **delete-by-provider** variant (the existing `deleteContractsForRequester`
  keys on requester). Inline SQL mirroring it: delete child tables
  (`contract_events`, `contract_usage*`, `contract_health_checks`, `invoices`) then
  `contract_sign_requests WHERE provider_pubkey = decode(…)`, plus `deleteOfferingsByProvider`.

**Tier:** full-suite, serial (shared testAccount pubkey). **Confidence: 7/10.** Main effort is
the delete-by-provider cleanup helper; the read path is certain from `stats.rs`.

#### 6. Become provider — onboarding submit (FLOWS.md L122, ⚠️)

**Verified path:** `/dashboard/provider/support/+page.svelte` has `saveOnboarding` (form
`onsubmit`, line 1148) that signs `POST /api/v1/providers/:pubkey/onboarding` (line 399) with a
`ProviderOnboarding` payload — a pure DB upsert, no external service. The existing smoke test
stops at the step-1/step-2 render.

**Approach:** drive the wizard form to its submit, click save, assert `onboardingCompleted`
flips (the page re-fetches and the banner changes). Cleanup: delete the provider_onboarding row
for the pubkey (or it's acceptable residue that the account teardown doesn't cascade — verify
the table name before relying on cascade).

**Tier:** full-suite. **Confidence: 7/10.** Uncertainty: the wizard form's required fields and
whether any step needs a cloud account (the create-offering no-account path suggests not, but
the support wizard should be traced field-by-field before writing the test).

#### 7. Password resets (provider) — populated (FLOWS.md L136, ⚠️)

**Verified path:** data comes from `/api/v1/providers/:pubkey/contracts/pending-password-reset`
(`password-resets/+page.svelte:109`) — i.e. contracts owned by the provider that have a
password-reset-pending flag. So this is the *same fixture* as earnings-populated (contracts
where provider = testAccount) **plus** the contract's password-reset-pending column set.

**Tier:** full-suite, serial. **Confidence: 5/10.** Higher effort: needs the exact contract
column/flag that marks "pending password reset" confirmed against the schema + the API query.
If that flag is a single column flip, it's a cheap add-on to the earnings fixture; if it's a
separate state machine, scope it separately.

#### 8. Admin account-management mutations (FLOWS.md L147, ❌ → ✅ partial)

**Verified path:** the admin page (`admin/+page.svelte`) drives these through real signed API
calls in `admin-api.ts`, all of which are **pure DB mutations** with no external dependency:

| Action | Handler | Backend effect |
|--------|---------|----------------|
| Toggle admin | `setAdminStatus` → `POST /admin/accounts/:u/admin-status` (L261) | flips `accounts.is_admin` |
| Set email-verified | `setEmailVerified` → `POST /admin/accounts/:u/email-verified` (L178) | flips `email_verified` |
| Set account email | `setAccountEmail` → `POST /admin/accounts/:u/email` (L194) | updates `email` |
| Delete account | `deleteAccount` → `DELETE /admin/accounts/:u` (L221) | cascade-delete + nullify contracts |

**Approach (safest two first — toggleAdmin + setEmailVerified):**
- Fixtures: `adminTest` (admin-account fixture, `test-admin-account.ts`) + a second seeded
  target account via `seedAccountDirect()`.
- `adminTest` `goto('/dashboard/admin')`; find the target row in "All Accounts"
  (`admin/+page.svelte:532`); click "Make Admin"; assert the role cell flips to "Admin" and
  `sql("SELECT is_admin FROM accounts WHERE username=…")` is `t`.
- Repeat for email-verified toggle.
- `deleteAccount` is also closeable but destructive — seed a throwaway account, admin deletes
  it, assert the `deletionResult` summary renders (L499) and the row disappears. Use it as a
  *separate* test so a failure can't pollute the toggle tests.
- Cleanup: `deleteAccountByUsername(target)` in `finally` (survives the admin-toggle test;
  for the delete-test itself there's nothing to clean).

**Tier:** full-suite (admin fixture + serial-friendly). **Confidence: 8/10.** This closes the
only ❌ in the catalog. **Do not** attempt "Send Test Email" here — see below.

---

### Gaps that genuinely CANNOT close (confirmed against code)

- **Verify email — success path (L83):** the verify token is generated server-side and
  delivered via email. Without real email delivery there's no way to obtain a valid token
  through the real flow. (DB-seeding a token and hitting the verify endpoint would bypass the
  email delivery — the very thing under test — so it's not a meaningful e2e.) Parked correctly.
- **Payment flows — real checkout (L103):** completing a Stripe checkout session → webhook →
  contract activation is the external boundary. `stripe-mock.ts` only mocks the client SDK; the
  server-side webhook/signature path can't be satisfied in-harness. UI-option rendering is the
  maximal honest coverage. Parked.
- **Cloud accounts — real connect (L115):** the connect form validates upstream Hetzner/Proxmox
  API credentials (external). Modal-render coverage is maximal. Parked.
- **Admin Send Test Email (L147 sub-item):** the email send requires `MAILCHANNELS_API_KEY`
  (see `support_bot/test_notifications.rs:68` pattern: `Email service not configured`). The
  success *result message* requires real delivery; only the queue insert is local, and asserting
  "sent" would be dishonest without delivery. Park.
- **Rent offering in smoke (L97):** this is a coverage *note*, not a gap — the flow IS covered
  in the full suite. Smoke exclusion is correct per the <5 s / low-seed rules (the rent-via-dialog
  flow is 6 s + complex seeding). Leave as-is.

---

## TASK 2 — UX Flow Optimization

### Existing shortcut baseline (verified, not assumed)

Tested in `keyboard-shortcuts.spec.ts` and rendered by the dashboard layout:
- `/` → focus marketplace search (`#marketplace-search`)
- `Cmd/Ctrl+K` → command palette (`CommandPalette.svelte`)
- `?` → keyboard help overlay (`KeyboardHelpOverlay.svelte`, `data-testid="keyboard-help"`)
- `Esc` → close dialogs/overlays

> **Note:** `OPEN_ISSUES.md` (L184) still lists "No `?` keyboard-shortcut help overlay" as a
> nice-to-have. **This is stale** — the overlay exists and is covered by 3 tests
> (`keyboard-shortcuts.spec.ts:42-86`). The doc should be updated.

The command palette's `NAV_ITEMS` (`CommandPalette.svelte:57`) contains only **4 items** —
Marketplace, My Rentals, Invoices, Account. **No provider-facing actions.**

---

### Top 3 optimizations

#### UX #1 — Add provider quick-actions to the command palette  *(highest impact)*

**Flow today:** reach "Create Offering" = sidebar → Offerings → "Create Offering" button
(2 navigations + a click) — and you must already be on a dashboard page to see the sidebar.
From the marketplace it's worse.

**Proposal:** extend `NAV_ITEMS` in `CommandPalette.svelte:57` with provider actions, gated on
`isAuthenticated` (already tracked at line 49):
- "Create Offering" → `/dashboard/offerings/create`
- "My Offerings" → `/dashboard/offerings`
- "Agent Pools" → `/dashboard/provider/agents`
- "Billing Settings" → `/dashboard/account/billing`

Result: from **any** page, `Cmd+K` → type "cr" → Enter lands on the create wizard. That's the
single most common provider action reduced to a 3-keystroke global shortcut with no mouse and
no sidebar dependency. The palette already does nav (`selectItem` → `goto`), so this is a
data-only change (~8 lines).

**Codifiable as e2e:** yes — assert the palette lists "Create Offering" and that selecting it
navigates to `/dashboard/offerings/create`. New test in a `command-palette.spec.ts`.
**Confidence: 9/10.**

#### UX #2 — Replace native `confirm()` on offering delete with the inline two-step pattern

**Flow today:** `offerings/+page.svelte:179` calls `confirm("Delete offering '…'?")` — a
blocking native browser dialog. The same pattern is at `AccountOverview.svelte:102` for device
removal. Native `confirm()` is unstyleable, jarring on mobile, blocks test automation
(Playwright silently auto-accepts, masking regressions), and is inconsistent with the rest of
the app.

**Proposal:** adopt the **inline two-step confirm** already proven in two places:
- API token revoke (`security/+page.svelte:511-527`: first click shows "Confirm revoke? Yes / Cancel", second click acts)
- Admin delete account (`admin/+page.svelte:679-714`: type-username confirm)

For offering delete: first "Delete" click → the footer swaps to "Delete this offering? ✓
Confirm / ✕ Cancel"; second click fires the signed DELETE. Low-stakes offerings (no active
contracts) could even skip the second step, but the inline pattern alone is the win.

**Codifiable as e2e:** yes — the inline pattern is deterministic and already tested for token
revoke. Replaces a flaky auto-accepted `confirm()` with a real two-click assertion.
**Confidence: 8/10.** (UX judgment call on whether to keep the confirm for offerings with
active contracts — recommend keeping it there, dropping for zero-contract offerings.)

#### UX #3 — Auto-suggest monthly price from the selected Hetzner server cost in the create wizard

**Flow today:** in `offerings/create/+page.svelte`, step 2 shows the Hetzner cost when a server
type is picked (`selectedServerType.priceMonthly`, line 686), and step 3 echoes it as a hint
(line 740) — **but the `#monthly-price` input stays empty** (`bind:value={monthlyPrice}`,
placeholder `"0.00"`). The provider must manually transcribe/translate the Hetzner cost into a
sale price, and `validateStep3` (`offering-wizard.ts:33`) blocks submit until they do.

**Proposal:** when a server type is selected and `monthlyPrice` is still null, pre-fill
`#monthly-price` with a sensible default derived from the cost (e.g. `cost × 1.5`, rounded).
The provider still adjusts it, but the common case (provider accepts the suggested margin)
becomes zero keystrokes. The hint already shows the cost basis, so the suggestion is
transparent.

**Codifiable as e2e:** yes — assert that selecting a server type in step 2 (against a seeded
catalog, or via the no-account path by mocking the catalog fetch) pre-fills a non-empty,
non-zero `#monthly-price`. **Confidence: 7/10** (depends on whether the team wants a margin
default baked in — a product call; the test is straightforward either way).

---

### Secondary candidates (lower priority, still concrete)

- **`g`-prefixed vim-style navigation** (e.g. `g o` → Offerings, `g r` → Rentals) for keyboard
  power users. The `?` overlay already documents shortcuts, so discoverability is solved. Lower
  ROI than palette actions (UX #1) which cover the same ground more discoverably.
- **Add-device quick-path:** `AddDeviceModal` walks the full seed-phrase reveal/copy flow even
  for "add *this* browser". A one-step "Generate & link as 'Device <date>'" that shows the seed
  once would cut ~4 clicks, but this is security-sensitive (seed must be surfaced) — needs
  product sign-off. **Confidence: 5/10.**
- **Create-offering: remember last-used visibility/currency** across the session so a provider
  creating multiple offerings doesn't re-set 'private'→'public' each time. Small `localStorage`
  persistence. Codifiable. **Confidence: 7/10.**

---

## Appendix — evidence commands run

- `bash scripts/dev-server.sh status` → api + web both healthy.
- `npx playwright test --list --grep @smoke` → **19 tests / 12 files** (FLOWS.md says 17;
  slightly stale).
- Read in full: `FLOWS.md`, `OPEN_ISSUES.md`, all 4 fixtures (`seed-helpers.ts`,
  `test-account.ts`, `test-admin-account.ts`, `stripe-mock.ts`, `auth-helpers.ts`), the 6
  relevant specs, 7 `+page.svelte` routes, `offering-wizard.ts`, `agent_pools.rs`,
  `stats.rs:153`, `001_schema.sql:664`, `admin-api.ts`, `test_notifications.rs`.
