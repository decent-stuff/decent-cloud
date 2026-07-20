# 2026-07-20 — App Health Audit & Radical E2E Harness Overhaul

**Author:** opencode autonomous session
**Scope:** `repo/` (submodule). Outer workspace is wrapper only.
**Confidence:** 9/10 that the plan is correct; per-phase confidence noted inline.

## Goal

1. Finish leftover work from prior sessions (none in-flight found; doc/spec hygiene only).
2. Find & fix every functional + visual issue, **documented first**, then new.
3. Persist open issues as `.md` and sync agent instructions.
4. Radically improve the E2E harness for the Web UI so dev iteration is **seconds** and coverage is **all user flows**.
5. Optimize multi-step UX; codify optimized flows in E2E tests.

The user-facing UI surface is the SvelteKit `website/`. No TUI or desktop app exists; the `cli/` is an argparse CLI (not a TUI) and is out of scope for "UI harness" work.

## Reconnaissance Summary (already complete)

- Local stack running: API `http://localhost:59011` (`/api/v1/health` OK), web `http://localhost:59010` (200 OK). Persistent background pattern documented in compressed context (setsid + separate stderr file).
- `cf/.env.dev` is missing from repo; `scripts/dev-server.sh` hard-fails without it.
- `api/src/main.rs` panics on missing `CANISTER_ID` (rule violation).
- `let _ = ...` audit: only **1 production instance** (`api/src/main.rs:1582` shutdown_tx). The 3 other matches are test-only unused-result suppressions (acceptable).
- Admin grant endpoint exists: `POST /admin/accounts/:username/admin-status` (`api/src/openapi/admin.rs:984`); first admin must be bootstrapped via direct DB UPDATE (chicken-and-egg with admin auth).
- E2E harness: 20 specs, 5498 LOC. **Not in CI** (`Makefile.toml all` excludes `website-e2e`). `reuseExistingServer: false` rebuilds + respawns per invocation. `test-admin-account.ts` shells out to `cargo run --bin api-cli` per worker. 16 dev workers each register a fresh account.
- `scripts/dc-secrets` exists (SOPS-based credential store). Initial recon report of a doc/code mismatch was wrong; no action needed.

## Phases

### Phase 1 — Documented rule violations & boot bugs (HIGH, ~30 min)

Confidence 10/10. Mechanical fixes; each gets its own commit.

- [x] **P1.1** Fix `api/src/main.rs:1582` `let _ = shutdown_tx.send(true)` → match on send result or use `.ok()` with a `tracing::warn!` if the receiver is gone. **Done:** `if let Err(e) = shutdown_tx.send(true) { tracing::debug!(...) }` (debug not warn — the only failure mode is all receivers already dropped, which is the intended shutdown path).
- [x] **P1.2** Fix `api/src/main.rs` CANISTER_ID panic → default to `ggi4a-wyaaa-aaaai-actqq-cai` with `tracing::warn!` when defaulted; LedgerClient::new failure also graceful. **Done.** Verified: server boots without CANISTER_ID env, logs warn, proceeds.
- [x] **P1.3** Add `cf/.env.dev.example` (gitignored `cf/.env.dev` is a local file; example checked in). Updated `scripts/dev-server.sh` to print a helpful "copy from .example" error when `.env.dev` is missing. **Done.**
- [x] **P1.4** ~~Reconcile `scripts/dc-secrets` doc/code mismatch~~. **N/A:** the script exists; initial recon was wrong. No action.

### Phase 2 — Baseline E2E run against live stack (HIGH, ~15 min)

Confidence 10/10. Just measurement.

- [ ] **P2.1** With API + web already running, run `E2E_AUTO_SERVER=0 PLAYWRIGHT_BASE_URL=http://localhost:59010 npx playwright test` against the running stack to capture current pass/fail state.
- [ ] **P2.2** Record results in `OPEN_ISSUES.md` (new file): each failing spec is a bug to triage.

### Phase 3 — Radical E2E harness overhaul (HIGH, ~2-3 h)

Confidence 9/10. The biggest payoff for "dev cycle in seconds".

Guiding principles:
- A test run against an already-running stack must complete in **single-digit seconds** for a smoke set.
- No per-invocation API rebuild. The Rust binary is rebuilt by the developer; the harness reuses it.
- No `cargo run` shell-outs from fixtures. Direct DB writes for admin grants; HTTP for everything else.
- Zero flake from shared worker state: workers don't fight over the same seed data.
- Verify the REAL app: real api-server, real vite, real browser, real Postgres. No mocks of first-party code. Mocks allowed ONLY at external-service boundaries (Stripe, Chatwoot) per user's mock rule.

- [x] **P3.1** `playwright.config.ts`: both `reuseExistingServer` set to `true`; auto-spawned API now runs with `RATE_LIMIT_ENABLED=false`. No `E2E_REUSE_ONLY` env added — `reuseExistingServer: true` already does the right thing (spawn-or-reuse) so a separate mode would be YAGNI. Committed in `d837248b`.
- [x] **P3.2** `test-admin-account.ts` cargo shell-out replaced with `psql UPDATE accounts SET is_admin = TRUE ... RETURNING username` (sub-ms; no toolchain dep). Committed in `d837248b`.
- [x] **P3.1.5** (added) Disable API rate limiter in dev/test. `RATE_LIMIT_ENABLED` env (default: on in production, off elsewhere). Required for parallel workers — without it, 16 workers sharing 127.0.0.1 blew the 120/min RELAXED bucket and produced mass 429s. Committed in `d837248b`.
- [x] **P3.3** ~~Add `scripts/e2e-up.sh` (one-shot launcher).~~ **Folded into `scripts/dev-server.sh` (DRY):** added `--e2e` profile (forces local API, builds binary if missing, sets `RATE_LIMIT_ENABLED=false`), detached setsid launch (survives caller exit; pid file written from inner bash before exec; stderr to separate file per opencode-bash quirk), idempotent start (no-op if `_healthy` returns true), "ready in Xs" timer, process-group stop. Verified: cold start 7s, warm start 0s, smoke 4/4 in 21s, full 109/4-skip in 2.6m.
- [x] **P3.4** Added npm scripts to `website/package.json`: `e2e:up`/`e2e:down`/`e2e:status` (call dev-server.sh), `test:e2e:fast` (`PLAYWRIGHT_BASE_URL=http://localhost:59010 playwright test` — no auto-spawn, reuses warm stack), `test:e2e:fast:smoke` (smoke subset). Workflow: `npm run e2e:up` once, iterate on `npm run test:e2e:fast` afterward.
- [x] **P3.5** Audit existing 20 specs for any non-boundary mock usage; convert to real flow where possible. **Done:** de-mocked 2 first-party-API mocks (first-login-onboarding `external-keys` — fresh accounts return empty keys naturally so the mock was redundant; compare-share `offerings/*` + `prices/icp` — switched to real dev DB offerings IDs 1,2 since the test only asserts URL canonicalization + clipboard). 3 remaining mocks all sit at external-dep boundaries (Stripe SDK via `stripe-mock.ts`, Stripe `verify-checkout` post-payment state in post-rental-welcome, network-failure 500 in registration-flow error-handling test) and are justified per "mocks only at the smallest boundary if external dep".
- [x] **P3.6** Wire E2E into CI. Updated `Makefile.toml` `website-e2e` task from slow `E2E_AUTO_SERVER=1 npx playwright test` (per-invocation cargo run + 120s health-check timeout) to the warm-stack pattern (`dev-server.sh start --e2e` → `npm run test:e2e:fast` → `dev-server.sh stop`, trap-based teardown). Added parallel `website-e2e-smoke` task. Added `e2e` job to `.github/workflows/build-and-test.yml` (parallel to `build-test`, same self-hosted runner, calls `cargo make website-e2e`). Verified locally: smoke 4/4 in 15s + task overhead = 60s total; full 109/4-skip in 2.5m + overhead = 3m20s total.
- [ ] **P3.7** (added) Triage 86 stale-test failures across 17 specs (UI drifted from assertions; e.g. account-page expects removed "Account ID" text, signin-flow expects "Import Existing" without prior "Sign in with seed phrase instead" click). Group by spec family, dispatch parallel subagents to update assertions to match real UI without losing coverage intent. Discovered during Phase 2 baseline re-run after rate-limit fix.

### Phase 4 — Coverage of all user flows (HIGH, ~2 h)

Confidence 9/10. Enumerate flows from `src/routes/` and existing specs, fill gaps.

- [ ] **P4.1** Enumerate ALL user flows from `src/routes/` (top-level + 14 dashboard subroutes). Cross-reference with existing 20 specs. List uncovered flows.
- [ ] **P4.2** Add specs for uncovered flows (per spec: small, focused, one flow). Prioritize flows reachable from sidebar nav: `account`, `admin`, `cloud`, `invoices`, `marketplace`, `offerings`, `provider`, `providers`, `rentals`, `reputation`, `saved`, `transfers`, `user`, `validators`.
- [ ] **P4.3** Migrate `scripts/browser.js`-based UX verification flows (per user instruction) to playwright specs.

### Phase 5 — UX optimization & codification (MEDIUM, ~1-2 h)

Confidence 8/10. Subjective; codify via tests.

- [ ] **P5.1** Identify multi-step user actions (registration → verify-email → onboarding; checkout → payment → confirmation; provider publish flow; rental request → active).
- [ ] **P5.2** Reduce clicks/keystrokes per flow where safe (auto-focus first input, sensible defaults, Enter-to-submit, smart redirects after auth).
- [ ] **P5.3** Codify each optimized flow as an E2E spec that asserts the click count / step count is at or below target.

### Phase 6 — Visual issues & new bug discovery (MEDIUM, ongoing)

- [ ] **P6.1** Visual audit of each route via `scripts/browser.js snap` + `errs` (no JS errors).
- [ ] **P6.2** Triaged bugs filed as either (a) fixed in this session, (b) added to `OPEN_ISSUES.md` for follow-up, or (c) `gh issue create` for larger architectural items per AGENTS.md.

### Phase 7 — Knowledge base sync (LOW, ~30 min)

- [ ] **P7.1** Update `repo/AGENTS.md` (and `repo/CLAUDE.md` symlink) with: working `cf/.env.dev` flow, fast E2E commands, removed `dc-secrets` references.
- [ ] **P7.2** Update `repo/website/AGENTS.md` with the fast E2E pattern.
- [ ] **P7.3** Update `repo/PROMPT.md` (session log for next session) with what was done.
- [ ] **P7.4** Keep `docs/plans/2026-07-20-app-health-and-harness.md` (this file) updated as the source of truth; check off items as completed.

## Operating Rules (per session prompt + repo/AGENTS.md)

- **Autonomous**: don't ask unless 9/10+ confidence is impossible.
- **Subagents** for high-level decisions; **swarm** (parallel subagents) for plan/build/verify where independent.
- **Commit each unit**: every checkbox above is a commit (or part of one logical unit).
- **Dev cycle in seconds**: every choice must serve fast iteration.
- **No silent errors**: `match`/`?` everywhere; no `let _ = ...` for Results.
- **DRY/KISS/YAGNI**: no speculative abstraction, no duplicate code paths.
- **No mocks of first-party code** in tests. Mocks only at the smallest external boundary.
- **TDD**: write failing test → make it pass → keep test.
- **Idempotent**: every script must be safe to re-run.
- **Confidence shown**: each commit message or PR section reports a 1-10 confidence.
- **Alignment verified**: mechanical (build/test/clippy) + human (does it actually fix what the user asked for?). Update `docs/human-expectations.md` if divergent.

## Out of Scope This Session

- Deferred-post-launch GitHub issues (per `repo/PROMPT.md`).
- The `cli/` argparse tool (not a UI surface).
- The `ic-canister/` canister runtime (separate deploy target).
- Building a TUI or desktop app (neither exists; user gave "TUI (alternatively: desktop)" as alternative — both N/A, web is the focus).

## Completion Criteria

- [ ] Every Phase 1-7 checkbox is checked or explicitly descoped with a reason.
- [ ] `npm run test:e2e:fast` runs in <10s for smoke set against pre-started stack.
- [ ] Full E2E suite passes against real api + web + Postgres.
- [ ] `OPEN_ISSUES.md` exists; each open issue is either a GitHub issue or an in-repo note with rationale.
- [ ] `repo/AGENTS.md` reflects reality (no `dc-secrets` references if script absent).
- [ ] Each commit ships with a confidence rating.
