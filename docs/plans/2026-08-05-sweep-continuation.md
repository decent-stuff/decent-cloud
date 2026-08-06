# 2026-08-05 — Sweep Continuation (UX audit / #444 splits / robustness / e2e)

## STATUS: DONE (2026-08-06)

All waves shipped + implementer fixes. 8 commits on branch `sweep-2026-08-05` (all gates green):

- `70075b05` — smoke stabilize (Playwright empty-state deterministic waits + 10s expect ceiling)
- `352f2355` — #444 split: contracts.rs 2244→1745, new contract_telemetry.rs 527L, tuple1 13→14
- `a1f1a2f0` — #444 docs (wave record)
- `e64b37a8` — dc-agent: docker image-pull 600s overall timeout (was unbounded — could wedge contract lock) + gateway iptables silent errors surfaced via match+warn! (3 sites)
- `f6c37458` — real-deploy harness: marketplace honesty assertions (FAIL-on-prod if catalog all-demo or zero-rentable) + new flows console-errors/drift/stats-honesty
- `3b372ae9` — api: stats honesty (active_providers now reads LIVE provider_agent_status not retired ICP provider_check_ins; total_offerings aligned to the same marketplace pool rule; SyncService::new→Result not panic; new doctor guard DOCTOR_EXAMPLE_OFFERINGS_PRESENT)
- `eb3f8ba5` — e2e docs: rewrote stale README (318→~95 lines, correct ports), fixed FLOWS category-run section (tags are doc-only), pruned 2 low-signal test sets
- `c32177f3` — frontend: removed zombie demo UI code (showDemoOfferings/?demo=/Demo-only badges), rent dialog OS defaults to offering's first OS, trending strip excludes known-offline offerings, first-visit dashboard greeting

### Wave completion
- [x] **Wave A — No-mock UX audit** → findings reported (F1-F6); implementer fixes shipped in `c32177f3` + `3b372ae9`.
- [x] **Wave B — #444 large-file split** → `352f2355` (contracts.rs split) + `a1f1a2f0` (wave docs).
- [x] **Wave C — Rust robustness sweep** → `e64b37a8` (dc-agent image-pull timeout + iptables error surfacing) + `3b372ae9` (SyncService::new → Result, no expect-panic).
- [x] **Wave D — E2e harness audit** → `eb3f8ba5` (README rewrite, FLOWS category-run fix, prune low-signal test sets) + `70075b05` (smoke stabilize under parallel load).
- [x] **A2 — Real-deploy audit** → `f6c37458` (marketplace-honesty harness + new flows) + `3b372ae9` (stats-honesty backend + doctor guard).
- [x] **Implementer fixes (Wave-A findings)** → `c32177f3` (zombie demo UI, rent-OS default, trending offline filter, first-visit greeting) + `3b372ae9` (stats active_providers + total_offerings honesty).

### Verification (all green)
- api release build clean + restart health 200
- `/api/v1/stats` E2E honesty confirmed live (active_providers=1=online agents; total_offerings=3=marketplace list)
- dc-agent clippy 0 + 246 tests
- frontend svelte-check 0/0 + vitest 860/860
- Playwright smoke 26/26 (28s)

### Operator blockers still open
Need deploy/operator action — recorded in detail in `docs/OPEN_ISSUES.md` → "Operator / deploy blockers (need human)":
- **OP-1 (CRITICAL)** Prod marketplace still serves 10 synthetic demo offerings — migration `053_drop_example_provider_seed.sql` never applied to the prod DB (prod never redeployed). Autonomous guard now in place: doctor `DOCTOR_EXAMPLE_OFFERINGS_PRESENT` + harness marketplace-honesty both FAIL-on-prod.
- **OP-2** Stage (dev-api) is stale — `/auth/capabilities` 404; offerings priced in retired ICP currency.
- **OP-3** PROD Chatwoot support widget 404s on every page (`X-Frame-Options:SAMEORIGIN` blocks the iframe).
- **OP-4** `stage-*` hostnames do not resolve (k8s dc-stage public cutover incomplete; `dev-*` is the de-facto stage).

## Context
- 2026-08-03 sweep is DONE (PR opened, all gates green). No in-flight unfinished work.
- Standing mandate: "big sweep" — fresh no-mock UX audit, continued #444 splits, robustness, e2e harness radicalization.
- BLOCKED (do NOT attempt): k8s cutover (operator), Hetzner first-offerings (operator creds), #447 refund replay (operator), Decent-Agents cluster #418/#415/#416/#427 (spec-only multi-PR epic with forks + needs real Hetzner host), F6 leaderboard (needs real offerings).

## Baseline (verify before any work)
- [x] `cargo build` (api-server) compiles — release build clean
- [x] `cargo clippy` 0 warnings — dc-agent clippy 0
- [x] `npx playwright test --grep @smoke` 26/26 (28s)
- [x] `vitest run` (860/860) + `svelte-check` clean (0/0)

## Waves (parallel subagents, each writes a FINDINGS report; auditor fixes nothing)

### Wave A — No-mock UX audit (REPORT ONLY, no commits)
Fresh new-user + returning-user lens against the REAL warm stack (web:59010, api:59011). Use chrome-cli screenshots + zai-vision + Plasmate SOM. Per FLOWS.md coverage, DO NOT re-test the 26 @smoke scenarios. Focus on:
- Marketplace empty state (honest-empty post-demo-removal): is it clear what to do next?
- Provider onboarding flow (`/dashboard/provider/start`) — all steps load within seconds, no spinners stuck, no AI slop/stubs.
- Rent flow (non-smoke portions): full dialog suite, payment, lease lifecycle.
- Keyboard accessibility for common paths (tab/enter/esc coverage), shortcut discoverability.
- Visual consistency, dead elements, placeholders that never resolve, inconsistent copy.
- Any element that fails to load within a few seconds = BUG.

### Wave B — #444 large-file split (ship ONE high-confidence split)
Candidates (prod source, >2000L): `api/src/openapi/providers.rs` (4090), `dc-agent/src/main.rs` (3674), `api/src/database/offerings.rs` (2876), `api/src/database/cloud_resources.rs` (2445), `api/src/openapi/contracts.rs` (2244), `api/src/openapi/accounts.rs` (2230, exhausted). Pick the highest-confidence mechanical split; verify byte-identical OpenAPI via `spec_snapshot` guard; keep `create_combined_api()` tuple wiring valid.

### Wave C — Rust robustness sweep (ship ≥6/10 fixes only)
- Missing I/O timeouts (network + local) on non-shared-client paths.
- `.unwrap()`/`.expect()` on non-infallible paths → propagate or log with context.
- Swallowed `Result`s in `.ok()` chains → `match` with contextual `error!`/`warn!`.
- Dead/zombie code, duplicated constants (single source of truth), config drift.
- `let _ =` already clean (prior session). Re-verify.

### Wave D — E2e harness audit (ship improvements ≥6/10)
- Coverage gaps vs FLOWS.md routes; the 2 ⚠️ partial rows (rent full-suite; password-resets empty-state).
- Speed: target <35s for @smoke; reduce setup/teardown overhead; fold overlapping tests.
- Make tested-flow set trivially listable + new-flow addition cheap.
- No mocks in first-party paths; only sanctioned external-boundary mocks.

## Triage + ship
- Merge Wave A findings → fresh implementer subagents ship fixes (TDD RED→GREEN where possible), confidence ≥6/10 only.
- Each unit committed on a branch; open PR at end.

## Close-out
- [x] Update `docs/OPEN_ISSUES.md` (close resolved, add new findings, re-confirm deferrals).
- [x] Update `repo/AGENTS.md` + this plan with status.
- [x] Surface blockers/forks to user (Decent-Agents epic, Hetzner, k8s cutover) — recorded as OP-1..OP-4 in `docs/OPEN_ISSUES.md`.

## Status: DONE (2026-08-06) — see STATUS block at the top of this file.

## Round 2/3 (user feedback + CI/release, 2026-08-06)

Follow-on sweep on the same branch, driven by user feedback (Hetzner-requires-a-pool bug,
provider-sidebar defaults) + CI/release verification. 7 commits on top of round 1
(origin/main `252c7f76` → `45953812`); spec_snapshot unchanged (187 paths / 327 schemas); full
Playwright suite **314/0**.

- `28740f20` chore: workspace version 0.5.3→0.5.5 (release-tag consistency).
- `9e16e677` refactor: remove dead `is_example` concept entirely (field/SQL projection/`$N` param
  threading/frontend serialization) + remove the now-obsolete `DOCTOR_EXAMPLE_OFFERINGS_PRESENT`
  doctor guard (`is_example` was derived, never a column, always false since migration 053;
  `example_provider_pubkey()` retained — 2 live endpoints + a fresh-DB guard still use it).
- `2978b0ad` fix(website): Chatwoot widget env-gate (renders only when both `websiteToken`+`baseUrl`
  set); removed hardcoded dead `support.decent-cloud.org` default; `release.yml`+`cf/deploy.py` wire
  `VITE_CHATWOOT_*` build vars.
- `89b6dbb2` fix(ci): repaired 2 demo-removal Playwright regressions; removed empty `DC_REPO_WRITE`
  checkout override; committed Cargo.lock for `--locked`.
- `87c45517` fix(website): provider sidebar OPEN by default + added Cloud Accounts nav link.
- `a2a96862` fix(offerings): cloud-resell (Hetzner/Vultr) offerings visible in marketplace without a
  pool — DRY `is_cloud_resell`/`is_marketplace_visible` helpers (BackendType SSOT). The user's
  "Hetzner requires a pool" bug.
- `45953812` fix(e2e): root-caused 2 PRE-EXISTING Playwright failures (account-page ambiguous `Account`
  selector → exact match; offerings-editor-replace depended on example-provider templates dropped by
  053 → self-seeds). Full suite now 314/0.

**k8s manifest audit (read-only, no manifest changes):** migrations auto-run unconditionally at boot
(`database/core.rs:15`); website Chatwoot config is build-time-baked (no runtime env needed); stage
overlay correct + isolated; probes adequate; image-tag policy sound. One flagged non-action
(`CHATWOOT_INBOX_ID` env unread by code → wire-or-drop) surfaced as OP-5 in `docs/OPEN_ISSUES.md`.
A full prod+stage deploy runbook is recorded inline in the OP-1 entry of `docs/OPEN_ISSUES.md`
(retag → `release.yml` builds image → ArgoCD syncs → migration `053` applies at boot). The cutover
sequence itself is cross-referenced in `docs/MIGRATION-CUTOVER.md` (not duplicated here).

The standing mandate (no-mock UX audit, continued #444 splits, robustness, e2e harness
radicalization) continues.
