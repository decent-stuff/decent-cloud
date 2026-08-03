//! sqlx offline-cache split-footgun guard.
//!
//! The build reads sqlx query plans from the **workspace-root** `.sqlx/`: sqlx
//! walks up from `CARGO_MANIFEST_DIR` (the `api/` package dir) until it finds a
//! `.sqlx/` directory, and the only committed one lives at the workspace root.
//!
//! The footgun: `cargo sqlx prepare` run from `api/` *without* `--workspace`
//! writes to a per-package `api/.sqlx/` instead. `.gitignore` hides that dir, so
//! a developer who edits a `query!`/`query_scalar!` and re-prepares from `api/`
//! sees their local build go green (sqlx finds the nearer `api/.sqlx/` first)
//! while the committed root cache silently goes stale — and the next fresh CI
//! clone fails with "no cached data for query".
//!
//! This test catches exactly that: `api/.sqlx/` MUST NOT exist. It deliberately
//! does NOT re-assert that the root cache exists/is populated — that is already
//! covered (non-overlapping) by
//! `database::migration_tests::test_sqlx_offline_mode_data_exists`. If this test
//! fails, delete `api/.sqlx/` and regenerate with `scripts/sqlx-prepare.sh`
//! (which always uses `--workspace` from the repo root).

#[test]
fn no_per_package_sqlx_cache_dir() {
    let api_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let api_sqlx = api_dir.join(".sqlx");

    assert!(
        !api_sqlx.exists(),
        "api/.sqlx/ exists at {} — this is the sqlx cache-split footgun. A bare \
         `cargo sqlx prepare` (without --workspace) wrote to the gitignored \
         per-package cache, which the build reads preferentially over the \
         committed workspace-root cache. Delete api/.sqlx/ and regenerate with \
         `scripts/sqlx-prepare.sh` so the root .sqlx/ stays the single source.",
        api_sqlx.display(),
    );
}
