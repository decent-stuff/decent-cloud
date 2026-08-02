# Plan: 2026-08-02 — E2E harness radicalization + UX audit + tech-debt closure

**Started:** 2026-08-02. **Baseline:** `a5326e16` (clean tree) → after Wave 0, `897a90e5`.

## Context

The 2026-08-01 plan (clippy + e2e gap + UX) is COMPLETE; the 2026-08-02 SaaS-removal
session is committed. No active plan. This session continues the standing radical-overhaul
mandate (harness + UX + tech debt + new-issue discovery) against a verified-green baseline.

## 0. Verified real baseline (NOT docs) — 2026-08-02

| Check | Result |
|-------|--------|
| Warm stack (`dev-server.sh start --e2e`) | up, healthy (api:59011 + web:59010) |
| Smoke e2e (`test:e2e:fast:smoke`) | **26/26 green (39.6s)** — over the <35s target |
| `svelte-check` | **0 errors / 0 warnings** |
| vitest | **858/858 (17s)** |
| `cargo clippy --workspace --tests --all-targets` | **0 warnings** |

**Prior-session WIP found + committed (Wave 0):**
- `f186c0d9` fix(api): chatwoot `create_portal` must not claim a shared `custom_domain`
  (real bug: only the FIRST provider could onboard a Help Center; every later one 422'd
  "Custom domain has already been taken". Fix: send `custom_domain=""`. TDD regression test.)
- `e5a1f08e` docs: GitHub Issues canonical-source note in AGENTS.md
- `897a90e5` chore(secrets): re-encrypt common.yaml (sops 3.9.4→3.11.0, benign)

**Stale issues confirmed already-fixed (need OPEN_ISSUES.md reconciliation; `gh` unauth):**
- **#382** `try_trigger_hetzner_provisioning` backward-compat alias → **0 matches** in dc-agent/api/cli. GONE.
- **#373** `extract_contract_id` DRY → single shared fn at `dc-agent/src/provisioner/mod.rs:12`, imported by all 3 provisioners. DONE.

## 1. E2E harness radicalization (PRIMARY ask) — subagent WAVE-A  ✅ COMPLETE

- **1a. Smoke speed:** 39.6s > <35s target. Profile per-test, find the slow ones, optimize
  (deterministic waits, shared seed, drop redundant navigations) WITHOUT losing coverage.
  Target <30s. Must stay green + reliable.
- **1b. Coverage audit:** reconcile FLOWS.md vs `website/src/routes/` + CLI commands for any
  undocumented flow. The known ⚠️/❌ rows are ALL external-dep-blocked (send-test-email needs
  MAILCHANNELS_API_KEY; password-resets empty-state needs a backing table; rent in smoke is
  >5s by design). Document any NEW gap found.
- **1c. No-mock invariant:** confirm zero first-party mocks (only Stripe SDK + outbound HTTP
  boundaries mocked). Report violations.
- **1d. Discoverability:** ensure `npx playwright test --list` + FLOWS.md make the tested-flow
  set trivially listable; add/fix any "list which flows are tested" affordance.

## 2. Live UX audit, NO MOCKS (PRIMARY ask) — subagent WAVE-B  ✅ COMPLETE

Drive the REAL warm-stack app (chrome-cli screenshots + zai-vision; Plasmate/Lightpanda for
DOM). Test functionality NOT covered by e2e. Look for: AI slop/templates, spinners/placeholders
that don't resolve in seconds (BUG), multi-step flows that could be 1 step, missing keyboard
shortcuts, visual inconsistencies. New + returning user lens. Screenshots NOT committed.

## 3. Tech-debt closure — subagent WAVE-C (parallel, low-risk)  ✅ COMPLETE

- **3a.** Reconcile OPEN_ISSUES.md: mark #382, #373 CLOSED (verified gone); audit #214/#212/
  #344/#334/#107 real status; remove resolved rows; record the reconciliation.
- **3b.** One safe #444 large-file split, byte-identical OpenAPI verified (candidate:
  `openapi/accounts.rs` 2903L clusters per roadmap, OR `dc-agent/src/main.rs` 3674L if a clean
  module boundary exists). Pick the HIGHEST-confidence split only.

## 4. New-issue discovery + persistence — folded into WAVE-A/B/C + my own pass  ✅ COMPLETE

Silent errors (`let _ =`), missing timeouts, anti-patterns (invalid-data seeding + runtime
detection), dead/zombie code, config drift. Each finding: confidence + safe score; only ship
≥6/10. Persist to `docs/OPEN_ISSUES.md`; update AGENTS.md if a new normative pattern emerges.

## Execution strategy (subagent orchestration; preserve main context)

- **WAVE-A** (e2e harness): one implementer subagent, TDD, against the warm stack.
- **WAVE-B** (UX audit): one subagent, drives the real app, reports findings (I triage + ship
  high-confidence fixes myself or via a fresh subagent).
- **WAVE-C** (tech debt): one subagent for 3a (docs reconcile), one for 3b (the split).
- All subagents: `timeout` on commands, no first-party mocks, commit per unit, report confidence.

## Gates (per changed area)

- Web: `npm run check` 0/0; `npx vitest run` green; `npm run test:e2e:fast:smoke` green + <35s.
- Rust: `cargo clippy -p <crate> --tests --all-targets` 0 warnings; `cargo nextest run -p <crate>` green.
- #444 split: byte-identical OpenAPI spec diff (spare-port instance compare).

## Blockers (carried over, NOT autonomously resolvable)

- Decent-Agents cluster (#418/#415/#416/#427-3,4/#429-432): blocked on credentials + #413 infra.
- #447 money-path retroactive refund replay: needs operator sign-off (do not auto-replay money).

## Outcome

Baseline `a5326e16` (verified green: smoke 26/26, clippy 0, vitest 858, svelte-check 0/0) → **17
commits** (`9657dee8`→`749cf876`) → final gates **all green**: smoke **26/26 in ~28s** (<30s target),
clippy **0**, vitest **862**, svelte-check **0/0**.

- **§1 (e2e):** smoke 39.6s → ~28s (`9657dee8`, dropped double-navigation in `testAccount` fixture);
  closed the `/dashboard/reputation/[identifier]/trust` coverage gap (`9e437e45`,
  `reputation-trust.spec.ts`); documented the 2 first-party fetch mocks as sanctioned exceptions
  (`41ee69b8`); fixed stale smoke-table titles + count drift (`445a17d4`, `3178799d`).
- **§2 (UX):** no-mock audit shipped 2 high-confidence fixes — U1 hero trust-card fake data →
  "Illustrative example" (`50fb8a15`); U2 all-zero marketplace stats → honest "Be Among the First
  Providers" empty state via `marketplaceIsEmpty` helper (`d719df71`). Low findings U3/U4/U5 parked
  (below/over threshold).
- **§3 (tech-debt):** stale-issue reconciliation 8→3 open rows (`e775492d`, `3178799d`); #444 Wave 9
  `TotpApi` split accounts.rs 2903→2594 (`1729e7c6`, `8c6dd37c`); #444 Wave 10 `RecoveryApi` split
  2594→2442 (`f041a121`, `d9e51a58`). Both byte-identical OpenAPI.
- **§4 (new-issue discovery):** code-robustness audit — 1 shipped (auth single-source `d34e11fb` +
  `749cf876`, R1/R2), 1 tracked (R3 `StripeClient::new().ok()` silent-swallow — money-safe + boot-gate
  mitigated, parked for a "BE LOUD about Stripe misconfig" pass). No new normative convention emerged
  (auth single-source is already an AGENTS.md bullet; now fully enforced — no outlier remains).

**Not this mandate's work (concurrent process, left untouched):** `.github/workflows/release.yml`
(`5d87411f`, pinned VITE build vars in the release website build) and the `AGENTS.md` PACKAGE REGISTRY
section (`5135e69e`, image-hotfix tagging policy). Both are coherent deploy/registry operational
additions from a parallel process; not edited here.

**Blockers unchanged:** Decent-Agents cluster (credentials + #413 infra); #447 money-path (operator
sign-off).
