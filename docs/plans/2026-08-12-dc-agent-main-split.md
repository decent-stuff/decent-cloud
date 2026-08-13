# #444 (dc-agent leg) — split `dc-agent/src/main.rs` (3681 lines)

**Date:** 2026-08-12
**Scope:** `repo/dc-agent/` only. Pure refactor — zero behavior change.
**Parent:** `docs/plans/2026-07-25-large-file-splits-444.md` (Wave 13 candidate
analysis flagged this file at 6/10, "Path-B" — no `#[OpenApi]`, no spec guard).
**Status:** **DONE (2026-08-13).** All 5 waves shipped (S1→S5→S2→S4→S3); `main.rs`
3681→139 lines (thin clap dispatch), all logic in `dc_agent::` lib modules.
Verified per-wave: build/clippy(`--tests`)/nextest green (252 passed, 4 skipped)
and all 8 `dc-agent ... --help` outputs byte-identical to the pre-split baseline.

## Goal

Bring `dc-agent/src/main.rs` under the repo's 2k-line ceiling by moving the five
command implementations + their shared helpers into the existing **library**
crate (`dc_agent`), leaving `main.rs` as a thin `clap` dispatch. Decompose the
LARGE effort (#444) into **5 MEDIUM subtasks**, each independently committable
and testable.

## Why move code into the library (not bin-local submodules)

`dc-agent` is a **lib + bin** package. Key facts that shape the design:

1. The library `dc_agent` is **not consumed by any other crate** in the workspace
   (`grep -rn "dc_agent::" repo/` outside `dc-agent/` → zero hits). We have full
   freedom to relocate code from `main.rs` into library modules.
2. The lib already owns the cohesive subsystems the commands drive
   (`api_client`, `config`, `gateway`, `provisioner`, `setup`, `upgrade`,
   `orphan_tracker`). The command handlers are the orchestration layer *over*
   those — a natural lib surface.
3. Lib-local modules avoid (a) double-compilation of shared files and (b) the
   `src/`-directory muddiness of bin-local submodules (`src/runtime.rs` next to
   lib modules with no signal as to which crate owns it). This mirrors the
   intent of the api-cli Wave 7 split (`src/bin/api-cli/`) but reuses the
   already-present lib.
4. Direction of dependency is unchanged: `main.rs` → lib, never the reverse.

This is a **"Path-B" split** (no `#[OpenApi]`, no `/api/v1/openapi` spec to keep
byte-identical). The verification bar is therefore: `cargo build -p dc-agent` +
`cargo clippy --tests -p dc-agent` clean, `cargo nextest run -p dc-agent` green,
and `dc-agent --help` byte-identical before/after each subtask (the clap surface
must not change). No `spec_snapshot` guard applies here.

## Current structure of `main.rs` (3681 lines)

| Lines | Item | Cluster |
|------:|------|---------|
| 1–29  | imports, `ProvisionerMap` / `OptionalGatewayManager` aliases, consts | (shared) |
| 31–108 | `Cli`, `Commands` (`clap`) | **dispatch** |
| 110–189 | `SetupProvisioner` (`clap` subcommand enum) | **setup** |
| 192–245 | `main()` | **dispatch** |
| 249–677 | `run_setup_token` (429L — biggest single fn) | **setup** |
| 678–825 | `is_service_installed`, `install_systemd_service` | **host helpers** |
| 826–875 | `is_proxmox_host`, `parse_datacenter_from_pool_id` | **host helpers** |
| 877–1083 | `run_proxmox_setup_if_requested`, `run_gateway_setup_if_requested` | **setup** |
| 1085–1129 | `run_setup` | **setup** |
| 1131–1324 | `run_test_provision` | **test-provision** |
| 1325–1491 | `run_agent` (the polling loop) | **runtime** |
| 1492–1597 | `run_health_checks`, `check_for_updates_and_log` | **runtime** |
| 1598–1713 | `send_heartbeat` (free fn; only called inside `run_agent`) | **runtime** |
| 1714–2135 | `poll_and_provision` (422L) | **runtime** |
| 2136–2170 | `collect_running_by_contract` | **runtime** |
| 2171–2631 | `reconcile_instances` (461L) | **runtime** |
| 2632–2681 | `create_provisioner_from_config`, `create_provisioner_map` | **shared factory** |
| 2683–2698 | `format_bytes` | **shared util** (only `run_doctor` uses it today) |
| 2700–3219 | `run_doctor` (520L) | **doctor** |
| 3221–3294 | `run_reset_password` | **reset-password** |
| 3295–3681 | `mod tests` (387L) | (tests) |

## Cross-cluster dependency graph (the coupling that decides the order)

The **only** item shared across more than one command cluster is
`create_provisioner_from_config` (called by runtime-via-`create_provisioner_map`,
doctor, test-provision, reset-password). Everything else is a leaf helper or
cluster-internal.

```
main()
 ├─ Run         → run_agent ─┬─ create_provisioner_map ──┐
 │                          ├─ send_heartbeat (free fn)  │  SHARED
 │                          ├─ run_health_checks         │  create_provisioner_from_config
 │                          ├─ check_for_updates_and_log │  (→ dc_agent::provisioner in S1)
 │                          └─ poll_and_provision ───────┤
 │                                   ├─ collect_running_by_contract
 │                                   └─ reconcile_instances
 ├─ Doctor      → run_doctor ── create_provisioner_from_config ──┐
 │                              format_bytes ──┐                  ├─ SHARED
 ├─ TestProvision → run_test_provision ── create_provisioner_from_config
 ├─ ResetPassword → run_reset_password ── create_provisioner_from_config
 ├─ Setup        → run_setup ─ run_setup_token ─┬─ run_proxmox_setup_if_requested
 │                                              ├─ run_gateway_setup_if_requested
 │                                              └─ is_proxmox_host / parse_datacenter_from_pool_id /
 │                                                 is_service_installed / install_systemd_service
 └─ Upgrade      → dc_agent::upgrade::run_upgrade   (already in lib — untouched)
```

Implication: extract the shared **factory** + **host helpers** first (S1). After
S1, the four command clusters (doctor / runtime / setup / ops) are mutually
independent and can be extracted in any order. There is **no** lib→bin call
problem as long as a callee cluster lands in the lib before (or with) its caller;
within the runtime cluster, `run_agent` calls `poll_and_provision`, so the whole
runtime cluster ships together (S3).

## The plan — 5 MEDIUM subtasks

All new modules live under `repo/dc-agent/src/` and are declared `pub mod …` in
`src/lib.rs`. Each subtask ends with: `cargo build -p dc-agent` clean,
`cargo clippy --tests -p dc-agent` → 0 warnings, `cargo nextest run -p dc-agent`
green, and `dc-agent --help` output byte-identical to pre-subtask.

---

### S1 — Foundation: extract shared helpers into the library
**Effort:** M (2–3 h) · **Confidence:** 8/10 · **Depends on:** none · **FIRST**

Move the cross-cluster shared items out of `main.rs` into their natural lib
homes, so every later subtask consumes them via `dc_agent::…`.

- **`dc_agent::provisioner` (existing module, extend `src/provisioner/mod.rs`
  or new `src/provisioner/factory.rs`):**
  - `create_provisioner_from_config(prov_config) -> Result<Box<dyn Provisioner>>` (24L)
  - `create_provisioner_map(config) -> Result<(ProvisionerMap, String)>` (24L)
  - `type ProvisionerMap = HashMap<String, Box<dyn Provisioner>>` (alias, 2L)
  - Rationale: this is the factory over the provisioner enum — the single
    source of truth already lives in `provisioner/`. No duplication.
- **`dc_agent::host` (NEW module, `src/host.rs`):**
  - `is_proxmox_host()` (21L)
  - `parse_datacenter_from_pool_id(pool_id) -> Option<String>` (29L) + its **6
    unit tests** (`test_parse_datacenter_from_pool_id_*`, L3299–3387)
  - `is_service_installed()` (6L) + `test_is_service_installed_returns_false_in_test_env`
  - `install_systemd_service(config_path)` (142L) + the 2 `test_systemd_*` tests
    (L3389–3454)
  - `format_bytes(bytes)` (15L) — small util; groups with host diagnostics
- **`main.rs` after S1:** re-imports `dc_agent::provisioner::{create_provisioner_from_config,
  create_provisioner_map, ProvisionerMap}` and `dc_agent::host::*`; loses ~250L
  of helpers + ~165L of tests.

**Acceptance:** helpers + their tests gone from `main.rs`; `mod tests` in
`main.rs` no longer references them; build/clippy/nextest green; `--help`
unchanged.

---

### S2 — Extract `run_doctor` → `dc_agent::doctor`
**Effort:** M (2–3 h) · **Confidence:** 8/10 · **Depends on:** S1

- **`dc_agent::doctor` (NEW module, `src/doctor.rs`, ~520L):**
  - `pub async fn run(config: Config, verify_api: bool, test_provision: bool) -> Result<()>`
    (body = current `run_doctor`, unchanged)
  - Uses S1's `dc_agent::provisioner::create_provisioner_from_config` and
    `dc_agent::host::format_bytes`; existing `dc_agent::api_client::ApiClient`.
- **`main.rs`:** `Commands::Doctor { … }` arm calls
  `dc_agent::doctor::run(config, !no_verify_api, !no_test_provision).await`.
- **Tests:** none currently live in `main.rs::tests` for doctor (verified) —
  the doctor subtask carries no test moves; consider adding one as part of the
  subtask if a pure-logic slice (e.g. an extracted helper) is separable.

**Acceptance:** `run_doctor` gone from `main.rs`; `Doctor` command output
byte-identical for a fixed config (capture `dc-agent doctor --no-verify-api
--no-test-provision` before/after); build/clippy/nextest green.

---

### S3 — Extract the `Run` command (polling loop + provisioning core) → `dc_agent::runtime`
**Effort:** M-high (3–4 h) · **Confidence:** 7/10 · **Depends on:** S1 · **LARGEST**

This is the cohesive agent run-loop; it must ship as one unit because
`run_agent` calls `poll_and_provision` (lib→lib once both are in the lib).
~1300L of logic + ~215L of tests.

- **`dc_agent::runtime` (NEW module; recommended as a directory
  `src/runtime/mod.rs` + `src/runtime/reconcile.rs` for file hygiene, but ONE
  subtask/commit):**
  - `runtime/mod.rs`: `pub async fn run(config: Config) -> Result<()>` (= `run_agent`),
    `send_heartbeat` (free fn), `run_health_checks`, `check_for_updates_and_log`,
    the `SYSTEMCTL_OP_TIMEOUT` / `QUICK_QUERY_TIMEOUT` consts if runtime-scoped.
  - `runtime/reconcile.rs`: `poll_and_provision`, `collect_running_by_contract`,
    `reconcile_instances`.
  - Uses S1's `dc_agent::provisioner::create_provisioner_map`; existing
    `dc_agent::{api_client, gateway::GatewayManager, orphan_tracker::OrphanTracker}`.
- **Tests moved with their code** (currently in `main.rs::tests`):
  - `collect_running_by_contract` — 3 tests (L3466–3647, incl. the
    `tracing_test` silent-failure regression) → `runtime/reconcile.rs`.
  - `test_agent_ssh_uses_direct_port_not_gateway_port` (L3649–3680) →
    `runtime/reconcile.rs` (it exercises `provisioner::Instance` used by reconcile).
- **`main.rs`:** `Commands::Run` arm calls `dc_agent::runtime::run(config).await`.

**Acceptance:** the whole `Run` cluster gone from `main.rs`; `mod tests` in
`main.rs` loses the 4 moved tests; build/clippy/nextest green (the 3 reconcile
tests + the SSH-port test now run from `runtime::reconcile::tests`); `--help`
unchanged.

**Optional internal split** (if the implementer prefers smaller commits): land
`runtime/reconcile.rs` first (callee), then `runtime/mod.rs` (caller). The
interim state — `main.rs::run_agent` calling `dc_agent::runtime::reconcile::poll_and_provision`
— is valid (bin→lib). This turns S3 into S3a + S3b but does not change the
module boundary.

---

### S4 — Extract the `Setup` command → `dc_agent::setup_cmd`
**Effort:** M (3 h) · **Confidence:** 7/10 · **Depends on:** S1

- **`dc_agent::setup_cmd` (NEW module, `src/setup_cmd.rs`, ~880L):**
  - `pub enum SetupProvisioner` (the `clap` subcommand enum, L110–189) — moves
    here as `pub`; `main.rs` does `use dc_agent::setup_cmd::SetupProvisioner` so
    `Commands::Setup { provisioner: Box<SetupProvisioner> }` still compiles.
    (Mirrors api-cli Wave 7: `*Action` enums live with their handler.)
  - `pub async fn run(provisioner: SetupProvisioner) -> Result<()>` (= `run_setup`)
  - `run_setup_token` (429L), `run_proxmox_setup_if_requested`,
    `run_gateway_setup_if_requested`
  - Uses S1's `dc_agent::host::{is_proxmox_host, parse_datacenter_from_pool_id,
    is_service_installed, install_systemd_service}`; existing
    `dc_agent::{setup::*, api_client::*, registration::*}`.
- **`main.rs`:** `Commands::Setup { provisioner }` arm calls
  `dc_agent::setup_cmd::run(*provisioner).await`.

**Acceptance:** the `Setup` cluster + `SetupProvisioner` gone from `main.rs`;
`dc-agent setup token --help` byte-identical (the clap enum moved, not changed);
build/clippy/nextest green.

---

### S5 — Extract ops (test-provision + reset-password) + thin `main.rs`
**Effort:** M-low (2 h) · **Confidence:** 9/10 · **Depends on:** S1 (or S2–S4 done)

- **`dc_agent::ops` (NEW module, `src/ops.rs`, ~270L):**
  - `pub async fn test_provision(config, ssh_pubkey, keep, contract_id, test_gateway,
    skip_dns) -> Result<()>` (= `run_test_provision`)
  - `pub async fn reset_password(config, contract_id, password) -> Result<()>`
    (= `run_reset_password`)
  - Uses S1's `dc_agent::provisioner::create_provisioner_from_config`.
- **`main.rs` after S5 = thin dispatch (~250L):** imports, `Cli`, `Commands`,
  `main()`, and the per-arm one-line calls into
  `dc_agent::{runtime, doctor, setup_cmd, ops, upgrade}`.

**Acceptance:** `run_test_provision` / `run_reset_password` gone from `main.rs`;
`main.rs` ≤ ~300L (pure `clap` wiring); `dc-agent test-provision --help` and
`dc-agent reset-password --help` byte-identical; build/clippy/nextest green.

## Recommended order

```
S1 (foundation) ──► S5 (smallest; proves the move pattern end-to-end)
                ──► S2 (doctor; self-contained 520L)
                ──► S4 (setup; 880L, biggest single fn run_setup_token)
                ──► S3 (runtime; ~1300L + tests — last, pattern is proven)
```

S1 unblocks S2/S3/S4/S5. After S1, S2/S3/S4/S5 are mutually independent (no
cross-cluster calls except through the S1 factory) and can be reordered or
parallelized across separate branches. S3 is intentionally last so the
move-into-lib pattern is well-established on smaller, safer extractions first.
(If "eat the frog" is preferred: S1 → S3 → S2 → S4 → S5.)

## Risks & design concerns

- **Clap surface drift (highest-priority invariant).** Every subtask MUST keep
  `dc-agent --help` (and each subcommand's `--help`) byte-identical. The
  `SetupProvisioner` enum move in S4 is the highest-risk step — verify
  `dc-agent setup token --help` before/after. Capture `--help` for all commands
  pre-S1 as the baseline.
- **`#[allow(clippy::too_many_arguments)]` on `run_setup_token`.** Moves with
  the fn in S4; no change needed, but confirm the lint stays silenced at the new
  site.
- **`SYSTEMCTL_OP_TIMEOUT` / `QUICK_QUERY_TIMEOUT` consts.** Confirm which
  cluster uses each before assigning them (runtime vs doctor vs shared). If
  shared, hoist into `dc_agent::lib.rs` next to `HTTP_TIMEOUT_SECS`; if
  cluster-local, move with the cluster.
- **Test relocation discipline.** Each test references items via `use super::*`
  today. When a tested fn moves to a lib module, its test moves with it and the
  `use super::*` continues to resolve. Do NOT leave a test in `main.rs::tests`
  that references a moved fn (it won't compile).
- **No `spec_snapshot`-style guard exists for the CLI surface.** Unlike the
  OpenAPI splits, there is no committed byte-hash guard for `--help` output.
  Consider adding a tiny `tests/cli_help_snapshot.rs` (snapshot of `--help` for
  every command) as a TDD gate in S1 — it then protects S2–S5 for free. This is
  the single biggest confidence multiplier for the whole effort.
- **Library API surface grows.** The new modules are `pub`. Since no external
  crate consumes `dc_agent`, this is acceptable, but the modules should use
  `pub` (not `pub(crate)`) only where the bin actually reaches in; prefer tight
  `pub fn run(...)` entry points and keep helpers module-private.
- **Behavior must be identical.** No logic edits during the move — pure
  relocation. Any tempting cleanup (e.g. splitting `run_setup_token`,
  refactoring `reconcile_instances`) is OUT OF SCOPE and belongs in a follow-up.

## Verification checklist (per subtask, per repo `POST-CHANGE CHECKLIST`)

1. `cargo build -p dc-agent` clean.
2. `cargo clippy --tests -p dc-agent` → 0 warnings, 0 errors.
3. `cargo nextest run -p dc-agent` → all green (moved tests run from new sites).
4. `dc-agent --help` + every subcommand `--help` byte-identical to pre-subtask
   baseline (diff against the S1-captured snapshot).
5. `git diff --stat` confirms: new module(s) added, `main.rs` shrinks by the
   expected line count, `lib.rs` gains the `pub mod …` line(s). No logic lines
   changed in the moved bodies.

## Other >2k-line files — current status (verified 2026-08-12)

| File | Lines | Status |
|------|------:|--------|
| `dc-agent/src/main.rs` | **3681** | **This plan.** Target: ≤ ~300L after S1–S5. |
| `api/src/openapi/providers.rs` | 4082 | **done (prior #444 waves).** All separable clusters extracted; remaining ~4k is the interwoven providers/offerings/rental core. Not a mechanical target. |
| `api/src/database/offerings.rs` | 2981 | **deferred (prior verdict, still holds).** Recommendations cluster (`impl Database` block #3) is logically separable but spatially scattered across 4 non-contiguous regions — needs a dedicated PR with query-level test coverage, not a mechanical wave task. |
| `api/src/database/cloud_resources.rs` | 2701 | **skip (not actually >2k prod).** `#[cfg(test)] mod tests` begins at L1092 → prod code is ~1091L, tests ~1610L. Under the 2k ceiling once tests are excluded. |

No NEW >2k prod-code file has appeared since the prior #444 wave assessment.
`api/src/openapi/accounts.rs` (now 1312L) and `api/src/database/accounts.rs`
(1312L + `accounts/tests.rs` 1944L) are the completed reference splits for this
lib-extraction pattern.

## Out of scope

- Splitting `dc-agent/src/setup/gateway.rs` (33k bytes) or
  `dc-agent/src/setup/proxmox.rs` (28k bytes) — they are under the line ceiling
  and already cohesive.
- Any logic/behavior change to the moved functions.
- The `api/` >2k files above (separate concerns, separately deferred).
