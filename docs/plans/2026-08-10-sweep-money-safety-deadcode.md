# 2026-08-10 sweep: money-safety + dead-code + e2e-flake

**Scope:** finish all autonomously-actionable planned work from the 2026-08-10
`OPEN_ISSUES.md` section A. Five commits, all on `main`, all verified green.

## What was done

| Commit | Item | Summary |
|--------|------|---------|
| `bba87c98` | **Money-safety: wallet webhook top-up idempotency** | Stripe replay of `checkout.session.completed` could double-credit the wallet (no idempotency on the credit reference). Fix (TDD RED→GREEN): migration `056_wallet_topup_idempotency.sql` adds a partial UNIQUE index `wallet_ledger_topup_reference_unique ON wallet_ledger(reference) WHERE entry_type='topup' AND reference IS NOT NULL` (scoped to top-ups only — refunds keyed on contract id are NOT covered, since a contract may legitimately have multiple refund rows). New `credit_wallet_balance_idempotent()` returns `WalletCreditResult::{NewlyCredited, AlreadyProcessed}`; balance upsert + ledger INSERT in one tx; `is_unique_violation()` aborts the whole tx (no partial commit) and returns the existing balance. The webhook handler logs + returns **200 OK** on replay so Stripe stops retrying. `credit_wallet_balance` kept intact for refunds. 34/34 wallet tests pass; `.sqlx` unchanged (identical SQL string). |
| `dae7d59e` | **A11 — VM-reconciliation dead-code (was stale/invalid)** + **A12 — ICP MetadataCache retirement** | **A11:** the entry was stale — 2 of 3 cited functions were already gone. Removed the dormant `ContractPendingTermination` struct, `get_pending_termination_contracts` DB method, `/providers/:pubkey/contracts/pending-termination` endpoint, and dc-agent `get_pending_terminations`. **Kept** `get_pending_termination_RESOURCES` (`cloud_resources.rs:626`) — a different LIVE function used by `CloudProvisioningService::terminate_pending_resources` (central-API cloud-resell VM termination). **A12:** removed `metadata_cache.rs` entirely; moved `LedgerClient::new` init from shared `setup_app_context` into `sync_command` only; removed dead `metadata` field from `PlatformOverview` (ts-rs regenerated); removed dead `fetch_metadata`/`try_fetch_metadata`. +86/-528 across 11 files. |
| `ba1ed300` | **A13 — minor tech-debt** | Stale doctor hint `dc-agent setup proxmox` → `setup token` (the real subcommand). Audited 6 residual `let _ =` sites — all comments/test/example, none production. |
| `8f1e2013` | **A10 — marketplace-empty-state e2e flake (#477)** | Parallel-worker DB contamination: sibling specs (search-dsl seeds 6 always-online offerings) leaked into the empty-state spec's unfiltered marketplace view → count-0 assertion flaked. Fix: scope the empty-state test to its own provider via `?provider=<pubkey>` (the production "View all from provider" URL param). `providerFilter` is transparent to the reveal-empty-state branch, so the assertion is unchanged. Verified: 3× parallel runs (9/9 each), 36-row worst-case contamination load passes, 5-spec marketplace suite 23/23. |

## Verification

- `cargo build -p api` + `cargo build -p dc-agent`: clean.
- `cargo clippy -p api/dc-agent --tests`: clean.
- `cargo nextest run -p api`: 610+ pass (6 SIGTERM = slow DB integration tests
  killed by the 300s runner timeout, NOT assertion failures).
- `spec_snapshot` test passes (186 paths / 331 schemas).
- E2e: 5 marketplace specs 23/23; empty-state repro 3× stable.

## What was deliberately NOT done

- **A3 (large-file splits, #444):** the remaining >2k-line files
  (`dc-agent/src/main.rs`, `database/offerings.rs`, `accounts.rs`) have no
  clean decoupled cluster — `accounts.rs`'s handlers all share one tag +
  account-resolution helper; `database/offerings.rs`'s recommendations cluster
  is spatially scattered across 4 non-contiguous regions. Each is a moderate-risk
  "dedicated PR" refactor, not a quick win, so it was deferred. The plan doc was
  updated: the `create_combined_api` tuple is now balanced `(14,14)` with
  headroom, so `accounts.rs` is no longer tuple-blocked — the next session can
  take it as a focused PR.

## Wallet follow-ups still open (not blocking)

- (b) auto-renewal wallet-debit path.
- (c) verify spending-alert cap interaction with the instant-debit model.
