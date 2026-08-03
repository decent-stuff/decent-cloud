# Plan: 2026-08-03 — Fresh e2e/UX/tech-debt sweep

**Created:** 2026-08-03. **Baseline:** `d3378074` (clean tree; verified green: smoke 26/26, vitest 862/862, warm stack healthy).

## Context

- The 2026-08-02 mandate (e2e radicalization + UX slop + #444 Wave 9/10/11 + auth single-source) is
  COMPLETE and committed. Baseline re-verified green at session start.
- A separate, larger plan (`2026-08-03-staging-to-k8s-dc-stage-consolidation.md`) consolidates the
  two secret stores onto k8s. It has **3 forks** (DB strategy, image strategy, hostname rename) AND
  requires live cluster / external k8s repo / Cloudflare access — infra/ops risk. **POSTPONED**;
  forks batched to the user at session end (touching live prod k8s autonomously is not safe).
- `gh` is unauthenticated in this environment → GitHub Issues fall back to the in-repo inventory at
  `docs/OPEN_ISSUES.md` (AGENTS.md notes this limitation).

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

- **k8s consolidation plan** (`2026-08-03-staging-to-k8s...`): 3 forks (DB strategy, image strategy,
  hostname rename) + live cluster / k8s repo / Cloudflare access. POSTPONED → batched user question.
- **Decent-Agents cluster** (#418/#415/#416/#427-3,4/#429-432): blocked on credentials + #413 infra.
- **#447** money-path retroactive refund replay: needs operator sign-off.
