# 2026-07-20 Session Complete — Next Session Prompt

## What shipped this session

App-health audit + radical E2E harness overhaul. See `docs/plans/2026-07-20-app-health-and-harness.md`
for the full plan and per-phase confidence ratings. Headline numbers:

- **E2E went from 24/114 pass (2m46s) → 135/139 pass + 4 skipped (2m42s)** against a warm stack.
- **Smoke subset: 4/4 in ~20 s** against a warm stack (was minutes per invocation).
- **E2E wired into CI** (`.github/workflows/build-and-test.yml` `e2e` job, `Makefile.toml`
  `website-e2e` + `website-e2e-smoke` tasks).
- **5 real product bugs fixed**: CANISTER_ID panic on boot, `let _ = shutdown_tx.send`, offering
  detail SLA null-guard (`latestUptimePercent`), `/dashboard/saved` 404 during auth bootstrap,
  inverted light-theme body color (invisible 404).
- **3 UX optimizations**: registration lands on `/dashboard` so WelcomeModal fires, Save promoted
  to a visible bookmark toggle (2 clicks → 1), mobile search placeholder shortened.
- **22 new E2E tests** covering 4 previously-uncovered routes (rentals, invoices, transfers, saved).
- **6 GitHub issues filed**, 1 closed as false alarm. See `docs/OPEN_ISSUES.md`.

## Operating posture (unchanged)

- Autonomous. Subagents for high-level decisions; swarm for parallel plan/build/verify.
- Plan file `docs/plans/YYYY-MM-DD-<slug>.md` (extend the existing one if same day).
- Commit each unit. Dev cycle in SECONDS against a warm stack (use `npm run test:e2e:fast`).
- No silent errors (`match`/`?`, never `let _ = ...`). DRY/KISS/YAGNI. No backward-compat.
- Mocks only at the smallest external-dep boundary in tests; never mock first-party code.
- TDD: RED → GREEN → keep test. Every path needs +/− meaningful tests.
- Confidence 1-10 shown for changes. Verify alignment (mechanical + human).

## Where to start next session

1. **Read `docs/OPEN_ISSUES.md`** for the categorized open-issue inventory (in scope vs deferred).
   GitHub Issues at `decent-stuff/decent-cloud` is the canonical source — re-sync with
   `gh issue list --repo decent-stuff/decent-cloud --state open`.

2. **In-scope issues** (NOT `deferred-post-launch`): highest impact first
   - #433 No UI to top up account balance [launch] — found during this session's UX audit.
   - #418 Decent Agents: beta onboarding [launch].
   - #427 Anthropic API key proxy/sidecar [decent-agents, launch].
   - #416 DA usage metering + customer dashboard [decent-agents].
   - #415 DA subscription billing [decent-agents].
   - #410 Stripe: cleanup stale pending contracts [stripe].

3. **Bring up the warm stack first** — `bash scripts/dev-server.sh start --e2e` from the repo root.
   Then iterate with `cd website && npm run test:e2e:fast` (see `AGENTS.md` for the full workflow).

4. **Coverage gaps remaining** (lower priority — info/admin pages): `/dashboard/cloud`,
   `/dashboard/providers`, `/dashboard/user`, top-level `/agents`, `/checkout`,
   `/dashboard/reputation`, `/dashboard/validators` (only visibility checks).

5. **In-repo known issue** (not on GitHub): marketplace empty-state hint suggests
   `product_type:gpu` field syntax but the API rejects it (`Unknown field: product_type`).
   See `docs/OPEN_ISSUES.md` → "In-repo known issues".
