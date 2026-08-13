# Split design — `database/offerings.rs` (2966L) + premise correction

**Date:** 2026-08-13
**Scope:** `repo/api/src/database/` (analysis) → proposed split of `offerings.rs`.
**Parent:** `docs/plans/2026-08-12-dc-agent-main-split.md` (TD-1 done; this is the next
large-file-design pass the parent flagged as deferred).
**Status:** DESIGN ONLY — no code changed. This document supersedes the prior "deferred /
no clean clusters" verdict for `offerings.rs` with evidence.

## TL;DR (read this first)

1. **Premise correction.** The brief named two files: `api/src/database/providers.rs` (4082L)
   and `api/src/database/offerings.rs` (2981L). **`database/providers.rs` does not exist** — it
   was already split into the `database/providers/` directory (mod.rs + auto_accept.rs +
   external.rs + sla.rs + tests.rs) in commit `51220d1d` "refactor: split contracts.rs and
   providers.rs into focused modules (#173)". That work is **DONE**. The "4082-line providers.rs"
   the brief reached for is actually `api/src/openapi/providers.rs` (the *handler* layer, now
   **4325L**) — a different file and a different concern (Path-A `#[OpenApi]`, see §9).
2. **The real database target is `offerings.rs` (2966L, ~2827L prod + ~133L inline tests +
   5571L external `tests.rs`).** It is over the 2k prod-code ceiling and is a legitimate split
   target.
3. **The prior verdict ("no clean decoupled clusters", "recommendations scattered across 4
   non-contiguous regions") is DISPROVEN by call-graph analysis.** The three `impl Database`
   blocks are **mutually independent** (star topology around the shared `Offering` struct core).
   The split is clean and mechanical — it mirrors the *already-shipped* `providers/` convention
   exactly (`impl Database` blocks spread across submodules, each doing `use super::*`).
4. **Proposed: 5 MEDIUM subtasks (S1 foundation + S2–S5 cluster extractions)** that take
   `offerings.rs` 2966L → a thin `offerings/mod.rs` (~280L core) + 5 cohesive submodules. Pure
   relocation, zero behavior change.

## Goal

Bring `api/src/database/offerings.rs` under the repo's 2k prod-code ceiling by moving its three
`impl Database` blocks + the independent tier/suggestion cluster into submodules under a new
`offerings/` directory (the directory already exists, holding only `tests.rs`). Decompose the
LARGE effort (#444) into **5 MEDIUM subtasks**, each independently committable and testable,
mirroring the proven `providers/` split.

## Why the prior "no clean clusters" verdict was wrong

The parent plan (`2026-08-12-dc-agent-main-split.md`, line 304) deferred `offerings.rs` with:
> *Recommendations cluster (`impl Database` block #3) is logically separable but spatially
> scattered across 4 non-contiguous regions — needs a dedicated PR.*

That was a **preliminary judgment**. Deep call-graph analysis shows the clusters are clean:

| Concern from prior verdict | Actual finding |
|---|---|
| "no clean decoupled clusters" | The 3 `impl Database` blocks have **zero cross-block private-method calls** (verified by grepping every private method's callsites + an `awk` scan of block#3 for block#1/#2 method calls → empty). |
| "recommendations scattered across 4 non-contiguous regions" | True *spatially* (structs at L293–308 + L2734; impl at L2414–2745; free fns at L2747–2826; tests at L2833–2966). But a **move** gathers them into one contiguous file. Scattering is not a blocker for extraction — only for a *non-move* refactor. |
| "needs query-level test coverage first" | Coverage already exists: external `offerings/tests.rs` (5571L, ~60 tests) covers CRUD/search/DSL/analytics + tier tests; inline `recommendation_tests` (5 tests) covers the pure scoring fns. Tests move or stay-referencing with zero logic change. |

## Current structure of `offerings.rs` (2966L) — full inventory

| Lines | Item | Cluster |
|------:|------|---------|
| 1–22 | imports (`Database`, `insert_notification`, `BackendType`, `country_to_region`/`is_valid_country_code`, `anyhow`, `Object`, `Deserialize`/`Serialize`, `Row`, `HashMap`/`HashSet`, `TS`) | (shared) |
| 24–74 | `is_cloud_resell` (`pub(crate)`), `is_marketplace_visible`, `is_rentable_now` (free fns) | **CORE** (visibility helpers) |
| 76 | `OFFERING_BASE_SELECT: &str` (the canonical SELECT) | **CORE** |
| 81–233 | `pub struct Offering` (~155L — the shared type) | **CORE** |
| 235–243 | `opt_f64_changed` (free fn) | **WRITE** (only block#2 uses it) |
| 245–289 | `OfferingPricingStats`, `TrendingOffering`, `RecommendedOffering` structs | **ANALYTICS** / **RECS** |
| 293–308 | `UserPreferenceProfile`, `SignalOffering` (private structs) | **RECS** |
| 310–327 | `OfferingAnalytics`, `DailyViewTrend` structs | **ANALYTICS** |
| 329–354 | `pub struct OfferingTier` | **TIERS** |
| 356–423 | `default_compute_tiers`, `default_gpu_tiers` (free fns) | **TIERS** |
| 425–435 | `UnavailableTier` struct | **TIERS** |
| 437–469 | `OfferingSuggestion` struct | **TIERS** |
| 471–511 | `select_applicable_tiers` (free fn) | **TIERS** |
| 513–570 | `check_tier_eligibility` (private free fn) | **TIERS** |
| 572–584 | `region_to_country_code` (private free fn) | **TIERS** |
| 586–640 | `generate_suggestions` (free fn) | **TIERS** |
| 642–653 | `pub struct SearchOfferingsParams<'a>` | **CORE** |
| 655–1153 | `impl Database { … }` — **block #1 READ** (~500L) | **READ** |
| 1154–2413 | `impl Database { … }` — **block #2 WRITE** (~1260L) | **WRITE** |
| 2414–2745 | `impl Database { … }` — **block #3 SAVED/ANALYTICS/RECS** (~330L) | **ANALYTICS** + **RECS** |
| 2734 | `CandidateOffering` (private struct) | **RECS** |
| 2747–2826 | `build_preference_profile`, `score_candidate` (free fns) | **RECS** |
| 2828–2829 | `#[cfg(test)] mod tests;` (external, 5571L) | (tests) |
| 2833–2966 | `#[cfg(test)] mod recommendation_tests` (5 tests) | (tests → RECS) |

### `impl Database` block #1 — READ (L655–1153)
`search_offerings`, `compute_provider_online_status` (private), `get_provider_offerings`,
`get_provider_offerings_public`, `get_offering`, `get_example_offerings`,
`get_example_offerings_by_type`, `get_available_product_types`, `example_provider_pubkey`,
`resolve_public_offerings_with_status` (private), `resolve_rentable_offerings`,
`count_rentable_offerings`, `search_offerings_dsl`.

### `impl Database` block #2 — WRITE (L1154–2413)
`count_offerings`, `notify_saved_offering_price_change` (private), `create_offering`,
`update_offering`, `publish_scheduled_offerings`, `bulk_publish_offerings`, `delete_offering`,
`duplicate_offering`, `bulk_update_stock_status`, `bulk_update_offering_prices`,
`import_offerings_csv`, `import_seeded_offerings_csv`, `import_offerings_csv_internal`
(private), `parse_csv_record` (private free fn).

### `impl Database` block #3 — SAVED/ANALYTICS/RECS (L2414–2745)
`save_offering`, `unsave_offering`, `get_saved_offerings`, `is_offering_saved`,
`get_saved_offering_ids`, `record_offering_view`, `get_offering_analytics`,
`get_offering_view_trends`, `get_trending_offerings`, `get_offering_pricing_stats`,
`get_recommended_offerings`, `fetch_user_signal_offerings` (private),
`fetch_seen_offering_ids` (private), `fetch_candidate_offerings` (private).

## Cross-cluster dependency graph (the evidence the split rests on)

```
                 ┌─────────────────────────────────────────────┐
                 │  CORE (offerings/mod.rs):                    │
                 │   Offering struct, OFFERING_BASE_SELECT,     │
                 │   is_cloud_resell / is_marketplace_visible / │
                 │   is_rentable_now, SearchOfferingsParams     │
                 └──────┬──────────┬──────────┬──────────┬──────┘
                        │          │          │          │
            ┌───────────┘          │          │          └────────────┐
            ▼                      ▼          ▼                       ▼
       tiers.rs (S2)          read.rs(S3)  write.rs(S4)    analytics.rs(S5a)
   ZERO Offering-impl deps    block#1      block#2         block#3 read-side
   (pure logic; consumed      ──────────   ──────────      + 5 response structs
    only by openapi handlers  uses CORE    uses CORE       ──────────
    + tests.rs)               only         +opt_f64_       uses CORE only
                                          changed(local)                 │
                                                                        ▼
                                                              recommendations.rs(S5b)
                                                              block#3 rec-side + free fns
                                                              + Signal/UserPref/Candidate structs
                                                              + recommendation_tests
                                                              ──────────
                                                              uses CORE only
```

**The decisive findings (every private method's callsites, verified):**

- **block#1 private `compute_provider_online_status`** → 6 callsites (L742, 896, 922, 955, 1051,
  1141) — **all in block#1** (`search_offerings`, `get_provider_offerings[_public]`,
  `get_offering`, `resolve_rentable_offerings`, `search_offerings_dsl`). ✅ intra-block.
- **block#1 private `resolve_public_offerings_with_status`** → 1 callsite (L1065, called by
  `resolve_rentable_offerings`). ✅ intra-block.
- **block#2 private `notify_saved_offering_price_change`** → 2 callsites (L1765, 2107 —
  `update_offering`, `bulk_update_offering_prices`). ✅ intra-block.
- **block#2 private `import_offerings_csv_internal` + `parse_csv_record`** → used only by the
  CSV-import methods in block#2. ✅ intra-block.
- **block#3 privates** (`fetch_user_signal_offerings`, `fetch_seen_offering_ids`,
  `fetch_candidate_offerings`) → used only by `get_recommended_offerings` (block#3). ✅ intra-block.
- **`awk 'NR>=2414 && NR<=2745'` scan of block#3 for `self.<block#1/#2 method>` calls → EMPTY.**
  block#3 never calls block#1/#2 methods. ✅ block#3 is fully self-contained.
- **Tier/suggestion free fns (L356–640)** → referenced **only by each other**; **ZERO** references
  inside any `impl Database` block. ✅ fully independent (consumed only by
  `openapi/providers.rs` `get_offering_suggestions`/`generate_offerings` handlers + `tests.rs`).

**Conclusion:** the dependency graph is a clean **star** — CORE at the center, every cluster
depends only on CORE, no cluster depends on another. This is *more* decoupled than the dc-agent
`main.rs` split (which had the shared `create_provisioner_from_config` factory). All clusters can
be extracted independently and in any order after the CORE is in place.

## Public-surface stability — the one load-bearing design rule

External callers reach `offerings` items by **full path** (`crate::database::offerings::X`).
To keep these paths valid after the split (zero caller edits), `offerings/mod.rs` must
**re-export** the `pub` items that move into submodules:

- `is_cloud_resell` (`pub(crate)`) — consumed by `database/stats.rs`. (Stays in mod.rs — it's a
  CORE visibility helper.)
- `select_applicable_tiers`, `default_compute_tiers`, `default_gpu_tiers`, `generate_suggestions`
  → consumed by `openapi/providers.rs` (L2898, L3021) and `offerings/tests.rs` (L3339, 3389, 3436,
  3481, 3527) as `crate::database::offerings::*`. **mod.rs does `pub use tiers::*;`.**
- `OfferingPricingStats`, `TrendingOffering`, `OfferingAnalytics`, `DailyViewTrend`,
  `RecommendedOffering` → consumed by `openapi/offerings.rs` (L224, 256, 744, 870, 945) +
  `openapi/common.rs` (`OfferingSuggestion`, `UnavailableTier`). **mod.rs does
  `pub use analytics::*; pub use tiers::*;`** (the tier types are re-exported for the same reason).
- **Methods on `Database`** (`create_offering`, `search_offerings`, …) need **no** re-export —
  Rust method resolution is crate-global; `db.create_offering(...)` resolves regardless of which
  submodule the `impl Database` block lives in.

The `providers/` split already follows this exact convention (submodules do `use super::*` +
`use crate::database::types::Database`; external callers use `crate::database::providers::X`).
**Reproducing it here is mechanical.**

### Callers of `crate::database::offerings::` (verified across `api/src/`)
`database/stats.rs` (`is_cloud_resell`), `database/users.rs` + `database/contracts/payment.rs` +
`database/cloud_resources.rs` + `database/visibility_allowlist.rs` (`Offering` type),
`database/tests.rs` (`SearchOfferingsParams`), `openapi/common.rs`
(`OfferingSuggestion`/`UnavailableTier`/`Offering`), `openapi/users.rs` + `openapi/cloud.rs`
(`Offering`), `openapi/offerings.rs` (8 types), `openapi/providers.rs` (`Offering` +
`generate_suggestions`/`select_applicable_tiers`). All resolve through CORE-in-mod.rs + re-exports.

## The plan — 5 MEDIUM subtasks

All new modules live under `api/src/database/offerings/` (directory already exists with
`tests.rs`). Each subtask ends with: `cargo build -p api` clean, `cargo clippy --tests -p api` →
0 warnings, `cargo nextest run -p api database::offerings` green, and `git diff --stat` showing
only moves (no logic-line changes in moved bodies). Pure relocation — **no logic edits**.

This is a **"Path-B" split** (no `#[OpenApi]`, no `/api/v1/openapi` spec to keep byte-identical).
The DB module has no HTTP surface, so there is **no spec-snapshot guard** to satisfy — only the
build/clippy/test bar above.

---

### S1 — Foundation: `offerings.rs` → `offerings/mod.rs` + extract `tiers.rs`
**Effort:** M (2–3 h) · **Confidence:** 9/10 · **Depends on:** none · **FIRST**

Establishes the directory-module pattern (the same one `providers/` uses) and proves it
end-to-end on the **single most independent** cluster (zero `impl Database` coupling).

- **Convert `offerings.rs` → `offerings/mod.rs`** (mechanical: `git mv` the file into the existing
  `offerings/` dir; the existing `mod offerings;` in `database/mod.rs` resolves to
  `offerings/mod.rs` unchanged). After the `git mv`, `offerings/mod.rs` is ~2966L (the whole file)
  — this is the interim state.
- **`offerings/tiers.rs` (NEW, ~290L)** — move the independent tier/suggestion cluster:
  - structs: `OfferingTier`, `UnavailableTier`, `OfferingSuggestion`
  - free fns: `default_compute_tiers`, `default_gpu_tiers`, `select_applicable_tiers`,
    `check_tier_eligibility` (private), `region_to_country_code` (private), `generate_suggestions`
  - header: `use super::*;` (brings `Offering`/etc. from mod.rs core; tiers need nothing else).
- **`offerings/mod.rs` after S1:** gains `mod tiers; pub use tiers::*;` (re-export keeps
  `crate::database::offerings::{select_applicable_tiers, default_compute_tiers,
  default_gpu_tiers, generate_suggestions, OfferingTier, OfferingSuggestion, UnavailableTier}`
  valid for `openapi/providers.rs` + `offerings/tests.rs`). Loses ~290L.

**Acceptance:** `tiers.rs` compiles with `use super::*`; all `crate::database::offerings::*`
tier paths still resolve (no caller edits); build/clippy(`--tests`)/nextest green; `tests.rs`'s
tier tests (`test_generate_suggestions`, the `select_applicable_tiers` tests at L3338–3550) pass
from the unchanged paths.

---

### S2 — Extract `read.rs` (block #1 — READ cluster)
**Effort:** M (2–3 h) · **Confidence:** 9/10 · **Depends on:** S1

- **`offerings/read.rs` (NEW, ~510L)** — move `impl Database` block #1 (L655–1153) verbatim:
  `search_offerings`, `compute_provider_online_status` (private), `get_provider_offerings`,
  `get_provider_offerings_public`, `get_offering`, `get_example_offerings`,
  `get_example_offerings_by_type`, `get_available_product_types`, `example_provider_pubkey`,
  `resolve_public_offerings_with_status` (private), `resolve_rentable_offerings`,
  `count_rentable_offerings`, `search_offerings_dsl`.
  - header: `use super::*;` + `use crate::database::types::Database;` (mirrors
    `providers/external.rs`). Uses CORE's `OFFERING_BASE_SELECT`, `SearchOfferingsParams`, the
    visibility free fns, and `Offering`.
- **`offerings/mod.rs`:** gains `mod read;`. The block#1 methods become `read::*` but resolve as
  `db.search_offerings(...)` everywhere (method resolution is global). Loses ~500L.

**Acceptance:** block#1 gone from mod.rs; `db.search_offerings`/`get_offering`/etc. still resolve
from all openapi callers; build/clippy/nextest green; the ~30 search/read tests in `tests.rs`
pass unchanged.

---

### S3 — Extract `write.rs` (block #2 — WRITE cluster)
**Effort:** M-high (3–4 h) · **Confidence:** 8/10 · **Depends on:** S1 · **LARGEST**

- **`offerings/write.rs` (NEW, ~1260L)** — move `impl Database` block #2 (L1154–2413) + the
  block-local `opt_f64_changed` free fn (L235–243, only block#2 uses it) + the block-local
  `parse_csv_record` free fn:
  `count_offerings`, `notify_saved_offering_price_change` (private), `create_offering`,
  `update_offering`, `publish_scheduled_offerings`, `bulk_publish_offerings`, `delete_offering`,
  `duplicate_offering`, `bulk_update_stock_status`, `bulk_update_offering_prices`,
  `import_offerings_csv`, `import_seeded_offerings_csv`, `import_offerings_csv_internal`
  (private), `parse_csv_record`, `opt_f64_changed`.
  - header: `use super::*;` + `use crate::database::types::Database;` + `use
    crate::database::user_notifications::insert_notification;` (used by the price-change notifier)
    + `use crate::cloud::types::BackendType;` (used by `is_cloud_resell`-adjacent write checks).
- **`offerings/mod.rs`:** gains `mod write;`. Loses ~1260L.

**Why this is the largest/riskiest subtask:** `create_offering` (~240L) and `update_offering`
(~250L) are the two biggest methods; they also touch the most columns. They are **pure moves** —
do not refactor them in this PR. The `notify_saved_offering_price_change` private method is
called by two block#2 methods → moves with them, still intra-module. `opt_f64_changed` moves with
the block (only block#2 uses it — verified).

**Acceptance:** block#2 gone from mod.rs; CRUD/bulk/CSV methods still resolve from openapi
callers; build/clippy/nextest green; the ~30 CRUD/CSV/DSL tests in `tests.rs` pass unchanged.

---

### S4 — Extract `recommendations.rs` (block #3 rec-side — the prior "deferred" cluster)
**Effort:** M (2–3 h) · **Confidence:** 9/10 · **Depends on:** S1

This is the cluster the prior verdict deferred. Doing it **before** analytics proves (on real
code, not judgment) that it is cleanly separable.

- **`offerings/recommendations.rs` (NEW, ~330L)** — move:
  - structs: `RecommendedOffering` (L245–289), `UserPreferenceProfile` (L293–299, private),
    `SignalOffering` (L302–308, private), `CandidateOffering` (L2734, private)
  - `impl Database` methods (the rec half of block#3): `get_recommended_offerings`,
    `fetch_user_signal_offerings` (private), `fetch_seen_offering_ids` (private),
    `fetch_candidate_offerings` (private)
  - free fns: `build_preference_profile` (L2747), `score_candidate` (L2783)
  - the inline `#[cfg(test)] mod recommendation_tests` (L2833–2966, 5 tests) → moves here
    verbatim (it tests `build_preference_profile`/`score_candidate` via `use super::*`).
  - header: `use super::*;` + `use crate::database::types::Database;` + `use
    std::collections::{HashMap, HashSet};`.
  - **`RecommendedOffering` is `pub`** and consumed by `openapi/offerings.rs:945` → mod.rs does
    `pub use recommendations::RecommendedOffering;` (or `pub use recommendations::*;`) to keep the
    path valid.
- **`offerings/mod.rs`:** gains `mod recommendations; pub use recommendations::RecommendedOffering;`.

**Acceptance:** recs cluster + its 5 inline tests gone from mod.rs; `db.get_recommended_offerings`
still resolves; `crate::database::offerings::RecommendedOffering` still valid; build/clippy/nextest
green; the 5 `recommendation_tests` run from `recommendations.rs`.

---

### S5 — Extract `analytics.rs` (block #3 read-side — saved/views/analytics) + thin mod.rs
**Effort:** M (2–3 h) · **Confidence:** 9/10 · **Depends on:** S1

- **`offerings/analytics.rs` (NEW, ~340L)** — move the remaining block#3 methods + their response
  structs:
  - structs: `OfferingPricingStats`, `TrendingOffering`, `OfferingAnalytics`, `DailyViewTrend`
  - `impl Database` methods: `save_offering`, `unsave_offering`, `get_saved_offerings`,
    `is_offering_saved`, `get_saved_offering_ids`, `record_offering_view`,
    `get_offering_analytics`, `get_offering_view_trends`, `get_trending_offerings`,
    `get_offering_pricing_stats`.
  - header: `use super::*;` + `use crate::database::types::Database;`.
  - mod.rs does `pub use analytics::*;` (keeps `OfferingPricingStats`/`TrendingOffering`/
    `OfferingAnalytics`/`DailyViewTrend` paths valid for `openapi/offerings.rs`).
- **`offerings/mod.rs` after S5 = thin CORE (~280L):** imports, the `Offering` struct, the
  visibility free fns (`is_cloud_resell`/`is_marketplace_visible`/`is_rentable_now`),
  `OFFERING_BASE_SELECT`, `SearchOfferingsParams`, and `mod`/`pub use` declarations for
  `tiers`/`read`/`write`/`analytics`/`recommendations`. (Plus `#[cfg(test)] mod tests;`.)
  **UNDER the 2k ceiling.**

**Acceptance:** block#3 fully gone from mod.rs; `db.get_offering_analytics`/`save_offering`/etc.
still resolve; the 4 response-struct paths still valid; build/clippy/nextest green; the
saved/analytics tests in `tests.rs` pass unchanged; **mod.rs ≤ ~300L**.

## Recommended order

```
S1 (foundation + tiers — proves the pattern) ──► S4 (recommendations — the prior "deferred" cluster; prove it's clean early)
                                             ──► S2 (read; 510L, self-contained)
                                             ──► S5 (analytics; 340L)
                                             ──► S3 (write; 1260L — last, biggest, pattern is proven)
```

S1 unblocks S2/S3/S4/S5. After S1, S2/S3/S4/S5 are **mutually independent** (star topology: each
depends only on CORE) and can be reordered or parallelized across branches. S4 is deliberately
**second** (not last) so the prior "deferred" verdict is disproven on real code before tackling the
large write cluster. S3 is last so the move pattern is well-established on smaller extractions.

## Final layout

```
api/src/database/offerings/
├── mod.rs             ~280L  CORE: Offering, OFFERING_BASE_SELECT, visibility fns, SearchOfferingsParams, mod/pub-use decls
├── tiers.rs           ~290L  OfferingTier/UnavailableTier/OfferingSuggestion + tier/suggestion free fns (block#0, independent)
├── read.rs            ~510L  impl Database block#1 (search/get/online-status)
├── write.rs          ~1260L  impl Database block#2 (CRUD/bulk/import) + opt_f64_changed
├── analytics.rs       ~340L  impl Database block#3-read (saved/views/analytics) + 4 response structs
├── recommendations.rs ~330L  impl Database block#3-recs + free fns + 4 structs + recommendation_tests
└── tests.rs          ~5571L  EXISTING external integration tests (unchanged; references resolve via re-exports)
```
All submodules ≤ ~1260L (write.rs) — every file under the 2k prod-code ceiling. `mod.rs` ≤ ~300L.

## Risks & design concerns

- **Re-export surface (highest-priority invariant).** External callers + `tests.rs` reach items by
  `crate::database::offerings::X`. mod.rs **must** `pub use` the moved `pub` items
  (`tiers::*`, `analytics::*`, `recommendations::RecommendedOffering`). Forgetting one is a
  compile error (loud, safe), not a silent break. Verify the full path set after each subtask with
  `rg 'crate::database::offerings::' api/src`.
- **Private items moving with their block.** `compute_provider_online_status` (block#1),
  `notify_saved_offering_price_change` (block#2), `parse_csv_record`/`import_offerings_csv_internal`
  (block#2), `fetch_*` (block#3) are private — they move **with** their block and stay
  module-private. Do NOT promote them to `pub` to "fix" a compile error; if one is referenced
  cross-block, the dependency analysis above is wrong and the subtask must stop and re-plan.
  (Verified: none are cross-block.)
- **`tests.rs` references tier fns by full path** (`crate::database::offerings::select_applicable_tiers`
  at L3339/3389/3436/3527, `default_compute_tiers`/`generate_suggestions` at L3481). The S1
  `pub use tiers::*;` re-export keeps these valid with **zero** edits to `tests.rs`. Double-check
  after S1.
- **`opt_f64_changed` ownership.** Verified used only by block#2 (L1199, 1208, 1665, 1666). Moves
  with block#2 in S3. If a later grep finds another caller, hoist to mod.rs instead.
- **`insert_notification` import.** Used only by block#2's `notify_saved_offering_price_change`.
  The `use crate::database::user_notifications::insert_notification;` import moves to `write.rs`
  in S3 (block#2), NOT to mod.rs core.
- **BackendType / region imports.** `is_cloud_resell` (CORE) uses `BackendType`; block#1 search
  uses `country_to_region`/`is_valid_country_code`; `region_to_country_code` (tiers) uses neither
  directly but lives in tiers.rs. Each submodule imports only what its cluster needs.
- **Pure relocation discipline.** No logic edits during moves — `create_offering`/`update_offering`
  are large and tempting to refactor; that is OUT OF SCOPE and belongs in a follow-up. Verify with
  `git diff --stat` (only moves) + spot-check that no body lines changed.
- **No spec-snapshot guard needed.** Unlike the `openapi/` splits, `database/offerings` has no
  `#[OpenApi]` surface. The verification bar is build + clippy(`--tests`) + nextest only.

## Verification checklist (per subtask, per repo `POST-CHANGE CHECKLIST`)

1. `cargo build -p api` clean.
2. `cargo clippy --tests -p api` → 0 warnings, 0 errors.
3. `cargo nextest run -p api database::offerings` → all green (~60 tests in `tests.rs` + 5 recs).
4. `rg 'crate::database::offerings::' api/src` → every external path still resolves (re-exports
   intact).
5. `git diff --stat` confirms: new submodule added, `offerings/mod.rs` shrinks by the expected
   line count, `database/mod.rs` unchanged (`mod offerings;` resolves to dir either way). No logic
   lines changed in moved bodies.

## Honest verdict — is the split worth doing?

**Yes, for `offerings.rs` (conditionally):** it is over the 2k prod ceiling (2827L prod), the
clusters are genuinely independent (star topology, verified), and the split mirrors an
**already-shipped** convention (`providers/`) — so the risk is low and the pattern is proven. The
payoff is real: `mod.rs` 2827L → ~280L, every submodule ≤ 1260L, and the previously-deferred
recommendations cluster becomes a normal extraction.

**Caveat (when NOT worth it):** if the team's actual pain is elsewhere, this is a pure-hygiene
refactor with no behavior change. It is lower priority than product work (real Hetzner offerings)
and should not preempt it. It is also **strictly less urgent** than the dc-agent split was,
because `offerings.rs` has no CLI/OpenAPI surface to drift — the only guard is the test suite,
which already exists and is comprehensive.

**Net recommendation:** schedule as a background tech-debt wave after any in-flight product work.
S1 + S4 alone (foundation + the prior "deferred" cluster) would retire the open question for ~1
day of effort; S2/S3/S5 finish the job. Do NOT bundle with feature work — the move-only discipline
is easier to review in isolation.

---

## §9 Secondary assessment — `api/src/openapi/providers.rs` (4325L)

The brief's "4082-line providers.rs" is this file (now 4325L), **not** a database file. It is a
**Path-A `#[OpenApi]` handler layer**, structurally different from the DB split above.

**Structure:** L1–515 free fns + small request/response DTOs (`validate_recipe_if_present`,
`validate_offering_currency`, the SSE handlers `password_reset_events`/`contract_status_events`,
`normalize_provisioning_details`, `validate_hetzner_offering_inner`/`validate_vultr_offering_inner`,
`apply_hetzner_catalog_specs`, `ProviderDashboardResponse`) → L516–3375 the **giant
`#[OpenApi] impl ProvidersApi { … }`** (~40 handler methods) → L3376–3429 more DTOs → L3444–4325
inline tests.

**Handler clustering (visible from the method list):**
- **Provider listing/profiles** (list/get_active/get_new_providers, get_provider_profile,
  get_provider_contacts add/delete, get_provider_dashboard) ~7 methods
- **Offering management** (get_provider_offerings, get_my_offerings, create/update/delete/duplicate,
  bulk_update_status/prices) ~8 methods
- **Contracts/provisioning/SSE** (get_provider_contracts, get_pending_provision/password_reset/
  ssh_key_rotation, complete_ssh_key_rotation, mark_contract_terminated, update_provisioning_status,
  update_contract_password) ~8 methods
- **Rental requests** (get_pending_rental_requests, respond_to_rental_request) ~2 methods
- **SLI reports** (upsert_provider_offering_sli_reports) ~1 method
- **Onboarding/helpcenter** (get/update_provider_onboarding, sync_provider_helpcenter) ~3 methods
- **Auto-accept rules** (get/set_auto_accept_rentals, list/create/update/delete_auto_accept_rule)
  ~5 methods
- **Bandwidth** (get_provider_bandwidth, get_contract_bandwidth) ~2 methods
- **Offering suggestions/recipes** (get_offering_suggestions, generate_offerings) ~2 methods
- **Reconcile** (reconcile_instances) ~1 method

**Verdict for `openapi/providers.rs`:** clusterable in principle, but **higher-risk than the DB
split** and **not recommended as a mechanical wave**:
1. It is a **`#[OpenApi]`** file — the repo's Path-A splits keep the generated `/api/v1/openapi`
   spec byte-identical (a `spec_snapshot`-style guard applies). Any handler move must preserve the
   spec exactly.
2. The clusters are **less clean** than `offerings.rs`'s: they share DTOs, auth-extraction
   helpers, and the SSE infra; `reconcile_instances` (271L) and `generate_offerings` (262L) are
   large interwoven methods.
3. It would need its **own dedicated design pass** (this document does not attempt it) — likely
   splitting `impl ProvidersApi` into multiple `*Api` types registered in the combined-API tuple
   (the repo convention: `PoolsApi` in `openapi/pools.rs`), which is a meaningfully different
   pattern from the DB submodule split.

**Recommendation:** leave `openapi/providers.rs` for a separate, dedicated plan. The DB
`offerings.rs` split (this document) is the higher-confidence, lower-risk, precedent-backed
target and should go first.

## Out of scope

- Any logic/behavior change to the moved functions (pure relocation).
- Splitting `openapi/providers.rs` (4325L) — Path-A, needs its own design pass (§9).
- Splitting `database/cloud_resources.rs` (2699L) — `#[cfg(test)] mod tests` begins ~L1092 → prod
  code ~1091L, **under the 2k ceiling** once tests excluded (per parent plan line 305). Not a
  target.
- Splitting `database/stats.rs` (1634L) — under ceiling.
