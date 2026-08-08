# 2026-08-08 — Documented issues + continuation sweep

## STATUS: COMPLETE (round 1 + round 2)

## Context
Previous session (2026-08-05/06 sweep) is DONE — clean tree on `main`. This session
picked up the remaining documented issues + ran the full Phase 3 sweep (UX audit,
e2e harness radicalization, robustness).

## Baseline (verified 2026-08-08)
- Start: `main` @ `3008c3d5` → round 1 ended @ `a039dbe2` → round 2 ended @ `e30b37d2`.
- `repo/target/` ownership resolved (agent:agent) — dev-cycle blocker eliminated.
- Warm stack: api http://localhost:59011, web http://localhost:59010 (postgres at hostname `postgres:5432`).

## Phase 1 — Documented issues (DONE, round 1)

All 9 documented issues fixed in round 1 (`b5be319c`–`a039dbe2`). See OPEN_ISSUES.md
"Resolved this session (2026-08-08)" for per-issue detail. Highlights:
- #466 cloud-resell race guard (`bc692bdc`)
- #452 dead CHATWOOT_INBOX_ID removed (`71210f8e`)
- #447 orphan dispute lifecycle replay (`36f5e550`)
- Chatwoot full reset (dev/stage/prod) (`8b44cb6b`)
- env parse, panic fix, debuggability, dev-server --dev flag, provision-probe token

## Phase 2 — Recent-commit verification (DONE, round 2)

Independent verifier subagent audited all 8 code commits from round 1. **All CLEAN** —
no dead/partially-wired code, no config drift. One low-severity stale rustdoc link found
+ fixed (`ee79347b`). Runtime verified against warm stack (api health 200, web 200, marketplace 200).

## Phase 3 — Big sweep (DONE, round 2)

### Phase 3a: UX audit (DONE)
No-mock audit of the REAL warm stack via browser.js + zai-vision + source inspection.
13 UX issues found (2 CRITICAL, 4 HIGH, 5 MEDIUM, 2 LOW). **11 fixed** across 8 commits.
2 deferred: UX-003 (OAuth enablement — FORK), UX-006 (reputation leaderboard — HIGH effort).
UX-009/014 deferred as MEDIUM.

### Phase 3c: Robustness sweep (DONE)
11 findings (0 CRITICAL, 2 HIGH, 5 MEDIUM, 4 LOW). **8 fixed** across 6 commits.
Codebase is notably robust — no bare `reqwest::Client::new()`, no `networkidle`,
all background loops have graceful shutdown. Remaining 5 tracked as A7.

### Phase 3b: E2e regression guards (DONE)
6 new `@smoke` tests in `ux-regression-guards.spec.ts`. FLOWS.md §6 added.
Smoke 26→32 (29.5s, under 35s target). Coverage gaps documented (UX-010, UX-012).

### A2: E2e parallel hang (DONE)
Root cause: unbounded `psql` call in worker-scoped fixture teardown bypassed the
per-test timeout → indefinite DB-lock stall under parallel workers. Fix: bounded
all DB/API helper I/O (15s timeout default). ~20 consecutive green parallel runs post-fix.

## Session summary
- **20 commits** on `main` (`a039dbe2`→`e30b37d2`), ahead of `origin/main`.
- **Gates:** npm check 0/0, e2e smoke 32/32 (28.9s), clippy 0, nextest green on all touched crates.
- **Stack never restarted** during the sweep (HMR + warm-stack e2e).

## Remaining work (next session)
- **A6:** Push 20 unpushed commits (`gh` not authenticated — blocker B1).
- **A7:** Remaining robustness items (ROB-005/006/009/012/013).
- **A8:** UX-009 (dashboard banner wall consolidation).
- **FORK:** UX-003 (OAuth enablement decision), UX-006 (reputation leaderboard design).
- **B1–B7:** Operator blockers (deploys, DNS, GitHub repo vars).
- **C1–C3:** Decent Agents epics (multi-session, design forks).

## Verification hash
- Round 2 verified @ `e30b37d2` (2026-08-08).
- Next check: broad app sweep from `e30b37d2`.

## Rules
- TDD wherever possible (RED → GREEN).
- Commit each unit when done.
- Postpone forks (Decent Agents epic #418/#415/#416/#427 — needs real Hetzner host + design forks).
- Operator blockers OP-1..OP-5 NOT autonomously fixable (deploys needed).
