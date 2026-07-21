# Issue #410 — Cleanup stale *pending* contracts (payment timeout)

**Issue (verbatim):** *"Contracts created in `pending` state that never receive a successful PaymentIntent accumulate indefinitely and may even provision if the race hits wrong. ... Periodic job finds contracts > configurable window (default 60 min) still in `pending`; transitions them to `expired`, releases held inventory/identity slot; emits metric for cleanup count; time-controlled unit test. Acceptance: stale pending contracts auto-expire; no provisioning attempted on expired contracts."*

**Status:** Pre-investigation assumed greenfield. **Post-investigation: largely already implemented** by commit `e62ac055` (`fix(api): timeout cleanup for stale requested and failed-provisioning contracts`) — but for the **`requested`** state, not the literal **`pending`** state named in the issue. This plan documents the conflict, the decision point, and the concrete changes for the recommended resolution.

---

## 1. Status investigation findings

### Q1 — Is `expired` a recognized contract status? **YES, no schema work needed.**
- `dcc_common::ContractStatus::Expired` exists and serializes to `"expired"` (`common/src/contract_status.rs:46`, `:155`, `:175`). It is a terminal state (`is_terminal`, line 102-105).
- The `contract_sign_requests.status` column is free-text `TEXT` with **no CHECK constraint** (`api/migrations_pg/001_schema.sql:476`; restated in `045_contract_timeout_states.sql:16-19`). The application-side enum is the source of truth. Adding new status values needs **no migration**.
- `expired` is already written in two places: `cloud_resources.rs:770,781,790` (cloud-contract end-of-life) and `contracts/timeouts.rs:104` (the existing #410 `expire_requested`).

### Q2 — What does a `pending` contract hold that needs releasing? **Inventory only (self-provisioned offerings); no identity slot. And: no production path currently creates `pending` contracts.**
- **No production write sets `contract_sign_requests.status = 'pending'`.** Contracts are created in `requested` (`rental.rs:147,192`). The Stripe webhook `checkout.session.completed` sets `payment_status='succeeded'` and **leaves status as `requested`** (`webhooks.rs:297`: *"Contract stays in 'requested' status"*). Auto-accept goes `requested → accepted` directly (`provisioning.rs:632`), skipping `pending`. The only `pending` references in production code are **read** sites (`stats.rs:162,321`) and the column **DEFAULT** (`001_schema.sql:476`: `status TEXT DEFAULT 'pending'`).
- **Inventory:** at creation (`requested` state), self-provisioned offerings reserve a `cloud_resources` row (`cloud_resources.contract_id = $1`) and flip `provider_offerings.stock_status` to `'out_of_stock'` (`rental.rs:206-249`; helper `reserve_self_provisioned_resource`, `cloud_resources.rs:155`). This reservation **persists across status changes** until explicitly released by `release_self_provisioned_resource` (`cloud_resources.rs:206`).
- **Identity slot / gateway ports:** not reserved until `provisioning`. Nothing to release for a `pending`/`requested` contract.
- The existing `expire_requested` already calls `release_self_provisioned_resource` after the transition (`timeouts.rs:156`). The same release path applies to any `pending` expiry.

### Q3 — Where do contract records live / which db methods? 
- Table: **`contract_sign_requests`**. Timeout-cleanup helpers live in **`api/src/database/contracts/timeouts.rs`** (`find_stale_requested`, `expire_requested`, `find_failed_provisioning`, `mark_provisioning_failed`; row type `StaleContractRow` at line 31-43). New helpers belong in this same file, next to `expire_requested`.

### Q4 — How is time handled in existing cleanup tests? **Seeded ns timestamps + strict `<` cutoff; no tokio clock mocking.**
- Tests use `setup_test_db()` (ephemeral Postgres) and seed rows with explicit `i64` nanosecond `created_at_ns` / `status_updated_at_ns` via the local `insert_test_contract` helper (`timeouts.rs:353-388`).
- Boundary semantics: a row whose timestamp equals the cutoff is **NOT** picked up (strict `<`); one nanosecond older IS. See `test_find_stale_requested_respects_timeout_boundary` (`timeouts.rs:661-701`) — uses fixed stamps `99`/`101` with cutoff `100`.
- The `TimeoutCleanupService` loop test seeds `status_updated_at_ns = 0` for stale rows and `now_ns()` for fresh rows, with a 1-second timeout (`timeout_cleanup_service.rs:215-243`).
- **Mirror this exact pattern** for the new pending tests.

### Q5 — Provision-loop safety check: **already safe; no change needed.**
- `get_pending_provision_contracts_for_pool` (`provisioning.rs:1007`) filters `c.status IN ('accepted', 'provisioning') AND c.payment_status = 'succeeded'` (line 1057-1058). **Both `pending` and `expired` are excluded.** Acceptance criterion "no provisioning attempted on expired contracts" is satisfied by existing code; the new `expired` transitions are belt-and-suspenders.

### Q6 (bonus) — Is there a metrics system? **No. `tracing` logs are the metric surface.**
- No `metrics`/`prometheus` crate in `api/Cargo.toml`. `network_metrics.rs` is a ledger/bandwidth stats serializer, unrelated. Every existing cleanup step emits `tracing::info!("... {}", count)` (e.g. `cleanup_service.rs:65,73,86,…` and `timeout_cleanup_service.rs:127-130`: `scanned={} expired={}`). **"Emit a metric" = a structured `tracing::info!` count line, matching convention.**

---

## 2. Decision point (REQUIRES HUMAN ACKNOWLEDGEMENT before coding)

The existing implementation and the issue text disagree on which status to expire:

| | Issue #410 text | Existing code (commit `e62ac055`) |
|---|---|---|
| Status to expire | **`pending`** | **`requested`** |
| `Pending → Expired` allowed? | (implied yes) | **Explicitly forbidden** (`contract_status.rs:473`, comment: *"Pending must NOT directly transition to Expired"*) |
| Default window | 60 min | `REQUESTED_TIMEOUT_SECONDS=3600` (= 60 min, matches) |
| Any production path creates `pending` rows? | — | **No** (only the DB column DEFAULT; `rental.rs:192` always sets `requested`) |

This is a **conflicting business-logic / data-model situation** per `repo/AGENTS.md` ("ARCHITECTURAL ISSUES THAT REQUIRE A HUMAN DECISION … inconsistent data models … Do NOT simply 'fix'"). **Flag back to the issue author** which interpretation is intended. Three resolutions:

- **Option A — Close #410 as already done.** The existing `requested`-state timeout *is* the "Stripe checkout never completed" path (a `requested` contract with `payment_status='pending'` is exactly a contract that never received a successful PaymentIntent). No code change; write a closure comment linking commit `e62ac055` + the acceptance evidence below.
- **Option B — Implement the literal `pending` timeout (RECOMMENDED).** Mirror the existing `requested` machinery for the `pending` status. Cheap, matches the issue text verbatim, and future-proofs against the `status` column DEFAULT (`001_schema.sql:476`) silently producing `pending` rows if any future INSERT omits the column.
- **Option C — Both (defensive).** Keep `requested` timeout, also add `pending` timeout.

**Recommendation: Option B/C (add the `pending` timeout alongside the existing `requested` one).** It is the smallest change that makes the issue text literally true, the schema default stops being a latent footgun, and the existing `requested` path is left intact. The changes below are for this option. If the human picks **Option A**, stop after Section 1 — there is nothing to implement.

**Money-safety guardrail (non-negotiable for Option B/C):** the `pending` expiry SQL MUST additionally require `payment_status != 'succeeded'`, so a `pending` row that somehow carries a successful payment is never silently expired (no refund logic exists in this path, mirroring `expire_requested`). This guard does NOT exist for `requested` because `requested` rows are pre-payment by construction; `pending` rows could in principle be paid edge cases.

---

## 3. Changes (landing order) — for Option B/C

Each item: file → symbol → what → why (acceptance criterion).

**3.1. `common/src/contract_status.rs` — allow `Pending → Expired`.**
- Modify `can_transition_to` (line 55-94): add `(Pending, Expired) => true,` in the `// From Pending` block (after line 67). Mirror the existing `(Requested, Expired) => true` at line 63.
- Modify `valid_transitions` (line 128-140): `Pending => &[Accepted, Rejected, Cancelled, Expired],` (replaces line 132).
- Update doc comment at line 8: `/// - Pending -> Accepted, Rejected, Cancelled, Expired`.
- **Flip the test at lines 471-473** (`test_requested_to_expired_for_pre_payment_timeout`): change `assert!(!ContractStatus::Pending.can_transition_to(ContractStatus::Expired))` → `assert!(...)` with an updated rationale comment citing issue #410 (pending = pre-payment per the issue). Add a new `test_valid_transitions_pending` assertion for the Expired target (extend the block at lines 287-294).
- *Why:* AC "transitions them to expired"; makes the literal `pending` status expirable.

**3.2. `api/src/database/contracts/timeouts.rs` — add scan + expire helpers.**
- New `pub async fn find_stale_pending(&self, older_than_ns: i64) -> Result<Vec<StaleContractRow>>` — clone of `find_stale_requested` (line 50-67) with `WHERE status = 'pending' AND COALESCE(status_updated_at_ns, created_at_ns) < $1`. Add `AND payment_status != 'succeeded'` (money-safety guardrail).
- New `pub async fn expire_pending(&self, contract_id: &[u8]) -> Result<bool>` — clone of `expire_requested` (line 101-164): `begin tx` → `UPDATE contract_sign_requests SET status='expired', status_updated_at_ns=$, status_updated_by='system-timeout' WHERE contract_id=$ AND status='pending'` (rows_affected==0 → rollback, `Ok(false)`) → insert `contract_status_history` (old_status='pending') → insert `contract_events` → commit → best-effort `release_self_provisioned_resource(contract_id)` with a `tracing::warn!` on failure (mirror line 156-162). Use `memo = "Pre-payment timeout: pending contract never completed checkout"`.
- **Audit column decision:** `expire_requested` writes `requested_expired_at_ns` (line 113). For `pending`, do **not** reuse that column (semantically wrong) and do **not** add a new column (the `contract_status_history` + `contract_events` rows are the audit trail). If the team wants column symmetry, add `pending_expired_at_ns BIGINT` via a migration — flagged as optional in Risks.
- *Why:* AC "finds pending contracts older than window" + "transitions them to expired" + "releases held inventory".

**3.3. `api/src/timeout_cleanup_service.rs` — wire the new task into the loop.**
- Add field `pending_timeout_secs: u64` to `TimeoutCleanupService` (line 31-37, mirror `requested_timeout_secs`).
- Extend `new(...)` (line 40-54) with a `pending_timeout_secs: u64` parameter and assignment.
- Add `async fn cleanup_stale_pending(&self) -> anyhow::Result<()>` — clone of `cleanup_stale_requested` (line 91-132): `cutoff_ns(self.pending_timeout_secs)` → `find_stale_pending` → per-row `expire_pending` with the same `Ok(true)/Ok(false)/Err` match + per-row `tracing::info!/debug!/error!` → final `tracing::info!("Stale \`pending\` cleanup pass: scanned={} expired={}", total, expired)`.
- Call `self.cleanup_stale_pending().await?;` in `cleanup_once()` (line 84-88) **between** `cleanup_stale_requested()` and `cleanup_failed_provisioning()`.
- Update existing `new(...)` callsite in `main.rs` (see 3.4) and the two test callsites in `timeout_cleanup_service.rs::tests` (line 243, 275) to pass the new arg.
- *Why:* AC "periodic job" + "emits metric (count)" (the `tracing::info!` scanned/expired line is the metric, per Q6).

**3.4. `api/src/main.rs` — new config env var + startup validation.**
- Add `let pending_timeout_secs = parse_env_seconds("PENDING_TIMEOUT_SECONDS", 3600)?;` next to line 1408 (default 3600s = 60 min, matching the issue). `parse_env_seconds` (line 60-80) already fail-fast-rejects `0` and non-u64.
- Pass `pending_timeout_secs` as the new arg to `TimeoutCleanupService::new(...)` at line 1450-1456.
- Extend the startup log (line 1457-1462) to include `pending_timeout: {}s`.
- Per `repo/AGENTS.md` deploy-validation rule, document the var in **`api/.env.example`** and **`cf/.env.example`** with the default and a one-line comment. (No `Doctor` check needed — it's an internal timeout, not external-service config.)
- *Why:* AC "configurable window (default 60 min)".

**3.5. `api/migrations_pg/046_pending_timeout_index.sql` — new partial index (belt-and-suspenders, mirrors 045).**
- `CREATE INDEX idx_contract_pending_timeout ON contract_sign_requests (status_updated_at_ns, created_at_ns) WHERE status = 'pending';` (mirror `045_contract_timeout_states.sql:32-34`). Keep the periodic scan O(stale).
- Also register the new migration filename in the test migration runner if it enumerates files by name (check `api/src/database/test_helpers.rs` around lines 342/651 which hardcode `032_auto_accept_rules.sql` — verify whether the runner globs the dir or lists names; if it globs, no change; if it lists, append `046_...`).
- *Why:* keeps the scan cheap and matches the existing convention; not strictly required for correctness.

**3.6. Tests — `api/src/database/contracts/timeouts.rs` (`mod tests`) and `api/src/timeout_cleanup_service.rs` (`mod tests`).**
- `test_find_stale_pending_respects_timeout_boundary` — mirror `test_find_stale_requested_respects_timeout_boundary` (line 661-701): seed stale-pending, fresh-pending, and a non-pending row; assert exactly the stale-pending row is returned and the strict-`<` boundary holds.
- `test_expire_pending_flips_status_and_writes_audit` — seed a `pending` contract; call `expire_pending`; assert status=`expired` and that exactly one `contract_status_history` (old_status=`pending`) and one `contract_events` row exist.
- `test_expire_pending_releases_marketplace_inventory` — mirror `test_expire_requested_releases_marketplace_inventory` (line 427-553): full provider/account/cloud_resource/offering/marketplace setup, reserve inventory, expire, assert `get_reserved_self_provisioned_resource` is `None` and `stock_status='in_stock'`.
- `test_expire_pending_idempotent_on_non_pending_contract` — mirror line 391-424: call on an `active` row → `Ok(false)`, no events written.
- **`test_expire_pending_skips_succeeded_payment_row` (money-safety)** — seed a `pending` contract with `payment_status='succeeded'`; call `find_stale_pending` → assert the row is NOT returned (the guard excludes it); this codifies the 3.2 guardrail.
- `test_cleanup_once_transitions_stale_pending` (in `timeout_cleanup_service.rs::tests`) — extend `test_cleanup_once_transitions_both_stale_classes_in_one_pass` (line 205-267) or add a sibling: seed a stale `pending` row, run `cleanup_once`, assert it flips to `expired` while a fresh `pending` row is untouched.
- *Why:* AC "time-controlled unit test" + project rule "every function needs meaningful tests, positive and negative".

### No changes required (verified)
- **Provision loop** `get_pending_provision_contracts_for_pool` (`provisioning.rs:1007,1057`): already excludes `pending` and `expired`. AC "no provisioning attempted on expired" satisfied by the existing positive-status filter.
- **`update_contract_status`** (`rental.rs:273`): generic; once the state machine allows `Pending→Expired` (3.1) it works for any manual/admin transition too. No allowlist to update.
- **Existing `expire_requested` / `cleanup_stale_requested`**: left untouched.

---

## 4. Test plan

**New unit tests (named in §3.6):**
- `test_find_stale_pending_respects_timeout_boundary`
- `test_expire_pending_flips_status_and_writes_audit`
- `test_expire_pending_releases_marketplace_inventory`
- `test_expire_pending_idempotent_on_non_pending_contract`
- `test_expire_pending_skips_succeeded_payment_row`
- `test_cleanup_once_transitions_stale_pending` (service-level)
- Updated `test_requested_to_expired_for_pre_payment_timeout` + new pending-transition assertion in `common`.

**Commands** (run from `repo/`):
```bash
cargo nextest run -p api contracts::timeouts     # db helpers + new tests
cargo nextest run -p api timeout_cleanup_service # service loop
cargo test -p common --lib contract_status       # state-machine change
cargo clippy --tests -p api                      # lint gate
cargo make clippy-api && cargo make test-api     # project-level gates (per api/AGENTS.md)
```

**Manual / integration verification (optional, low priority given unit coverage):**
- Start `api-server serve` locally, confirm the startup log line now prints `pending_timeout: 3600s`.
- Insert a synthetic stale `pending` row (or temporarily lower `PENDING_TIMEOUT_SECONDS`), watch logs for `Stale \`pending\` cleanup pass: scanned=1 expired=1`, and confirm the row's status flips to `expired` and `stock_status` returns to `in_stock` (for a self-provisioned offering).
- Confirm a `pending` row with `payment_status='succeeded'` is **not** touched.

---

## 5. Risks & follow-ups

1. **Decision blocker (human input required):** the existing implementation (commit `e62ac055`) deliberately expired `requested`, not `pending`, and explicitly forbids `Pending→Expired`. This plan flips that. **Confirm with the issue author** that the literal `pending` reading is intended (Option B/C) before coding — otherwise execute Option A (close as done, no code). This is the one item that should not be guessed.
2. **Money safety:** the `expire_pending` SQL must guard `payment_status != 'succeeded'`. Without it, a paid-but-pending edge row would be expired with no refund (the `requested` path is safe-by-construction; `pending` is not). Test §3.6 codifies this.
3. **Schema-default footgun:** `contract_sign_requests.status` defaults to `'pending'` (`001_schema.sql:476`). This plan turns the default into a self-cleaning state; consider a separate follow-up to change the default to `'requested'` (the only status the happy path ever inserts) so future code can't accidentally create `pending` rows. Out of scope here.
4. **Audit column symmetry:** `expire_requested` writes `requested_expired_at_ns`; the new `expire_pending` writes nothing equivalent (relies on `contract_status_history`/`contract_events`). Acceptable, but if ops wants a column to query, add `pending_expired_at_ns BIGINT` in the §3.5 migration. Flag for the implementer to confirm with the team's ops convention.
5. **Migration test runner:** verify whether `api/src/database/test_helpers.rs` enumerates migration files by glob or by hardcoded name list (it hardcodes at least `032_auto_accept_rules.sql` at lines 342/651). If hardcoded, the new `046_*.sql` must be appended there or it will not run in tests.
6. **Issue closure:** if Option B/C is executed, #410 is fully closed by this work (no split needed). If Option A, close #410 referencing commit `e62ac055` + §1 acceptance evidence.

---

## Acceptance evidence map (after implementation)
- *"Stale pending contracts auto-expire"* → §3.2 `expire_pending` + §3.6 `test_expire_pending_flips_status_and_writes_audit`.
- *"Configurable window default 60 min"* → §3.4 `PENDING_TIMEOUT_SECONDS` (default 3600).
- *"Releases held inventory"* → §3.2 `release_self_provisioned_resource` + §3.6 inventory test.
- *"Emits metric (count)"* → §3.3 `tracing::info!("Stale \`pending\` cleanup pass: scanned={} expired={}")` (convention per Q6).
- *"No provisioning attempted on expired contracts"* → already guaranteed by `provisioning.rs:1057` (verified Q5).
- *"Time-controlled unit test"* → §3.6 boundary + idempotency tests using the seeded-ns pattern (Q4).
