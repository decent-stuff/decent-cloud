# Plan: 2026-07-25 Robustness Tail + CLI E2E Harness + UX Audit

**Started:** 2026-07-25. **Baseline:** `e11ec2f9` (preserved 3 untracked audit docs).
**Context:** Previous session (2026-07-25 fresh sweep `56df84e6`→`64e46ef4`) shipped the
majority of the audit findings. This plan covers ONLY the verified-still-open tail + the
new CLI e2e harness deliverable + a fresh no-mock UX audit.

## Re-verification outcome (what is actually still open)

The three 2026-07-25 audit docs were re-verified against `main`. Most findings are **already
shipped** (ICP cleanup cluster → `83605227`; reqwest timeouts → prior session; `execute_command`
timeout → shipped; `STRIPE_API_BASE` pub → shipped; chatwoot warn → shipped; `post_provision.rs`
shim deleted). The genuinely-open tail:

### A. Robustness tail (high-confidence, mechanical)
| ID | Item | Site(s) | Conf | Safe |
|----|------|---------|------|------|
| A1 | `verify_api_token` client missing `.timeout()` | `dc-agent/src/setup/proxmox.rs:432` | 10 | 10 |
| A2 | 15 `match hex::decode` sites → `decode_pubkey`/`decode_hex_path` | `api/src/openapi/{agents,webhooks,providers,accounts,pools,offerings,invoices}.rs` | 9 | 8 |
| A3 | `REQUEST_TIMEOUT_SECS` duplicated 3× in cloud crates | `api/src/cloud/{hetzner,vultr,proxmox_api}.rs`; make `HTTP_TIMEOUT_SECS` pub in `http_util.rs` | 9 | 9 |
| A4 | `dc-agent/src/upgrade.rs` 4 `Command::new` without timeout | `upgrade.rs:119,144,153,166` | 8 | 7 |
| A5 | `api-cli.rs` ssh Command without overall timeout | `api/src/bin/api-cli.rs:1614` | 8 | 9 |
| A6 | dc-agent silent swallows (`.ok()`, template enum, `ss` check) | verify + fix if still live | 8 | 8 |

### B. GH issues (in scope)
| # | Title | Plan |
|---|-------|------|
| #446 | recovery-flow e2e: SeedPhraseStep Continue never reaches Processing | Debug the fake-token test path; fix test or component |
| #445 | Recovery/verify-email success-screen auto-redirect to dashboard | Small frontend feature + test |
| #442 | create-offering auto-suggest price | **DEFERRED** (product decision) — leave open |
| #444 | large-file splits | **ONGOING** (roadmap exists) — do one more safe split if time |

### C. E2E harness remaining (from 2026-07-25-e2e-harness-analysis.md)
| ID | Item | Effort |
|----|------|--------|
| C1 | offering-edit: share one seeded offering (`beforeAll`) | S |
| C2 | agent-pool detail edit (rename / provisioner change) | S |
| C3 | become-provider wizard `?step=N` deep-link (UX + test) | S |

### D. NEW: CLI e2e harness (the "TUI/desktop" deliverable)
The `cli/` crate has 10 `assert_cmd` smoke tests but **zero flow-level coverage**. Build a fast
flow-level e2e harness that drives the **real** `cli` binary against the **real** warm stack
(api:59011, postgres). Must cover all user-facing CLI flows:
- identity: generate / list / show / import
- ledger: list-offerings / list-contracts / balance
- account: profile / devices
- provider: pool-suggest / pool-generate
- keygen: generate / import
- Full lifecycle: identity → list-offerings → create-contract (--skip-payment) → wait → get → cancel
- Must run in **seconds** (reuse warm stack, no per-test server spawn, parallel-safe)

### E. Live UX audit (no mocks — drive the real app)
After A–D land, run a fresh no-mock UX audit (anonymous + authed via `testAccount` fixture) to
find **net-new** functional/visual issues. File findings as GH issues + add to `OPEN_ISSUES.md`.

### F. Persist + update
- Update `docs/OPEN_ISSUES.md` with this session's resolutions.
- Update `repo/AGENTS.md` conventions if new patterns emerge.
- Close GH issues as they land.

## Execution strategy (subagent orchestration)

**Wave 1 (parallel — dispatched immediately):**
- Subagent ALPHA: robustness tail A1+A3+A4+A5+A6 (mechanical timeout/DRY fixes; TDD where viable)
- Subagent BETA: hex::decode migration A2 (15 sites; mechanical sweep)
- Subagent GAMMA: #446 recovery-flow debug + #445 success-screen redirect

**Wave 2 (myself — the big new deliverable):**
- Build CLI e2e harness (D) while Wave 1 runs

**Wave 3 (after Wave 1+2 land):**
- E2E harness tail C1+C2+C3 (subagent or myself)
- Live UX audit (E) — myself, no mocks
- Persist + update (F)

## Verification gates (every unit of work)
1. `cargo clippy --tests -p <crate>` clean in changed crate
2. `cargo nextest run -p <crate>` (or `cargo test`) green
3. For UI: `scripts/browser.js errs` clean; relevant e2e spec green
4. Commit each unit when green
