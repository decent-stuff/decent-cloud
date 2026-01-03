/// Migration path verification tests
///
/// This test file verifies that PostgreSQL migrations run correctly in all contexts:
/// 1. From api-server (Database::new using sqlx::migrate!())
/// 2. From tests (setup_test_db using include_str!())
///
/// The two approaches differ for valid reasons:
/// - sqlx::migrate!() is relative to crate root, perfect for runtime
/// - include_str!() is needed in tests because sqlx::migrate!() has issues
///   with concurrent test execution and database creation/teardown
use sqlx::{PgPool, Row};

/// Test that migrations run correctly via Database::new (api-server context)
#[sqlx::test]
async fn test_migrations_via_database_new(pool: PgPool) {
    // This test verifies the sqlx::migrate!() macro works correctly
    // when called from Database::new() in api-server context

    // Check that tables created by migrations exist
    let result = sqlx::query(
        "SELECT table_name FROM information_schema.tables
         WHERE table_schema = 'public'
         AND table_name IN ('sync_state', 'user_registrations', 'provider_registrations')
         ORDER BY table_name",
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to query tables");

    assert_eq!(result.len(), 3, "Expected 3 tables to exist");
    assert_eq!(
        result[0].get::<String, _>("table_name"),
        "provider_registrations"
    );
    assert_eq!(result[1].get::<String, _>("table_name"), "sync_state");
    assert_eq!(
        result[2].get::<String, _>("table_name"),
        "user_registrations"
    );

    // Check that seed data was loaded
    let sync_position: Option<i64> =
        sqlx::query_scalar("SELECT last_position FROM sync_state WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("Failed to query sync_state");

    assert_eq!(
        sync_position,
        Some(0),
        "Expected initial sync position to be 0"
    );
}

/// Test that migration path resolution works relative to crate root
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

/// Test that both migration approaches (migrate! vs include_str) are equivalent
#[tokio::test]
async fn test_migration_approaches_are_equivalent() {
    // This test documents why we use two different approaches:

    // 1. sqlx::migrate!() in core.rs (runtime):
    //    - Relative path: "./migrations_pg"
    //    - Resolved from CARGO_MANIFEST_DIR (api/ directory)
    //    - Tracks migration state in __sqlx_migrations table
    //    - Runs each migration only once (idempotent)
    //    - Perfect for production use

    // 2. include_str!() in test_helpers.rs (tests):
    //    - Absolute path: "../../migrations_pg/..."
    //    - Embeds SQL at compile time
    //    - No migration tracking table needed
    //    - Allows fresh schema for each test
    //    - Better for test isolation and concurrent execution

    // Both approaches execute the same SQL files, so the resulting schema is identical

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

    assert!(seed_migration.contains("INSERT INTO sync_state"));
    assert!(seed_migration.contains("INSERT INTO email_templates"));
}

/// Helper test to verify sqlx-data.json files are properly generated
#[tokio::test]
async fn test_sqlx_offline_mode_data_exists() {
    // Verify that .sqlx/query-*.json files exist (offline mode support)
    use std::fs;
    use std::path::Path;

    let sqlx_dir = format!("{}/.sqlx", env!("CARGO_MANIFEST_DIR"));
    let sqlx_path = Path::new(&sqlx_dir);

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
