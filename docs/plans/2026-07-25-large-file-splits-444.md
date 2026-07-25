# #444 Large-file splits — bounded evaluation & plan

**Date:** 2026-07-25 (Wave 5)
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

## Constraint that shapes all further OpenAPI splits

> poem-openapi 5.1.16 implements `OpenApi` for tuples up to arity 16, and each
> endpoint type needs its own single `#[OpenApi] impl` block. So splitting an
> OpenAPI handler file always means: (1) move a cohesive handler group into a new
> `*Api` type, (2) add it to `openapi.rs::create_combined_api` (one tuple entry),
> (3) re-export it. This is a wiring change, not a behavior change — the resulting
> routes/spec are identical. The bar is "cohesive + decoupled + tests green".

## providers.rs — recommended next extractions (in priority order)

The remaining ~5782 lines still have clearly-separable `#[oai]`-tagged clusters.
Each is a candidate for the same treatment in a follow-up PR:

| Cluster | Approx lines | Tag | Notes |
|---------|--------------|-----|-------|
| Notification config + usage + test | ~250 (3025–3225) | `Providers` | self-contained; returns `Notification*` types from common |
| SLA uptime config + summary | ~160 (3225–3390) | `Providers` | self-contained |
| Offering CSV import/export | ~280 (2200–2476) | `Providers`/`Offerings` | shares `CsvImport*` types; check helper coupling |
| Offering allowlist (get/add/remove) | ~175 (2476–2650) | `Providers` | self-contained |
| Provider stats / feedback / health summaries | ~700 (774–1194) | `Providers` | largest remaining group; verify shared helpers |

**Before each extraction**, re-run the coupling check:
`rg -n '<providers-private-symbol>' <cluster-range>` — a zero result means the
cluster can move cleanly. The free helpers to watch for: `validate_recipe_if_present`,
`validate_offering_currency`, `build_response_metrics`, `normalize_provisioning_details`,
`default_new_providers_limit`, `default_sla_days`, and the local structs
`ProviderDashboardResponse`, `OfferingSliReportRequest`/`Update…`, `Bandwidth*`,
`*AutoAcceptRuleRequest`.

## Verdicts for the other >2k-line files

| File | Lines | Verdict | Rationale |
|------|------:|---------|-----------|
| `api/src/openapi/providers.rs` | 5782 | **extract-now (in progress)** | PoolsApi done this wave; 5 more clusters identified above for follow-up PRs |
| `api/src/openapi/accounts.rs` | 2903 | **defer-with-plan** | Single `#[OpenApi] impl AccountsApi`; clusters (recovery, TOTP, contacts, keys) are cohesive but several share the account-resolution helper flow. Lower payoff than providers.rs; do after providers.rs clusters land |
| `api/src/database/offerings.rs` | 2865 | **defer-with-plan** | Pure DB layer (no OpenAPI constraint) — a split is mechanically simpler (move query groups to `offerings_*.rs` + `pub use`), but it's a different risk profile (SQL mapping) and should be its own PR with query-level test coverage confirmed first |
| `api/src/bin/api-cli.rs` | 3657 | **defer-with-plan** | CLI binary using `clap` subcommands. Natural split is per-subcommand module (`contract`, `dns`, `e2e`, `gateway`, `identity`, `health`). Each subcommand is largely self-contained; lowest-risk of the remaining files but out of the OpenAPI-focused scope of this wave |
| `website/src/lib/services/api.ts` | 4228 | **out of scope** | Frontend; separate concern. Do not touch in a backend DRY/split wave |

## Recommended sequence for closing #444

1. Land the 5 providers.rs clusters above (one PR per 1–2 clusters, each verified
   by the live-spec check + smoke).
2. Then `accounts.rs` (recovery + TOTP clusters).
3. Then `offerings.rs` (query-group split, DB-layer focused).
4. `api-cli.rs` (per-subcommand) can proceed in parallel — independent of OpenAPI.
