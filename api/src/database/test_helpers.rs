/// Shared test helpers for database tests
///
/// This module provides ephemeral PostgreSQL instances for testing.
/// It will automatically spin up a PostgreSQL server if one isn't available,
/// allowing `cargo nextest run` to work without any external setup.
///
/// Priority order for PostgreSQL connection:
/// 1. TEST_DATABASE_URL environment variable (external PostgreSQL)
/// 2. Ephemeral PostgreSQL server (started automatically)
use super::Database;
use sqlx::PgPool;
use std::io::Write;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Global ephemeral PostgreSQL instance shared across all tests in a process
static EPHEMERAL_PG: OnceLock<EphemeralPostgres> = OnceLock::new();

/// An ephemeral PostgreSQL server for testing
struct EphemeralPostgres {
    /// Base connection URL (without database name)
    url: String,
    /// Data directory (cleaned up on drop)
    data_dir: PathBuf,
    /// PostgreSQL server process
    _process: Child,
}

impl EphemeralPostgres {
    /// Start a new ephemeral PostgreSQL server
    fn start() -> Result<Self, String> {
        // Check if initdb is available
        if Command::new("initdb")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_err()
        {
            return Err("initdb not found - install PostgreSQL server".to_string());
        }

        // Use /dev/shm for speed if available, otherwise /tmp
        let base_dir = if std::path::Path::new("/dev/shm").exists() {
            "/dev/shm"
        } else {
            "/tmp"
        };

        // Create unique data directory
        let data_dir =
            PathBuf::from(base_dir).join(format!("pg_test_{}_{}", std::process::id(), rand_u32()));

        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        let pg_data = data_dir.join("data");
        let socket_dir = data_dir.clone();

        // Find a free port
        let port = find_free_port()?;

        // Initialize the database cluster
        let init_status = Command::new("initdb")
            .args([
                "-D",
                pg_data.to_str().unwrap(),
                "--no-locale",
                "--encoding=UTF8",
                "-A",
                "trust",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("initdb failed: {}", e))?;

        if !init_status.success() {
            return Err("initdb failed".to_string());
        }

        // Write optimized config for testing
        let conf_path = pg_data.join("postgresql.conf");
        let mut conf = std::fs::OpenOptions::new()
            .append(true)
            .open(&conf_path)
            .map_err(|e| format!("Failed to open postgresql.conf: {}", e))?;

        writeln!(
            conf,
            r#"
listen_addresses = '127.0.0.1'
port = {}
unix_socket_directories = '{}'
fsync = off
synchronous_commit = off
full_page_writes = off
"#,
            port,
            socket_dir.to_str().unwrap()
        )
        .map_err(|e| format!("Failed to write postgresql.conf: {}", e))?;

        // Start PostgreSQL
        let log_file = data_dir.join("postgres.log");
        let process = Command::new("pg_ctl")
            .args([
                "-D",
                pg_data.to_str().unwrap(),
                "-l",
                log_file.to_str().unwrap(),
                "-o",
                &format!("-k {}", socket_dir.to_str().unwrap()),
                "start",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start pg_ctl: {}", e))?;

        // Wait for PostgreSQL to be ready
        let url = format!("postgres://{}@127.0.0.1:{}", whoami(), port);
        wait_for_postgres(&url, 50)?;

        // Create the 'postgres' database for admin connections
        let create_status = Command::new("createdb")
            .args(["-h", "127.0.0.1", "-p", &port.to_string(), "postgres"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("createdb failed: {}", e))?;

        if !create_status.success() {
            return Err("Failed to create postgres database".to_string());
        }

        Ok(Self {
            url,
            data_dir,
            _process: process,
        })
    }
}

impl Drop for EphemeralPostgres {
    fn drop(&mut self) {
        // Stop PostgreSQL
        let pg_data = self.data_dir.join("data");
        let _ = Command::new("pg_ctl")
            .args(["-D", pg_data.to_str().unwrap(), "stop", "-m", "immediate"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        // Clean up data directory
        let _ = std::fs::remove_dir_all(&self.data_dir);
    }
}

/// Find a free TCP port
fn find_free_port() -> Result<u16, String> {
    let listener =
        TcpListener::bind("127.0.0.1:0").map_err(|e| format!("Failed to bind: {}", e))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("Failed to get local addr: {}", e))?
        .port();
    Ok(port)
}

/// Get current username
fn whoami() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "postgres".to_string())
}

/// Generate a random u32 for unique naming
fn rand_u32() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish() as u32
}

/// Wait for PostgreSQL to accept connections
fn wait_for_postgres(base_url: &str, max_attempts: u32) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("Failed to create runtime: {}", e))?;

    let url = format!("{}/postgres", base_url);

    for attempt in 0..max_attempts {
        match rt.block_on(async { PgPool::connect(&url).await }) {
            Ok(pool) => {
                rt.block_on(pool.close());
                return Ok(());
            }
            Err(_) => {
                if attempt < max_attempts - 1 {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
    }

    Err(format!(
        "PostgreSQL not ready after {} attempts",
        max_attempts
    ))
}

/// Get or start the ephemeral PostgreSQL server
fn get_postgres_url() -> String {
    // Check for external PostgreSQL first
    if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
        return url;
    }

    // Start or get ephemeral PostgreSQL
    let pg = EPHEMERAL_PG
        .get_or_init(|| EphemeralPostgres::start().expect("Failed to start ephemeral PostgreSQL"));

    pg.url.clone()
}

/// Set up a test database with all migrations applied
///
/// Automatically starts an ephemeral PostgreSQL server if TEST_DATABASE_URL is not set.
/// Each test gets a unique database that is isolated from other tests.
pub async fn setup_test_db() -> Database {
    let base_url = get_postgres_url();

    // Create a unique database name for this test
    let test_id = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("test_db_{}_{}", std::process::id(), test_id);

    // Connect to the postgres database to create our test database
    let admin_url = format!("{}/postgres", base_url);
    let admin_pool = PgPool::connect(&admin_url)
        .await
        .expect("Failed to connect to PostgreSQL admin database");

    // Drop the test database if it exists, then create it fresh
    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name))
        .execute(&admin_pool)
        .await
        .expect("Failed to drop existing test database");

    sqlx::query(&format!("CREATE DATABASE {}", db_name))
        .execute(&admin_pool)
        .await
        .expect("Failed to create test database");

    admin_pool.close().await;

    // Connect to the new test database and run migrations
    let test_url = format!("{}/{}", base_url, db_name);
    let pool = PgPool::connect(&test_url)
        .await
        .expect("Failed to connect to test database");

    // Run migrations from consolidated PostgreSQL files using raw_sql for multi-statement execution
    let migrations = [
        include_str!("../../migrations_pg/001_schema.sql"),
        include_str!("../../migrations_pg/002_seed_data.sql"),
    ];

    for migration in &migrations {
        sqlx::raw_sql(migration)
            .execute(&pool)
            .await
            .expect("Migration failed");
    }

    Database { pool }
}
