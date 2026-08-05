# 2026-08-05 — Sweep Continuation (UX audit / #444 splits / robustness / e2e)

## Context
- 2026-08-03 sweep is DONE (PR opened, all gates green). No in-flight unfinished work.
- Standing mandate: "big sweep" — fresh no-mock UX audit, continued #444 splits, robustness, e2e harness radicalization.
- BLOCKED (do NOT attempt): k8s cutover (operator), Hetzner first-offerings (operator creds), #447 refund replay (operator), Decent-Agents cluster #418/#415/#416/#427 (spec-only multi-PR epic with forks + needs real Hetzner host), F6 leaderboard (needs real offerings).

## Baseline (verify before any work)
- [ ] `cargo build` (api-server) compiles
- [ ] `cargo clippy` 0 warnings
- [ ] `npx playwright test --grep @smoke` 26/26
- [ ] `vitest run` + `svelte-check` clean

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
- [ ] Update `docs/OPEN_ISSUES.md` (close resolved, add new findings, re-confirm deferrals).
- [ ] Update `repo/AGENTS.md` + this plan with status.
- [ ] Surface blockers/forks to user (Decent-Agents epic, Hetzner, k8s cutover).

## Status: IN PROGRESS
