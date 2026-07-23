# CLI Test-Harness Assessment — `cli/` (`decent-cloud` crate)

**Date:** 2026-07-23
**Scope:** `repo/cli/` (Rust crate, package name `decent-cloud`, binaries `dc` + `keygen`)
**Mode:** RESEARCH ONLY — no source code was modified. All commands wrapped with `timeout`.

## TL;DR

- The brief's premise ("CLI uses `dialoguer` for interactive prompts") is **incorrect**.
  `dialoguer` is declared in `cli/Cargo.toml` but has **zero references** in any `.rs` file.
  The only interactive surface is `mnemonic_from_stdin()`, which reads 12 words via plain
  `std::io::stdin().read_line()` — line-buffered, fully pipeable, **no PTY required**.
- The CLI is a **thin** (≈960 LOC of command code) wrapper over `dcc-common`,
  `ledger-map`, `ic-agent` (`LedgerCanister`), and `reqwest`. Most commands
  (`account`, `provider`, `user`, `ledger-remote`) **require a live IC canister /
  network** and cannot be exercised locally.
- 39 tests exist, all pass in ~1.2 s warm. **≈70% of them assert hardcoded string
  literals**, not behavior. The single "integration" file (`cli_error_integration.rs`)
  never invokes the binary — it asserts string constants.
- No subprocess harness exists (`assert_cmd`/`duct`/`expectrl`/`Command::new` — none).
- **Recommendation:** Do **not** build a full CLI e2e harness (low value — network-bound).
  Do build a **small** `assert_cmd`-based smoke harness for the ~6 local-only flows to
  replace the fake string-literal tests. Then invest the rest in the **Web e2e suite**
  (Playwright, already exists). Confidence 8/10.

## 1. CLI structure

Package: `decent-cloud` v0.5.3, `cli/Cargo.toml`. Two binaries:
- `dc` (`src/main.rs`) — main CLI, `#[tokio::main]`, parses args → loads local ledger →
  `refresh_ledger_and_caches` → dispatches to `commands::handle_command`.
- `keygen` (`src/keygen.rs`) — standalone sign/verify demo reading mnemonic from stdin.

Command modules (`cli/src/commands/`, 959 LOC total):

| File | LOC | Purpose | Network needed? |
|------|----:|---------|:---:|
| `account.rs` | 66 | balance, list accounts, token transfer | **yes** (IC ledger) |
| `provider.rs` | 296 | list/register/check-in, pool suggest/generate offerings | **yes** (IC + API) |
| `user.rs` | 50 | list/register user identity | **yes** (IC canister) |
| `ledger.rs` | 243 | `ledger-local` (3 flags) + `ledger-remote` (11 subcmds) | local: no; remote: **yes** |
| `keygen.rs` | 158 | BIP-39 generate / import mnemonic → identity | **no** |
| `mod.rs` | 146 | network routing → canister URL/ID dispatch | n/a |

Subcommand tree (`cli/src/argparse.rs`):
```
dc
├── keygen          --generate | --mnemonic | --language   (--identity)
├── account         --balance | --list-all | --transfer-to
├── provider
│   ├── list                          (IC)
│   ├── register                      (IC)
│   ├── check-in                      (IC)
│   ├── pool-suggest-offerings        (HTTP API)
│   └── pool-generate-offerings       (HTTP API)
├── user
│   ├── list                          (IC)
│   └── register                      (IC)
├── ledger-local     --list-entries | --list-entries-raw | --list-accounts
└── ledger-remote
    ├── data-fetch / data-push-authorize / data-push   (IC)
    ├── metadata                                       (IC)
    ├── get-registration-fee / get-check-in-nonce      (IC)
    └── get-logs-{debug,info,warn,error}               (IC)
```

**Interactive input:** only `commands/keygen.rs:mnemonic_from_stdin()` — when `--mnemonic`
is passed with <12 words, it prompts "Word N:" 12 times via `read_line`. Plain stdin,
line-delimited, no raw-mode / no `dialoguer`. **Subprocess-pipeable without a PTY.**

## 2. Existing tests

39 tests total, all passing (`cargo test -p decent-cloud`, ~1.2 s warm).

| Test binary | Tests | File | What it actually asserts |
|---|---:|---|---|
| `decent_cloud` (lib) | 12 | `src/lib.rs` | `format!("{}", CliError::…)` contains substrings — **string-literal checks** |
| `dc` (main) | 10 | `src/commands/{mod,keygen,ledger}.rs` | network match arms (re-implemented in-test), mnemonic parse, log formatting |
| `keygen` | 9 | `src/keygen.rs` | mnemonic detect, seed derivation, sign/verify — **real pure-function unit tests** |
| `cli_error_integration` | 8 | `tests/cli_error_integration.rs` | **string constants only** — never invokes `dc`; misleadingly named |

**Quality verdict:** the keygen sign/verify/mnemonic unit tests are genuine and useful.
The 20 `CliError`/string tests (lib.rs + integration file) provide **near-zero regression
value** — they re-assert the same string literals the source hardcodes, so they pass even
if the actual error path is broken. `commands/mod.rs` tests re-implement the network `match`
inside the test rather than calling `handle_command`, so they are tautological.

**No end-to-end coverage of the `dc` binary exists.**

## 3. Build / dev cycle

- Cold `cargo build -p decent-cloud`: **1 m 25 s** (compiles `ic-cdk`, `ic-canister`,
  `candid`, full dependency closure).
- Warm `cargo test -p decent-cloud --no-run`: **4.2 s**.
- Warm `cargo test -p decent-cloud`: **1.2 s** (build + run).
- **Dev cycle is fast once warmed.** A subprocess test harness adds per-run process-spawn
  cost (each `dc` invocation re-links cold from cache: ~sub-second) — acceptable.

## 4. Interactive-flow testability

- `dialoguer` is **not used** → no PTY needed. The "interactive" mnemonic prompt reads
  line-delimited stdin, so `Command::new("dc").stdin(pipe)` + writing 12 lines works.
- `assert_cmd` (idiomatic, `predicates` assertions) is the right tool; no `expectrl`/
  PTY machinery required. Already a common workspace-pattern elsewhere.
- Blocking constraint for `account`/`provider`/`user`/`ledger-remote`: they call
  `LedgerCanister::*` (live IC) or `reqwest` to `api.decentcloud.net`. Locally testable
  only against a running canister replica (none in this repo's local stack).

## 5. Coverage gaps (by user-facing flow)

Fully untested end-to-end (no binary invocation anywhere):

| Flow | Locally testable? | Value |
|---|:---:|:---:|
| `keygen --generate` (writes identity dir) | ✅ yes | **high** |
| `keygen --mnemonic <12+ words>` | ✅ yes | **high** |
| `keygen` interactive stdin (12-word prompt) | ✅ yes (pipe stdin) | medium |
| `ledger-local list-{entries,entries-raw,accounts}` | ✅ yes (temp ledger dir) | medium |
| `--network <invalid>` error dispatch | ✅ yes | medium |
| clap arg validation (conflicts/requires) | ✅ yes (`--help`/exit codes) | medium |
| `account balance/transfer` | ❌ IC | low (network) |
| `provider *` / `user *` | ❌ IC/API | low (network) |
| `ledger-remote *` | ❌ IC | low (network) |

## 6. Assessment & recommendation

**Should we build a CLI subprocess e2e harness?**

- **Full harness: NO.** The majority of commands are network-bound (IC canister / API).
  E2E for those needs a running replica + seeded canister — out of scope for a local dev
  gate, and duplicative of what the Web e2e suite already covers through the API. The CLI
  is an admin/developer tool, not the primary user surface. **Confidence 9/10 not worth it.**

- **Small targeted harness: YES.** A focused `assert_cmd`-based suite covering the ~6
  local-only flows above is high-value and cheap (~4–6 h):
  - Drives the real `dc` binary (catches arg-parsing, ledger-path, and main() dispatch
    regressions the current string-literal tests miss).
  - No PTY needed (no `dialoguer`).
  - Replaces the fake `cli_error_integration.rs` tests with actual invocation + exit-code
    + stderr assertions.
  - Add `assert_cmd` + `predicates` as `[dev-dependencies]`; use `tempfile` (already a
    dev-dep) for `--local-ledger-dir`.
  **Confidence 7/10 this is worth doing.**

- **Primary investment: the Web e2e suite** (Playwright, already exists under
  `website/` + `scripts/dev-server.sh --e2e`). That covers the user-facing rental flow
  end-to-end. The CLI harness is a complement, not a replacement.

**Bottom line:** ~6 local-only CLI flows deserve a real `assert_cmd` smoke harness; the
network-bound commands are covered (indirectly) by Web e2e; a full CLI harness is not worth
the maintenance cost.

## Evidence (commands run)
- `cargo build -p decent-cloud` → 1 m 25 s cold.
- `cargo test -p decent-cloud` → 39 passed, 1.2 s warm.
- `rg dialoguer cli/` → only `Cargo.toml` (unused dependency).
- `rg "assert_cmd|duct|expectrl|Command::new" cli/` → no matches (no subprocess harness).
- `rg "read_line|stdin" cli/src/` → only `keygen` mnemonic path.
