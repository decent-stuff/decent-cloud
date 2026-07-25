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
| `api/src/openapi/accounts.rs` | 2903 | **defer-with-plan** | Single `#[OpenApi] impl AccountsApi`; clusters (recovery, TOTP, contacts, keys) are cohesive but several share the account-resolution helper flow. Lower payoff than providers.rs; do after providers.rs clusters land |
| `api/src/database/offerings.rs` | 2865 | **defer-with-plan** | Pure DB layer (no OpenAPI constraint) — a split is mechanically simpler (move query groups to `offerings_*.rs` + `pub use`), but it's a different risk profile (SQL mapping) and should be its own PR with query-level test coverage confirmed first |
| `api/src/bin/api-cli.rs` | 3657 | **defer-with-plan** | CLI binary using `clap` subcommands. Natural split is per-subcommand module (`contract`, `dns`, `e2e`, `gateway`, `identity`, `health`). Each subcommand is largely self-contained; lowest-risk of the remaining files but out of the OpenAPI-focused scope of this wave |
| `website/src/lib/services/api.ts` | 4228 | **out of scope** | Frontend; separate concern. Do not touch in a backend DRY/split wave |

## Recommended sequence for closing #444

1. ~~Land the 5 providers.rs clusters above~~ — **DONE** (Waves 5–6). providers.rs
   is down to 4280 lines and its separable clusters are exhausted.
2. **Gating prerequisite for any further OpenAPI split:** restructure
   `create_combined_api` so the second inner tuple is no longer at the arity-16
   cap (e.g. rebalance into the first inner tuple, which sits at 9, or introduce a
   third nested tuple). Do this before adding the first `accounts.rs`-derived type.
3. Then `accounts.rs` (recovery + TOTP clusters).
4. Then `offerings.rs` (query-group split, DB-layer focused).
5. `api-cli.rs` (per-subcommand) can proceed in parallel — independent of OpenAPI.
