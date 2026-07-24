# Fresh Issue Sweep + Coverage Closure + Harness/UX Hardening (2026-07-24)

**Extends:** `2026-07-23-money-safety-hardening.md` (ALL PHASES COMPLETE) and the
`2026-07-23-cost-safe-billing.md` research doc. This plan is **net-new** work: it
assumes the prior sessions shipped and verified, and focuses on issues a fresh pass
finds plus remaining coverage/robustness gaps.

## Baseline (verified this session, before any new work)
- Warm stack healthy: api:59011 `200`, web:59010 `200`.
- **Full e2e suite: 267 passed, 0 failed, 4.2m, 4 workers** (single warm stack).
- **Smoke tier: 19 passed, 0 failed, 20.6s** (`@smoke`).
- Git clean; latest commit `40128087 feat(ux): keyboard shortcut help overlay`.
- Prior sessions closed: money-safety holes R1/R2/R3/R5/R9/R10 (DB CHECKs + code guards),
  SSE double-prefix bug, route audit 43/43 clean, command-palette keyboard nav, F1/F2 UX,
  0 networkidle, `sqlx::migrate!()` DRY. See `docs/OPEN_ISSUES.md`.

## Non-goals (explicitly out of scope — need product decisions / external services)
- Decent Agents large features (#415/#416/#418/#427) — require product + Stripe/Google/GitHub.
- R4/R6/R7/R8 (ICPay payout, e-money, escrow hold cap) — parked in billing doc.
- Real Stripe checkout completion / real VM provisioning in e2e (external boundaries).
- P2P transfers send/receive (IC canister integration).
- Multi-Postgres sharding for <60s full suite (infra investment, empirically shown not to help
  on this box; single stack 4 workers is the proven-green optimum).

## Method
PoC-first (repo/AGENTS.md) → RED test reproducing the issue → GREEN minimal fix → keep test →
commit each unit. No mocks of first-party code; Stripe SDK = the only allowed external-boundary
mock. DRY/KISS/YAGNI, greenfield. Orchestrate via subagents to preserve context; subagents create
further subagents if needed. Every subagent MUST read `docs/OPEN_ISSUES.md` first and only report
NET-NEW findings (not items already shipped or already triaged as non-bugs). Run commands with
`timeout <N>` to avoid hangs.

## Phase 1 — Parallel fresh audits (subagents, read-only, no mocks)

Three subagents run in parallel against the warm stack (web:59010, api:59011):

- **A. Fresh no-mock UX audit** (`scripts/browser.js`, real Playwright Chromium). Walk the app as a
  first-time AND returning user across ALL routes. Find NET-NEW functional/visual defects: console
  errors, dead links, spinners that never resolve, AI slop/stubs/template text, forms that submit
  without feedback, non-intuitive flows, missing keyboard affordances. Report each as
  `{route, severity(🔴/🟠/🟡), repro, file:line if known, confidence 1-10, safe 1-10}`.
- **B. Code-robustness sweep** (grep/read against `repo/`). Find concrete instances of the brief's
  anti-patterns: silent errors (`let _ =`, `if let Ok` that drops Err, `.unwrap()`/`.expect()` in
  non-test prod paths, swallowed Results), I/O without timeouts (HTTP clients, subprocess, DB),
  files >2k lines (split candidates), obvious duplication, dead/unwired code, magic numbers that
  should be a single named const. Report `{file:line, pattern, fix sketch, confidence, safe}`.
- **C. Coverage-gap + UX-optimization analysis.** Review `tests/e2e/FLOWS.md` ⚠️/❌ rows; identify
  which gaps are closeable WITHOUT external services (e.g. add-device submit, create-offering
  submit, admin mutating actions, compare full view). Identify multi-step flows that can be
  shortened (fewer clicks/keystrokes). Report `{flow, current-step-count, proposed, testable?}`.

Audits log to `docs/audits/2026-07-24-*.md`. Findings aggregated into `docs/OPEN_ISSUES.md`.

## Phase 2 — Triage + fix (TDD, ordered by severity × confidence)

Each fix: RED (failing test reproducing the issue) → GREEN (minimal fix) → keep test → commit.
Only ship fixes with confidence ≥8 AND safe ≥8. Park the rest in OPEN_ISSUES.md with rationale.

## Phase 3 — Coverage closure + UX flow optimization

Close the realistic ⚠️/❌ gaps identified in Phase 1C; codify optimized flows as e2e tests.
Update `FLOWS.md` status rows. Keep smoke <30s.

## Phase 4 — Docs + verification

Update `docs/OPEN_ISSUES.md`, `website/AGENTS.md`, this plan, `FLOWS.md`. Final full-suite green.

## Session commit log
_(updated as units land)_
