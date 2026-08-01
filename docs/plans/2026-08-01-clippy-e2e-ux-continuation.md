# Plan: 2026-08-01 — Clippy cleanup + E2E gap closure + UX audit (continuation)

**Started:** 2026-08-01. **Status:** §1–§4 COMPLETE. **Baseline:** `d3908303` (clean tree).
**Context:** The 2026-07-26 session plan (`2026-07-26-gh-issues-harness-ux.md`) is complete per
`OPEN_ISSUES.md` (all 5 waves shipped: baseline regression fix, CLI harness expansion, web DRY
helper, OpenAPI rebalance, live UX audit with 0 findings). No in-progress plan for today; the
Decent-Agents GH cluster remains blocked on credentials + unbuilt infra (#413) — NOT autonomously
resolvable (do not mock/stub). This session continues the radical-harness/UX/tech-debt mandate
against the verified real baseline. **Final gates all green:** `cargo clippy --workspace --tests
--all-targets` → 0 warnings; cargo lib 1469/0; smoke 27/27; rent-flow 4/4; CLI 63/6.

## 0. Verified real baseline (NOT docs) — 2026-08-01

| Check | Result | Notes |
|-------|--------|-------|
| Warm stack (`dev-server.sh start --e2e`) | up in 13s, healthy | api:59011 + web:59010 |
| API `/health` + web `/` | 200 / 200 | — |
| Smoke e2e (`test:e2e:fast:smoke`) | **27/27 green (54s)** | matches docs |
| `svelte-check` | **0 errors / 0 warnings** | matches docs |
| vitest | **870/870 (15s)** | docs said 847 — more tests since |
| `cargo clippy --workspace --tests` | **30 warnings (DRIFT)** | docs claimed "clean" |
| Cargo lib tests | running (setsid) | confirm green |
| Surfaces | CLI (`cli/`, clap) + Web (SvelteKit) | **NO TUI, NO desktop app** |

**Doc drift found:** `OPEN_ISSUES.md` lists `#442` both as "Deferred — UX" (with a "now actionable"
decision note) AND as RESOLVED/closed in the 2026-07-25 session table (`c14cb939`). GH canonical
source is unauthenticated here; reconcile the local doc.

## 1. Clippy cleanup — real tech debt (30 warnings) — ✅ COMPLETE (0 warnings)

Shipped 10 edits across api + dc-agent. See `OPEN_ISSUES.md` 2026-08-01 session row for the full
manifest. Changed-crate tests green: dc-agent 246/246, api stripe_client 18/18, api refund_gate 8/8.

| # | Warning | File | Confidence | Safe |
|---|---------|------|-----------|------|
| 1a | ~12 unused serde fields/structs in `digitalocean.rs` | dc-agent/src/provisioner/digitalocean.rs | 9 | 9 (drop unused deserialization fields; keep only what the provisioner reads; the ignored `digitalocean_tests.rs` must still compile) |
| 1b | `while let` rewrite | dc-agent/src/setup/proxmox.rs:729 | 9 | 9 |
| 1c | dead `dispute_refund_idempotency_key` (non-test) | api/src/database/contracts/dispute.rs:694 | 8 | 9 (gate behind `#[cfg(test)]` or delete if truly only test-used) |
| 1d | unused `now_ns` ×2 | api/src/database/contracts/tests.rs:5793,5858 | 10 | 10 |
| 1e | too-many-args (payment.rs:11 8/7, timeouts.rs:499 9/7, tests.rs:5429 10/7) | api/src/database/contracts/* | 6 | 8 (introduce a params struct OR `#[allow]` with justification — decide per site) |
| 1f | very complex type | api/src/database/contracts/timeouts.rs:322 | 7 | 9 (extract a `type` alias) |

Gate: `cargo clippy --workspace --tests --all-targets` must emit **0 warnings**; `cargo nextest -p
<crate>` green per changed crate.

## 2. E2E harness — gap closure + coverage audit — ✅ COMPLETE

- **2a.** Audited `FLOWS.md` + routes + CLI commands. No undocumented user-flow gaps.
- **2b.** rent→pay→view→cancel: gap already CLOSED by `rent-flow.spec.ts` (4 serial tests).
  Re-ran → **4/4 in 24.8s**. FLOWS.md + OPEN_ISSUES tech-debt rows updated.
- **2c.** CLI harness: +4 offline tests (`cli_flows.rs` +141L, 63/6 now 59/6); amount-parse
  error messages upgraded with detailed context (`cli/src/commands/account.rs`). Full coverage
  matrix + deferred gaps in the subagent report (IC-mainnet leaves only).

## 3. Live no-mock UX audit (drive the REAL app) — ✅ COMPLETE (1 root-cause fix)

Homepage + marketplace: 0 console errors. Drove anonymous + authed surfaces. Surfaced ONE real
issue — the "environment variable not found" console error during rent→pay — root-caused it to
`VarError::NotPresent` from missing `STRIPE_SECRET_KEY` (`stripe_client.rs:35`) and fixed both the
root (`.context(...)` with actionable copy) and the handler UX (contracts.rs now returns "Rental
created but payment could not be initiated: ... You can retry payment or cancel from your rentals
page." + `tracing::warn!` server log). 18/18 stripe tests + rent-flow 4/4 + live-repro verified.

## 4. Persist + update — ✅ COMPLETE

- `docs/OPEN_ISSUES.md` — snapshot → 2026-08-01; `#442` drift reconciled; 2026-08-01 session row +
  net-new keygen-duplication finding added.
- `docs/plans/<this file>` — steps marked done.
- `repo/AGENTS.md` — no new normative pattern emerged (this session reinforced existing ones:
  detailed error messages over bare `?`, single-source-of-truth for request signing, `tracing::warn!`
  for misconfiguration surfacing).
- **Commits:** none (user has not asked). All changes staged in the working tree.

## Execution strategy (subagent orchestration; preserve main context) — executed as planned

- **Wave 1 (parallel subagents):** ALPHA (clippy §1), BETA (e2e §2a/2b), GAMMA (CLI §2c) — all
  complete.
- **Wave 2 (myself, no mocks):** live UX audit (§3) — complete (1 root-cause fix).
- **Wave 3:** persist + reconcile docs (§4) — complete.

## Blockers (carried over, NOT autonomously resolvable)

- Decent-Agents cluster (#418/#415/#416/#427-3,4/#429-432): blocked on credentials + #413 infra.
- #447 money-path retroactive refund replay: needs operator sign-off (do not auto-replay money).
