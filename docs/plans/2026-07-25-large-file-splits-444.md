# #444 Large-file splits — bounded evaluation & plan

**Date:** 2026-07-25 (Wave 5); follow-up wave 2026-07-25 (Wave 6)
**Scope:** `repo/` Rust monorepo. `website/` explicitly out of scope.

## What was done this wave

`api/src/openapi/providers.rs` (6739 → 5782 lines, **−957**) — extracted the
**agent-pool / setup-token / distributed-lock** cluster (13 handlers under
`ApiTags::Pools` + their 6 tests) into a new `api/src/openapi/pools.rs` (988 lines)
as a `PoolsApi` type. Commit `74fb9248`.

Why a new type: poem-openapi's `#[OpenApi]` derive reads exactly ONE impl block per
type, so handlers cannot move to a sibling file while staying in `ProvidersApi`.
The cohesive `Pools`-tagged cluster was fully decoupled (zero references to
providers.rs-private helpers/local types — verified pre-extraction), making it the
lowest-risk extraction. Behavior is identical: every path/method/tag/schema is
unchanged (confirmed via the live `/api/v1/openapi` spec — all 189 paths present)
and the 32-test smoke e2e is green against the rebuilt api-server.

## Wave 6 (2026-07-25) — providers.rs cluster sweep continued

Three more cohesive clusters extracted from `providers.rs` (5331 → 4280 lines,
**−1051** this wave). Each is its own `*Api` type, wired into
`create_combined_api`, and verified behavior-identical via the live spec.

| Commit | New type | Cluster | Shared-helper handling |
|--------|----------|---------|------------------------|
| `290a218f` | `AllowlistApi` (`allowlist.rs`) | offering visibility allowlist get/add/remove (3 handlers, `Offerings` tag) | none — fully self-contained |
| `d94d29af` | `OfferingCsvApi` (`offering_csv.rs`) | offering CSV export + import (2 handlers, `Offerings` tag) | `validate_cloud_offering` stays in `providers.rs` as `pub(crate)` (also used by offering create/update); referenced via `use super::providers::…` |
| `b5aa9acb` | `ProviderStatsApi` (`provider_stats.rs`) | read-only analytics: stats, revenue-by-month, trust-metrics, feedback-stats, feedback list, health-summary, per-contract health summary + checks, response-metrics (9 handlers, `Providers` tag) | `build_response_metrics` stays in `providers.rs` as `pub(crate)` (also used by the dashboard handler); referenced via `use super::providers::…` |

**Verification bar (refined):** the live `/api/v1/openapi` spec is **deep-equal**
before/after each split (same 189 paths, 332 schemas, identical 475281 bytes). A
raw byte-`diff` of the serialized JSON is *not* empty — but only because
poem-openapi emits `paths` keys in tuple-registration order, so relocating a
handler group to a new tuple entry reorders keys within the `paths` object. That
reordering is non-semantic (JSON objects are unordered); the deep-dict equality
(order-independent for objects, order-sensitive for the `parameters`/`tags`
arrays) is the authoritative check and passes for all three commits.

**Tuple arity is now at the poem-openapi max.** `create_combined_api` is
`(9-tuple, 16-tuple)`; the second inner tuple is full at 16. Any further OpenAPI
split that adds a new `*Api` to that tuple requires a **tuple restructure** first
(e.g. move some entries into the first inner tuple, which has headroom at 9, or
add a third nested tuple). This is the gating constraint for the next wave.


## Constraint that shapes all further OpenAPI splits

> poem-openapi 5.1.16 implements `OpenApi` for tuples up to arity 16, and each
> endpoint type needs its own single `#[OpenApi] impl` block. So splitting an
> OpenAPI handler file always means: (1) move a cohesive handler group into a new
> `*Api` type, (2) add it to `openapi.rs::create_combined_api` (one tuple entry),
> (3) re-export it. This is a wiring change, not a behavior change — the resulting
> routes/spec are identical. The bar is "cohesive + decoupled + tests green".

## providers.rs — cluster extraction status

All five separable clusters have now been extracted (PoolsApi in Wave 5;
Notification + SLA in the interim; Allowlist + CSV + Stats in Wave 6). The
remaining ~4280 lines are the providers/offerings/rental-requests/onboarding/
reconcile/bandwidth/offering-stats/auto-accept core of `ProvidersApi` — these are
tightly interwoven (shared resolution + validation helpers) and are not low-risk
mechanical extractions, so providers.rs is considered **done** for #444's
file-split purpose.

| Cluster | Status | Type / file |
|---------|--------|-------------|
| Agent-pool / setup-token / distributed-lock | ✅ Wave 5 | `PoolsApi` (`pools.rs`) |
| Notification config + usage + test | ✅ interim | `NotificationsApi` (`notifications.rs`) |
| SLA uptime config + summary | ✅ interim | `SlaApi` (`sla.rs`) |
| Offering allowlist (get/add/remove) | ✅ Wave 6 | `AllowlistApi` (`allowlist.rs`) |
| Offering CSV import/export | ✅ Wave 6 | `OfferingCsvApi` (`offering_csv.rs`) |
| Provider stats / feedback / health summaries | ✅ Wave 6 | `ProviderStatsApi` (`provider_stats.rs`) |

## Verdicts for the other >2k-line files

| File | Lines | Verdict | Rationale |
|------|------:|---------|-----------|
| `api/src/openapi/providers.rs` | 4280 | **done (clusters exhausted)** | All 6 separable clusters extracted across Waves 5–6; remaining body is the interwoven providers/offerings/rental core. Note: combined-API tuple now at `(9, 16)` — the 16 is the poem-openapi max, so the *next* new `*Api` anywhere needs a tuple restructure first |
| `api/src/openapi/accounts.rs` | 2903 | **defer-with-plan (needs Path A first)** | Single `#[OpenApi] impl AccountsApi`; clusters (recovery, TOTP, contacts, keys) are cohesive but several share the account-resolution helper flow. Gated on the tuple restructure below (the second inner tuple is at the arity-16 cap) |
| `api/src/database/offerings.rs` | 2865 | **defer-with-plan (assessed Wave 7)** | The recommendations cluster (saved-offerings + analytics + recommendations, `impl Database` block #3) is logically self-contained but **spatially scattered**: private `SignalOffering` struct at L240, the impl methods L2311–2629, free engine fns (`build_preference_profile`/`score_candidate`), and the inline `mod recommendation_tests` L2730–2865. The pub DTOs it depends on (`Offering`, `OfferingAnalytics`, `TrendingOffering`, `OfferingPricingStats`, `DailyViewTrend`) stay put. Extracting it cleanly = pulling 4 non-contiguous regions + careful pub/private split; keep deferred to a dedicated PR with query-level test coverage, as originally cautioned |
| `api/src/bin/api-cli.rs` | ~~3657~~ → 547 (`main.rs`) | **done (Wave 7)** | Split into a directory binary `src/bin/api-cli/` with one module per `clap` subcommand + the shared `api_cli/` client/identity infra moved under it. `main.rs` keeps only `clap` wiring + leaf handlers + cross-domain shared DTOs/helpers (private; subcommand modules reach them via `crate::`). Zero OpenAPI impact |
| `website/src/lib/services/api.ts` | 4228 | **out of scope** | Frontend; separate concern. Do not touch in a backend DRY/split wave |

## Wave 7 (2026-07-26) — api-cli per-subcommand split (Path B, no OpenAPI impact)

`api/src/bin/api-cli.rs` (3753 → 547 in `main.rs`, **−3206 / −85%**) — split the
monolithic CLI binary into a directory binary (`src/bin/api-cli/`) with **one
module per `clap` subcommand**: `identity`, `account`, `contract`, `offering`,
`provider`, `notify`, `dns`, `gateway`, `health`, `e2e`, `admin`, `cloud`,
`recipe` (13 modules). The pre-existing shared `api_cli/` client+identity infra
(`client.rs`, `identity.rs`, `mod.rs`) moved under the new dir as a pure
 `git` rename (0 content change). Commit `c7dbf962`.

Why this design:
- `main.rs` is now the crate root holding only the top-level `clap` wiring
  (`Cli` / `Environment` / `Commands` / `main` dispatch), the two leaf handlers
  (`test-email`, `seed-provider`), and the cross-domain **shared DTOs + helpers**
  (DB connect, contract-lifecycle `wait`/`cancel`, cloud-account request/response
  types, `Offering`/`Contract`/`CreateContractRequest`/`RentalRequestResponse`).
- Those shared items stay **private** at the crate root; the subcommand modules
  are *descendants* and reach them via `crate::…` (Rust privacy lets a descendant
  see an ancestor's private items — including private struct fields). So **only**
  each module's `*Action` enum + `handle_*` fn need `pub(crate)`; no field-level
  visibility churn. This mirrors how `providers.rs` kept its shared helpers.
- `gateway_tests` and `cloud_tests` moved verbatim into `gateway.rs` / `cloud.rs`
  (co-located with the code they test).

Verification (Path B bar — no OpenAPI tuple touched, so no spec comparison
needed): `cargo build -p api --bin api-cli` clean; `--help` output byte-identical
(all 16 commands + every subcommand's args preserved); all 16 unit tests pass
(incl. relocated `cloud::cloud_tests::*` and `gateway::gateway_tests::*`);
`cargo clippy --tests -p api --bin api-cli` clean (the only remaining warnings
are the 3 pre-existing `database/contracts` ones, untouched by this binary-only
change). Built in an isolated `CARGO_TARGET_DIR` to avoid lock contention with a
concurrent `cargo test … webhooks` run from another session — the warm stack
(api 59011 / web 59010) was never restarted.

**What's next for #444:**
1. **Path A (gating prerequisite):** restructure `create_combined_api` so the
   second inner tuple is no longer at the arity-16 cap — either rebalance ~4
   entries from the 16-tuple into the 9-tuple (→ `(13, 12)`) or add a third
   nested tuple. Verify via the live-spec deep-equality method (spare-port
   api-server on 59012/59013, same Postgres, deep-compare paths+schemas). Ship
   this as its own commit *before* adding any new `*Api`.
2. **`openapi/accounts.rs` (2903):** then extract the recovery + TOTP clusters
   (each its own `*Api`), now that Path A gives tuple headroom.
3. **`database/offerings.rs` (2865):** the recommendations `impl Database` block
   is logically separable but scattered (see verdict above) — dedicated PR with
   query-level test coverage, not a mechanical wave task.

## Recommended sequence for closing #444

1. ~~Land the 5 providers.rs clusters above~~ — **DONE** (Waves 5–6). providers.rs
   is down to 4280 lines and its separable clusters are exhausted.
2. ~~`api-cli.rs` (per-subcommand) — independent of OpenAPI~~ — **DONE** (Wave 7):
   3753 → 547 (`main.rs`), 13 subcommand modules.
3. **Gating prerequisite for any further OpenAPI split:** restructure
   `create_combined_api` so the second inner tuple is no longer at the arity-16
   cap (e.g. rebalance into the first inner tuple, which sits at 9, or introduce a
   third nested tuple). Do this before adding the first `accounts.rs`-derived type.
4. Then `accounts.rs` (recovery + TOTP clusters).
5. Then `offerings.rs` (query-group split, DB-layer focused — dedicated PR).

## Wave 8 (2026-07-26) — Path A: combined-API tuple rebalance `(9,16) → (13,12)`

**What was done:** rebalanced `create_combined_api` in `api/src/openapi.rs` by
moving 4 standalone `*Api` types — `PoolsApi`, `NotificationsApi`, `SlaApi`,
`AllowlistApi` — from the second inner tuple into the first. The structure went
from `(9-tuple, 16-tuple)` to `(13-tuple, 12-tuple)`. No `*Api` type was added or
removed; no handler code was touched — only the 4 tuple entries changed position.
poem-openapi's spec emission order is cosmetic (JSON objects are unordered), and
these 4 types have no ordering significance relative to the others, so the move
is behavior-neutral.

**Why:** the second inner tuple was pinned at the poem-openapi arity-16 max
(`OpenApi` is implemented for tuples up to arity 16 in 5.1.16). Any new `*Api`
extraction (e.g. from `accounts.rs`) was blocked until a slot was freed. This is
the gating "Path A" prerequisite called out at the end of Wave 7.

**Verification (definitive, clean-room):** the live `/api/v1/openapi` spec is
**deep-equal** before/after the rebalance. To eliminate a stale incremental-
compilation artifact in `target/` (which initially produced a misleading non-empty
diff), the comparison was run after `cargo clean -p api` on both sides:

- Fresh debug build of original `(9,16)` source → spare api-server on `:59015` →
  `spec_before_fresh` (192 paths, 337 schemas).
- Fresh debug build of rebalanced `(13,12)` source → spare api-server on `:59016` →
  `spec_after_fresh` (192 paths, 337 schemas).
- `diff spec_before_fresh spec_after_fresh` → **empty (exit 0)**.
- Corroborating: `diff spec_after_fresh` vs the warm-stack release binary on
  `:59011` → also **empty (exit 0)**.

The warm stack (api `:59011` / web `:59010`) was never restarted; both captures
used spare ports against the same `postgres:5432`. `cargo build -p api --bin
api-server` is clean (only the 2 pre-existing `dead_code` warnings in
`refund_requests.rs`, unrelated).

**Headroom after this wave:** tuple 2 is at 12/16 → **4 free slots** for future
`*Api` extractions. Tuple 1 is at 13/16 → 3 free slots. The next wave can now add
the `accounts.rs` recovery and TOTP cluster types without a further restructure.

**Commit:** `refactor: rebalance OpenAPI tuple (9,16)→(13,12) to unblock #444 splits`
