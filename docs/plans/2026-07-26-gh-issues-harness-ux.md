# Plan: 2026-07-26 — All GH Issues + Radical E2E Harness (TUI & Web) + UX Audit

**Started:** 2026-07-26. **Baseline:** `ca3e4f71` (clean tree, warm stack up: api:59011, web:59010).
**Context:** Previous session (`2026-07-25-robustness-cli-e2e-ux`) is complete per `OPEN_ISSUES.md`.
This session is the next continuation: tackle GH issues, radically improve both the CLI (TUI/desktop)
and Web e2e harnesses to cover ALL user flows against the REAL app, and run a fresh no-mock UX audit.

## 0. Baseline regression discovered (FIX FIRST)

`auth-capabilities.spec.ts` — 2 `@smoke` tests FAIL. Root cause: the 2026-07-25 secrets sync
populated `GOOGLE_OAUTH_CLIENT_ID/SECRET` on the warm stack, so `GET /api/v1/auth/capabilities`
now returns `{"google_oauth":true}`. The tests hardcoded `google_oauth=false` and the disabled-branch
login default. The product is correct; the tests are stale (anti-pattern: hardcoding an env
assumption the contract says the SERVER owns).

**Fix (high confidence 10, safe 10):** make the spec env-agnostic — fetch the real capability, then
assert the login page default MATCHES it (the actual contract). Covers both branches on any stack.
Update the spec header comment + the FLOWS.md row. Smoke stays green regardless of OAuth config.

## 1. Open GH issues — triage

| # | Title | Action |
|---|-------|--------|
| #447 | Replay dispute lifecycle (pause/refund) for orphans — money path | **Money-path deferred** (needs operator sign-off; pause replay already shipped `71732957`). Document; do NOT auto-replay refunds autonomously. |
| #444 | Large-file splits (>2k lines) | **One more safe split** if a clean cluster exists; else document + close this session's contribution. |
| #415,#416,#418,#427(#3/4),#429,#430,#431,#432 | Decent-Agents cluster | **BLOCKED on credentials + #413 infra** (per `OPEN_ISSURES.md`). Re-verify blocker; if still blocked, leave open with updated note. Do NOT mock/stub. |

## 2. Radical E2E harness improvement (the headline deliverable)

### 2a. CLI / TUI / desktop harness (`cli/tests/`)
Built last session (`cli_flows.rs`, 12 tests, offline + warm-stack + IC-mainnet tiers). Radical
improvement this session:
- **Cover ALL user-facing CLI flows** — audit `cli/src/commands/` vs `cli_flows.rs`; close gaps.
- **Warm-stack tier must prove the REAL api** (no mocks): identity round-trip, listings, signed
  provider commands (already-fixed auth), help/version on every subcommand.
- **Seconds-fast**: keep offline tier deterministic + instant; warm-stack auto-probes 59011.
- **One harness, no duplication**: `cli_smoke.rs` (single-command) + `cli_flows.rs` (flow-level) —
  consolidate if overlapping; keep distinct tiers if they cover different things.

### 2b. Web UI harness (`website/tests/e2e/`)
Already strong (299 tests, FLOWS.md catalog). Radical improvement per
`2026-07-25-e2e-harness-analysis.md` priorities NOT yet shipped:
- **Reliability:** fragile `:has-text` selectors → `getByRole` (highest-count specs first).
- **Speed:** route-audit blanket 700ms settle → content-gated (~15-25s saved).
- **DRY:** promote `accountIdHex`/`email_verified`/`assertNoNativeDialog`/`confirmInlineAction`.
- **Coverage:** provider password-resets / ssh-key-rotations POPULATED state (close ⚠️).
- **All flows covered:** re-scan FLOWS.md for any ❌/⚠️ closeable without external services.

### 2c. Migrate UI/UX verification to the harness
Any UI/UX check done by hand or by a brittle script → codified as a Playwright spec (Web) or a
`cli_flows.rs` test (CLI). The harness is the single verification surface.

## 3. Live no-mock UX audit (drive the REAL app)

After 0–2 land: drive the real site (anonymous + authed via `testAccount` fixture / chrome-cli)
across public + dashboard + provider + admin surfaces. Look for:
- AI slop / stubs / templates / placeholders that never resolve.
- Spinners/placeholders that don't settle in seconds (= bug).
- Multi-step flows that can be shortened (fewer clicks, keyboard).
- Keyboard-shortcut discoverability + intuitiveness.
- Inconsistencies, dead-ends, broken links, visual glitches.
File net-new findings as GH issues + add to `OPEN_ISSUES.md`; fix high-confidence (≥9/10) ones;
park product-judgment ones.

## 4. Persist + update
- `docs/OPEN_ISSUES.md` — this session's resolutions + blocker re-verification.
- `website/tests/e2e/FLOWS.md` — coverage status updates.
- `repo/AGENTS.md` conventions — any new normative patterns.
- Commit each unit of work when green.

## Execution strategy (subagent orchestration)

**Wave 0 (myself, now):** fix the baseline regression (0) — small, blocks the smoke loop.

**Wave 1 (parallel subagents):**
- ALPHA: CLI harness radical improvement (2a) — audit + extend `cli_flows.rs` to ALL flows.
- BETA: Web harness DRY+speed+reliability (2b) — the `2026-07-25-e2e-harness-analysis.md` items.
- GAMMA: one safe #444 large-file split (if a clean cluster exists) + Decent-Agents blocker re-verify.

**Wave 2 (myself or subagent):** close remaining FLOWS.md ⚠️/❌ gaps (2c).

**Wave 3 (myself, no mocks):** live UX audit (3) — drive real app, file + fix.

**Wave 4:** persist + update (4); final verification gate.

## Verification gates (every unit of work)
1. `cargo clippy --tests -p <crate>` clean in changed crate.
2. `cargo nextest run -p <crate>` (or `cargo test`) green.
3. UI: `npm run test:e2e:fast:smoke` green + relevant full-suite spec green; `browser.js errs` clean.
4. Commit each unit when green.
