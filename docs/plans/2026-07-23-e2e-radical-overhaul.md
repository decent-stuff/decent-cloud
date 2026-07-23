# E2E Harness Radical Overhaul + Issue Sweep (2026-07-23)

**STATUS: IN PROGRESS**

Builds on the COMPLETE `2026-07-23-harness-hardening-and-ux-audit.md`. The prior
session left the suite green at 202/0, but a re-baseline THIS session found a
fragile test (`reputation-detail.spec.ts` hardcoded a `uxaudit` pubkey that
drifted) — fixed first (commit `c8e25e3a`). This plan tackles the deeper asks:
radical harness speed, full user-flow coverage, a fresh issue sweep, and UX
flow optimization.

## Scope reality

- **No TUI / desktop app exists.** The user-facing surfaces are:
  1. **Web UI** (SvelteKit) — primary; 55 Playwright specs, ~202 tests, 152s.
  2. **CLI** (`cli/` crate, `dialoguer` interactive prompts) — the "TUI-like" surface.
- In-scope GitHub issues (#415/#416/#418/#427) are all LARGE Decent Agents
  features requiring product decisions (Stripe billing, GitHub App, magic-link
  auth). Out of scope for autonomous work; flagged in OPEN_ISSUES.md.

## Goals (per user brief)

1. **Harness runs in seconds.** Realistic target: <60s for the full Web suite
   (from 152s). Path: empirical re-test of worker counts (prior "8 = no gain"
   claim is suspicious), test folding, and sharding feasibility.
2. **Cover ALL supported user flows.** Enumerate every route + flow, map to
   tests, close gaps. Migrate any UI/UX-verification (screenshot/manual) tests
   into the e2e harness.
3. **Real app, no mocks** (except smallest external boundary — Stripe SDK).
4. **Find & fix ALL functional/visual issues** (fresh no-mock audit).
5. **UX flow optimization** — reduce clicks/keystrokes for common paths.
6. **Persist issues** in `docs/OPEN_ISSUES.md`; update AGENTS.md.

## Method

PoC-first (per repo/AGENTS.md) → RED test → GREEN fix → commit each unit.
No mocks in prod code. DRY/KISS/YAGNI. Greenfield. Orchestrate via subagents
to preserve context. TDD: RED → GREEN → keep test.

---

## Phase 0 — Baseline + first fragile-test fix ✓ (c8e25e3a)

Re-baselined: **201 passed, 1 FAILED** (not the claimed 202/0). Root cause:
`reputation-detail.spec.ts` hardcoded `uxaudit`'s pubkey, which drifted after
re-seeding. Fixed by making the test self-contained (seed → derive pubkey →
assert → cleanup). 3/3 green in 8s.

**Lesson:** any test depending on externally-seeded state with a hardcoded
identifier is fragile. Sweep the suite for the same anti-pattern.

## Phase 1 — Parallel investigation (subagents) ⏳

- **A. Coverage + speed analysis** (planner): enumerate all `src/routes/**`,
  map to specs, produce coverage matrix; empirically re-test worker counts
  (4/8/12/16) + measure per-test overhead; identify fold candidates.
- **B. Fresh no-mock UX audit** (implementer): browser.js tour of all key
  pages + zai-vision screenshot analysis; document functional/visual defects.
- **C. Fragile-test sweep** (planner): grep for hardcoded identifiers /
  externally-seeded dependencies across all specs; list fixes.
- **D. CLI harness assessment** (planner): how is `cli/` tested today? Is
  there an integration harness for `dialoguer` interactive flows?

## Phase 2 — Harness radical improvement

(To be filled from Phase 1 findings.)

## Phase 3 — Issue fixes (TDD)

(To be filled from Phase 1 findings.)

## Phase 4 — UX flow optimization

(To be filled.)

## Phase 5 — Docs + verification

- Update `docs/OPEN_ISSUES.md` (new findings + closed items).
- Update `repo/AGENTS.md` + `website/AGENTS.md` harness notes.
- Final full-suite green run.
