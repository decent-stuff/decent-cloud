/// Migration path verification tests
///
/// Migrations are applied identically in every context — production runtime and
/// tests — via the single `sqlx::migrate!("./migrations_pg")` macro:
/// - `api/src/database/core.rs` (`Database::new`) runs it against the live DB.
/// - `api/src/database/test_helpers.rs` (`setup_test_db`) runs it against the
///   ephemeral test template DB.
///
/// Both expand the same compile-time macro, so the test schema is guaranteed to
/// match the deployed schema, and adding a migration requires touching neither
/// file.
///
/// Note: We don't use #[sqlx::test] for these tests because:
/// - It requires DATABASE_URL environment variable to be set
/// - It doesn't integrate with our ephemeral PostgreSQL system (test_helpers.rs)
/// - The migration functionality is already exercised via setup_test_db(), which
///   runs the same `sqlx::migrate!` macro, providing equivalent coverage.
///
/// Test that the migration directory is resolved relative to crate root
#[tokio::test]
async fn test_migration_path_from_crate_root() {
    // Verify the migration path "./migrations_pg" is resolved correctly
    // relative to the crate root (api/Cargo.toml location)

    use std::path::Path;

    // The migrate!() macro should resolve "./migrations_pg" relative to CARGO_MANIFEST_DIR
    let migration_dir = format!("{}/migrations_pg", env!("CARGO_MANIFEST_DIR"));

    assert!(
        Path::new(&migration_dir).exists(),
        "Migration directory should exist at: {}",
        migration_dir
    );

    // Verify both migration files exist
    let schema_sql = format!("{}/001_schema.sql", migration_dir);
    let seed_sql = format!("{}/002_seed_data.sql", migration_dir);

    assert!(
        Path::new(&schema_sql).exists(),
        "Schema migration should exist"
    );
    assert!(
        Path::new(&seed_sql).exists(),
        "Seed data migration should exist"
    );
}

/// Test that the migration files contain expected baseline schema/seed content.
#[tokio::test]
async fn test_migration_baseline_content() {
    // A sanity check on two anchor migrations. Full migration execution is
    // verified by every test that calls setup_test_db(), which runs the same
    // sqlx::migrate!("./migrations_pg") macro used in production (core.rs).

    let migration_dir = format!("{}/migrations_pg", env!("CARGO_MANIFEST_DIR"));

    // Read migration files
    let schema_migration = std::fs::read_to_string(format!("{}/001_schema.sql", migration_dir))
        .expect("Schema migration should be readable");
    let seed_migration = std::fs::read_to_string(format!("{}/002_seed_data.sql", migration_dir))
        .expect("Seed migration should be readable");

    // Verify they contain expected content
    assert!(schema_migration.contains("CREATE TABLE sync_state"));
    assert!(schema_migration.contains("CREATE TABLE user_registrations"));
    assert!(schema_migration.contains("CREATE TABLE provider_registrations"));
    // sync_state INSERT is in schema migration as it's required for the schema to be functional
    assert!(schema_migration.contains("INSERT INTO sync_state"));

    // Seed data contains example provider offerings, not system config.
    // (Migration 053 removes these demo rows after seeding so the live catalog
    // is honest-empty — see docs/PRODUCT-DIRECTION.md F2. The 002 seed inserts
    // are intentionally left in place to avoid a checksum change on an
    // already-applied migration; 053 is the guarded cleanup.)
    assert!(seed_migration.contains("INSERT INTO provider_offerings"));
}

/// Helper test to verify sqlx-data.json files are properly generated
#[tokio::test]
async fn test_sqlx_offline_mode_data_exists() {
    // Verify that .sqlx/query-*.json files exist (offline mode support)
    // Note: .sqlx directory is at the workspace root, not crate root
    use std::fs;
    use std::path::Path;

    // Get workspace root (parent of crate manifest dir)
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_dir
        .parent()
        .expect("Crate should be inside workspace");
    let sqlx_dir = workspace_root.join(".sqlx");
    let sqlx_path = &sqlx_dir;

    assert!(
        sqlx_path.exists(),
        ".sqlx directory should exist for offline mode support"
    );

    let entries: Vec<_> = fs::read_dir(sqlx_dir)
        .expect("Should be able to read .sqlx directory")
        .filter_map(|e| e.ok())
        .collect();

    // Should have many query-*.json files (at least 10 from current codebase)
    let query_files = entries
        .iter()
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("query-") && n.ends_with(".json"))
                .unwrap_or(false)
        })
        .count();

    assert!(
        query_files >= 10,
        "Expected at least 10 sqlx query files, found {}",
        query_files
    );

    // Verify one file to ensure correct format
    if let Some(first_query) = entries.iter().find(|e| {
        e.path()
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("query-"))
            .unwrap_or(false)
    }) {
        let content =
            fs::read_to_string(first_query.path()).expect("Should be able to read query file");

        // Verify it's valid JSON with expected structure
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("Query file should be valid JSON");

        assert!(
            json.get("db_name").is_some(),
            "Query file should have db_name"
        );
        assert!(json.get("query").is_some(), "Query file should have query");
        assert!(
            json.get("describe").is_some(),
            "Query file should have describe"
        );
        assert!(json.get("hash").is_some(), "Query file should have hash");

        // Verify it's PostgreSQL data
        let db_name = json["db_name"].as_str().unwrap();
        assert_eq!(
            db_name, "PostgreSQL",
            "sqlx-data.json should be for PostgreSQL"
        );
    }
}
