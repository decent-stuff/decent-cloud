# Plan: 2026-08-03 — Fresh e2e/UX/tech-debt sweep

**Created:** 2026-08-03. **Baseline:** `origin/main 31483130` (last merged commit incl. 2026-08-02 Wave 11; the earlier-stated `d3378074` was a local migration-branch HEAD, NOT the clean baseline). Verified green at session start: smoke 26/26 (slow, see Finding B), warm stack healthy. This sweep runs on dedicated branch **`sweep-e2e-ux-techdebt`** (reset from origin/main + cherry-picked this plan doc), isolated from k8s PR #454.

## Context

- The 2026-08-02 mandate (e2e radicalization + UX slop + #444 Wave 9/10/11 + auth single-source) is
  COMPLETE and committed. Baseline re-verified green at session start.
- **k8s consolidation (`2026-08-03-staging-to-k8s-dc-stage-consolidation.md`) is DONE — autonomous
  portion.** Tracks 1+2+3 complete: nuc-k3s base/prod/stage split (byte-identical prod), dc-stage
  LIVE on cluster (`/api/v1/health` HTTP 200, 52 migrations/86 tables, prod untouched), product-repo
  Phase 2/3 in PR #454. Only **operator cutover** remains (push nuc-k3s, persist stage DB pw to
  SOPS, ship `:stage` image, repoint tunnel, tear down dev host) — documented in
  `docs/MIGRATION-CUTOVER.md`. No longer a blocker for this sweep.
- `gh` is unauthenticated in this environment → GitHub Issues fall back to the in-repo inventory at
  `docs/OPEN_ISSUES.md` (AGENTS.md notes this limitation). Product-repo pushes use the
  `GITHUB_TEST_PAT` (user `andris-k85`) over HTTPS; main is branch-protected → open a PR.

## Baseline findings (recorded 2026-08-03)

- **Finding A (reliability):** `invoices.spec.ts @smoke empty state` FAILED under full parallel
  smoke (25/26) but PASSES in isolation — flaky under parallel load (timing timeout at
  invoices.spec.ts:35:82), NOT a real regression (snapshot confirmed the empty state rendered
  correctly). §4 to harden.
- **Finding B (speed):** smoke 26/26 green but **80s wall vs the <35s target** (FLOWS.md says
  2026-08-02 achieved ~27s). Individual slow tests: signin 9.9s, rentals-empty 6.5s, transfers 4.8s.
  Vite HMR `[vite] connecting...connected` debug lines fire between every test (per-context browser
  launch overhead). §4 to root-cause + fix.

## Scope (this session — fully autonomous, local, no external creds needed)

Standing mandate continued: fix documented issues → radical e2e harness → no-mock UX audit →
tech-debt/robustness closure → persist open issues.

### §1 Real-app UX audit (PRIMARY, no mocks) — WAVE-B subagent

Drive the REAL warm stack (api:59011 + web:59010) via chrome-cli screenshots + zai-vision; Plasmate
for DOM. New-user + returning-user lens. Look for: AI slop/templates, spinners/placeholders that
don't resolve in seconds (= BUG), multi-step flows that could be 1 step, missing/intuitive keyboard
shortcuts, visual inconsistencies, dead elements. Each finding: confidence 1-10 + safe 1-10; only
ship ≥6/10. NO commits from the auditor — it REPORTS; I triage + ship (or spawn a fresh implementer).
Screenshots NOT committed.

### §2 #444 large-file split — one HIGH-confidence split — WAVE-D subagent

Investigate `api/src/openapi/webhooks.rs` (2504L) for a clean `#[OpenApi]`/handler cluster with a
free tuple slot (verify `create_combined_api()` arity; tuple 2 had 4 free slots after the 2026-07-26
rebalance). If a clean cluster exists, extract it (byte-identical OpenAPI via spare-port diff; clippy
0; nextest green). If webhooks.rs has no clean boundary, fall back to the next-best candidate from the
OPEN_ISSUES largest-file list. ONE split only (highest confidence). accounts.rs is exhausted.

### §3 Code-robustness / silent-error sweep — WAVE-C subagent

`let _ =` is already clean (only legit child.kill/wait + 1 test). Target the OTHER categories:
missing I/O timeouts (network/local), `.unwrap()`/`.expect()` on non-infallible paths, swallowed
`Result`s in `.ok()` chains (cf. the 2026-08-02 `StripeClient::new().ok()` pattern), anti-patterns
(seeding invalid data + runtime detection), dead/zombie code, config drift / duplicated constants.
Each finding: confidence + safe score; ship ≥6/10 only; commit per unit. NO mocks in prod code.

### §4 E2e coverage + speed reconciliation — folded into WAVE-A (my own pass after §1-3)

Reconcile FLOWS.md vs `website/src/routes/` + CLI commands for undocumented flows. Confirm `npx
playwright test --list` makes the tested set trivially listable. Smoke target stays <30s. Add tests
ONLY for real gaps surfaced by §1's UX audit (each new flow codified as a fast e2e).

### §5 Persist + reconcile open issues

Update `docs/OPEN_ISSUES.md`: record this session; reconcile any stale rows surfaced by the sweeps;
link from AGENTS.md is already present.

## Execution (subagent orchestration; preserve main context)

- **WAVE-B** (UX audit): one subagent, drives the real app, REPORTS findings (no commits).
- **WAVE-D** (#444 split): one implementer, TDD/byte-identical, commits per unit, `timeout` on cmds.
- **WAVE-C** (robustness): one implementer, ships ≥6/10 fixes, commits per unit, `timeout` on cmds.
- Run WAVE-B/C/D **in parallel**. I triage WAVE-B findings → ship high-confidence fixes myself or via
  fresh implementers. All subagents may spawn further subagents if needed.

## Gates (per changed area)

- Web: `npm run check` 0/0; `npx vitest run` green; `npm run test:e2e:fast:smoke` green + <30s.
- Rust: `cargo clippy -p <crate> --tests --all-targets` 0 warnings; `cargo nextest run -p <crate>` green.
- #444 split: byte-identical OpenAPI spec diff (spare-port instance compare).

## Blockers / forks (carried over — NOT autonomously resolvable)

- **k8s consolidation — OPERATOR CUTOVER ONLY remains** (push nuc-k3s, persist stage DB pw to
  SOPS before ArgoCD first sync, ship `:stage` image, public tunnel/DNS cutover, tear down dev host).
  See `docs/MIGRATION-CUTOVER.md`. Autonomous code work is DONE (PR #454).
- **Decent-Agents cluster** (#418/#415/#416/#427-3,4/#429-432): blocked on credentials + #413 infra.
- **#447** money-path retroactive refund replay: needs operator sign-off.
