/// Shared test helpers for database tests
///
/// This module provides ephemeral PostgreSQL instances for testing.
///
/// **PostgreSQL Connection Priority:**
/// 1. `TEST_DATABASE_URL` environment variable (set by user or CI)
/// 2. `/tmp/ephemeral_pg_env.sh` (created by `cargo make postgres-start`)
/// 3. Auto-started ephemeral PostgreSQL (fallback for quick `cargo test` runs)
///
/// **Recommended usage:**
/// - Use `cargo make test` for full test suite (starts PostgreSQL once, reuses it)
/// - Use `cargo nextest run` for quick iteration (auto-starts PostgreSQL per process)
///
/// **Configuration:**
/// Both Makefile.toml and this module use identical PostgreSQL settings:
/// - `shared_buffers=4MB` - Balanced for concurrent operations without /dev/shm exhaustion
/// - `dynamic_shared_memory_type=mmap` - Use mmap instead of POSIX shm
/// - `fsync=off`, `synchronous_commit=off` - Speed optimizations for tests
use super::Database;
use sqlx::PgPool;
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

static TEST_DB_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Global ephemeral PostgreSQL instance shared across all tests in a process
static EPHEMERAL_PG: OnceLock<EphemeralPostgres> = OnceLock::new();

/// Global template database name (tracks current migration version)
static TEMPLATE_INITIALIZED: OnceLock<String> = OnceLock::new();

/// An ephemeral PostgreSQL server for testing
struct EphemeralPostgres {
    /// Base connection URL (without database name)
    url: String,
    /// Data directory (cleaned up on drop)
    data_dir: PathBuf,
    /// PostgreSQL binary directory
    pg_bin_dir: PathBuf,
    /// PostgreSQL server process
    _process: Child,
}

/// Find PostgreSQL binary directory by checking common installation paths
fn find_postgres_bin_dir() -> Option<PathBuf> {
    // Common PostgreSQL installation paths (ordered by preference)
    let search_paths = [
        // In PATH
        "",
        // Debian/Ubuntu standard locations
        "/usr/lib/postgresql/17/bin",
        "/usr/lib/postgresql/16/bin",
        "/usr/lib/postgresql/15/bin",
        "/usr/lib/postgresql/14/bin",
        // Red Hat/Fedora/CentOS standard locations
        "/usr/pgsql-17/bin",
        "/usr/pgsql-16/bin",
        "/usr/pgsql-15/bin",
        "/usr/pgsql-14/bin",
        // Homebrew on macOS
        "/opt/homebrew/opt/postgresql@17/bin",
        "/opt/homebrew/opt/postgresql@16/bin",
        "/usr/local/opt/postgresql@17/bin",
        "/usr/local/opt/postgresql@16/bin",
    ];

    for path_str in &search_paths {
        let path = if path_str.is_empty() {
            // Check if initdb is in PATH
            if Command::new("initdb")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
            {
                return Some(PathBuf::from(""));
            }
            continue;
        } else {
            PathBuf::from(path_str)
        };

        let initdb = path.join("initdb");
        if initdb.exists() && initdb.is_file() {
            return Some(path);
        }
    }

    None
}

impl EphemeralPostgres {
    /// Get full path to a PostgreSQL binary command
    fn pg_cmd(pg_bin_dir: &Path, cmd: &str) -> PathBuf {
        if pg_bin_dir.as_os_str().is_empty() {
            // Command is in PATH
            PathBuf::from(cmd)
        } else {
            pg_bin_dir.join(cmd)
        }
    }

    /// Start a new ephemeral PostgreSQL server
    fn start() -> Result<Self, String> {
        // Find PostgreSQL binaries - check common installation paths
        let pg_bin_dir = find_postgres_bin_dir()
            .ok_or_else(|| "PostgreSQL not found - install postgresql-server (Red Hat) or postgresql (Debian/Ubuntu)".to_string())?;

        // Use /tmp for PostgreSQL data (more space than /dev/shm which may be too small)
        let base_dir = "/tmp";

        // Create unique data directory
        let data_dir =
            PathBuf::from(base_dir).join(format!("pg_test_{}_{}", std::process::id(), rand_u32()));

        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Failed to create data directory: {}", e))?;

        let pg_data = data_dir.join("data");
        let socket_dir = data_dir.clone();

        // Find a free port
        let port = find_free_port()?;

        // Initialize the database cluster with minimal shared memory requirements
        // Use -c to set shared_buffers and other memory settings during bootstrap
        let init_output = Command::new(Self::pg_cmd(&pg_bin_dir, "initdb"))
            .args([
                "-D",
                pg_data.to_str().unwrap(),
                "--no-locale",
                "--encoding=UTF8",
                "-A",
                "trust",
                // Use small but sufficient shared memory during bootstrap
                "-c",
                "shared_buffers=4MB",
                "-c",
                "dynamic_shared_memory_type=mmap",
            ])
            .output()
            .map_err(|e| format!("initdb failed to run: {}", e))?;

        if !init_output.status.success() {
            // Best-effort cleanup of failed initialization (ignore errors)
            std::fs::remove_dir_all(&data_dir).ok();
            let stderr = String::from_utf8_lossy(&init_output.stderr);
            let stdout = String::from_utf8_lossy(&init_output.stdout);
            return Err(format!(
                "initdb failed (exit {}): stdout={}, stderr={}",
                init_output.status,
                stdout.trim(),
                stderr.trim()
            ));
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
# Balanced shared_buffers: large enough for concurrent operations, small enough for /tmp
# With 4MB we can handle ~100 concurrent CREATE DATABASE operations
shared_buffers = 4MB
# Use mmap instead of POSIX shared memory to avoid /dev/shm
dynamic_shared_memory_type = mmap
"#,
            port,
            socket_dir.to_str().unwrap()
        )
        .map_err(|e| format!("Failed to write postgresql.conf: {}", e))?;

        // Start PostgreSQL
        let log_file = data_dir.join("postgres.log");
        let process = Command::new(Self::pg_cmd(&pg_bin_dir, "pg_ctl"))
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
        wait_for_postgres(&pg_bin_dir, &url, 50)?;

        // Note: The 'postgres' database is automatically created by initdb,
        // so no need to create it explicitly.

        Ok(Self {
            url,
            data_dir,
            pg_bin_dir,
            _process: process,
        })
    }
}

impl Drop for EphemeralPostgres {
    fn drop(&mut self) {
        // Best-effort cleanup: stop PostgreSQL and remove data directory
        // Errors are expected if process already died or directory was removed
        let pg_data = self.data_dir.join("data");
        Command::new(Self::pg_cmd(&self.pg_bin_dir, "pg_ctl"))
            .args(["-D", pg_data.to_str().unwrap(), "stop", "-m", "immediate"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .ok();

        std::fs::remove_dir_all(&self.data_dir).ok();
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

/// Get current username using the `whoami` command for reliability
fn whoami() -> String {
    Command::new("whoami")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("USER").ok())
        .or_else(|| std::env::var("USERNAME").ok())
        .unwrap_or_else(|| "postgres".to_string())
}

/// Generate a random u32 for unique naming
fn rand_u32() -> u32 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    RandomState::new().build_hasher().finish() as u32
}

/// Wait for PostgreSQL to accept connections using pg_isready (synchronous)
fn wait_for_postgres(pg_bin_dir: &Path, base_url: &str, max_attempts: u32) -> Result<(), String> {
    // Parse host and port from postgres URL: postgres://user@host:port
    let url_without_scheme = base_url
        .strip_prefix("postgres://")
        .or_else(|| base_url.strip_prefix("postgresql://"))
        .ok_or_else(|| format!("Invalid PostgreSQL URL: {}", base_url))?;

    // Extract host:port (after @ if present)
    let host_port = url_without_scheme
        .split('@')
        .next_back()
        .ok_or_else(|| "Missing host in URL".to_string())?;

    let (host, port) = host_port
        .split_once(':')
        .ok_or_else(|| "Missing port in URL".to_string())?;

    for attempt in 0..max_attempts {
        let status = Command::new(EphemeralPostgres::pg_cmd(pg_bin_dir, "pg_isready"))
            .args(["-h", host, "-p", port])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        match status {
            Ok(exit_status) if exit_status.success() => return Ok(()),
            _ => {
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

/// Calculate migration hash for versioning
fn migration_hash() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    include_str!("../../migrations_pg/001_schema.sql").hash(&mut hasher);
    include_str!("../../migrations_pg/002_seed_data.sql").hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Ensure template database exists and is current
async fn ensure_template_db(base_url: &str) -> String {
    let template_name = format!("template_test_db_{}", migration_hash());

    // Check if already initialized in this process
    if let Some(existing) = TEMPLATE_INITIALIZED.get() {
        if existing == &template_name {
            return template_name;
        }
    }

    // Connect to postgres database
    let admin_url = format!("{}/postgres", base_url);
    let admin_pool = PgPool::connect(&admin_url)
        .await
        .expect("Failed to connect to PostgreSQL admin database");

    // Check if template exists and is properly marked
    let template_status: Option<bool> =
        sqlx::query_scalar("SELECT datistemplate FROM pg_database WHERE datname = $1")
            .bind(&template_name)
            .fetch_optional(&admin_pool)
            .await
            .expect("Failed to check template existence");

    let needs_creation = match template_status {
        Some(true) => {
            // Template exists and is properly marked
            false
        }
        Some(false) => {
            // Database exists but is NOT a template
            // This could be:
            // 1. A leftover from a failed run (should be cleaned up)
            // 2. Another process actively setting it up (should wait for it)
            //
            // CRITICAL: We must NOT clean up databases with the current migration hash
            // because another process might be running migrations on it right now.
            // Only mark as needs_creation=true, and let the CREATE DATABASE error
            // handling coordinate with the other process.
            //
            // Cleanup of truly stale databases happens in the old templates cleanup below.
            true
        }
        None => {
            // Database doesn't exist
            true
        }
    };

    if needs_creation {
        // Clean up old templates from previous migration versions
        let old_templates: Vec<String> = sqlx::query_scalar(
            "SELECT datname FROM pg_database WHERE datname LIKE 'template_test_db_%' AND datistemplate = TRUE"
        )
        .fetch_all(&admin_pool)
        .await
        .expect("Failed to query old templates");

        for old_template in old_templates {
            // Terminate connections to old template (best effort)
            match sqlx::query(&format!(
                "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
                old_template
            ))
            .execute(&admin_pool)
            .await
            {
                Ok(_) => {}
                Err(e) => eprintln!(
                    "Warning: Failed to terminate connections to old template '{}': {:#?}",
                    old_template, e
                ),
            }

            // Drop old template
            sqlx::query(&format!("DROP DATABASE IF EXISTS {}", old_template))
                .execute(&admin_pool)
                .await
                .unwrap_or_else(|e| {
                    panic!("Failed to drop old template '{}': {:#?}", old_template, e)
                });
        }

        // Try to create new template database
        let create_result = sqlx::query(&format!("CREATE DATABASE {}", template_name))
            .execute(&admin_pool)
            .await;

        // Handle creation - might fail if another process created it concurrently
        let created_by_us = match create_result {
            Ok(_) => true,
            Err(sqlx::Error::Database(ref db_err))
                if db_err.code().as_deref() == Some("42P04")
                    || db_err.code().as_deref() == Some("23505") =>
            {
                // Database already exists or unique constraint violated (concurrent creation)
                // Both error codes indicate another process is creating the template
                // Wait for it to be marked as a template (with timeout)
                for attempt in 0..100 {
                    let is_template: Option<bool> = sqlx::query_scalar(
                        "SELECT datistemplate FROM pg_database WHERE datname = $1",
                    )
                    .bind(&template_name)
                    .fetch_optional(&admin_pool)
                    .await
                    .expect("Failed to query template status");

                    if matches!(is_template, Some(true)) {
                        // Template is ready!
                        break;
                    } else if matches!(is_template, Some(false)) {
                        // Database exists but not yet marked as template - wait
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    } else {
                        // Database was dropped? Retry template creation
                        panic!("Template database disappeared during concurrent setup");
                    }

                    if attempt == 99 {
                        panic!(
                            "Timeout waiting for concurrent process to finish template setup for '{}'",
                            template_name
                        );
                    }
                }
                false // Created by another process
            }
            Err(e) => panic!("Failed to create template database: {:#?}", e),
        };

        // Only run migrations if we created the database
        if created_by_us {
            // Connect to template and run migrations
            let template_url = format!("{}/{}", base_url, template_name);
            let template_pool = PgPool::connect(&template_url)
                .await
                .expect("Failed to connect to template database");

            let migrations = [
                (
                    "001_schema.sql",
                    include_str!("../../migrations_pg/001_schema.sql"),
                ),
                (
                    "002_seed_data.sql",
                    include_str!("../../migrations_pg/002_seed_data.sql"),
                ),
            ];

            for (name, migration) in &migrations {
                sqlx::raw_sql(migration)
                    .execute(&template_pool)
                    .await
                    .unwrap_or_else(|e| panic!("Template migration failed for {}: {:#?}", name, e));
            }

            template_pool.close().await;

            // Mark as template
            sqlx::query(&format!(
                "UPDATE pg_database SET datistemplate = TRUE WHERE datname = '{}'",
                template_name
            ))
            .execute(&admin_pool)
            .await
            .expect("Failed to mark database as template");
        }
    }

    admin_pool.close().await;

    // Cache the template name (ok if another thread already set it)
    TEMPLATE_INITIALIZED.set(template_name.clone()).ok();

    template_name
}

/// Get or start the ephemeral PostgreSQL server
fn get_postgres_url() -> String {
    // Check for external PostgreSQL first (set by cargo make or user)
    // This takes precedence over everything else
    if let Ok(url) = std::env::var("TEST_DATABASE_URL") {
        // Strip database name if present (e.g., "postgres://user@host:port/dbname" -> "postgres://user@host:port")
        // The template creation code expects a base URL without a database name
        if let Some(pos) = url.rfind('/') {
            // Check if this is the database name (not part of the host)
            // URLs are in format: postgres://user@host:port or postgres://user@host:port/dbname
            let before_slash = &url[..pos];
            if before_slash.contains("://") && before_slash.contains('@') {
                // This is the database name separator, strip it
                return before_slash.to_string();
            }
        }
        return url;
    }

    // For all other cases (env file or auto-start), use the coordinated approach
    // which verifies PostgreSQL is actually running
    start_or_wait_for_shared_postgres()
}

/// Start PostgreSQL if we're first, or wait for another process to start it
fn start_or_wait_for_shared_postgres() -> String {
    use std::fs::File;
    use std::io::Write;

    let env_file = "/tmp/ephemeral_pg_env.sh";
    let lock_dir = "/tmp/ephemeral_pg_env.lock.d";

    // Try to acquire exclusive lock via atomic directory creation
    // Only one process succeeds - this is guaranteed atomic by the filesystem
    match std::fs::create_dir(lock_dir) {
        Ok(_) => {
            // We got the lock! Check if env file already exists from previous run
            if let Ok(content) = std::fs::read_to_string(env_file) {
                for line in content.lines() {
                    if let Some(url) = line.strip_prefix("export TEST_DATABASE_URL=\"") {
                        if let Some(url) = url.strip_suffix('"') {
                            // Verify PostgreSQL is actually running by trying to connect
                            use std::net::TcpStream;
                            if let Some(port_str) = url.split(':').next_back() {
                                if let Ok(port) = port_str.parse::<u16>() {
                                    if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                                        // PostgreSQL from previous run is still alive, reuse it
                                        return url.to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // No existing PostgreSQL or it's dead - start new one
            let pg = EphemeralPostgres::start().expect("Failed to start ephemeral PostgreSQL");
            let url = pg.url.clone();

            // Store in global static so Drop handler works
            // This should succeed since we hold the lock
            match EPHEMERAL_PG.set(pg) {
                Ok(_) => {}
                Err(_) => panic!("Failed to store PostgreSQL instance in global static - already set by another thread"),
            }

            // Write env file for other processes
            let env_content = format!("export TEST_DATABASE_URL=\"{}\"\n", url);
            let mut file = File::create(env_file).expect("Failed to create env file");
            file.write_all(env_content.as_bytes())
                .expect("Failed to write env file");
            file.sync_all().expect("Failed to sync env file");

            // Keep lock directory until process exits (don't remove it)
            // This ensures other processes know someone owns the PostgreSQL instance

            url
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            // Another process has the lock and is starting/has started PostgreSQL
            // Check if lock is stale (>10 seconds old with dead PostgreSQL)
            if let Ok(metadata) = std::fs::metadata(lock_dir) {
                if let Ok(created) = metadata.modified() {
                    if let Ok(elapsed) = created.elapsed() {
                        if elapsed.as_secs() > 10 {
                            // Lock is old - check if PostgreSQL is dead
                            let mut pg_dead = true;
                            if let Ok(content) = std::fs::read_to_string(env_file) {
                                for line in content.lines() {
                                    if let Some(url) =
                                        line.strip_prefix("export TEST_DATABASE_URL=\"")
                                    {
                                        if let Some(url) = url.strip_suffix('"') {
                                            use std::net::TcpStream;
                                            if let Some(port_str) = url.split(':').next_back() {
                                                if let Ok(port) = port_str.parse::<u16>() {
                                                    if TcpStream::connect(("127.0.0.1", port))
                                                        .is_ok()
                                                    {
                                                        pg_dead = false;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if pg_dead {
                                // PostgreSQL is dead and lock is stale - clean up and retry
                                eprintln!("Warning: Detected stale lock directory with dead PostgreSQL, cleaning up...");
                                std::fs::remove_dir_all(lock_dir).ok();
                                std::fs::remove_file(env_file).ok();
                                // Random backoff to avoid thundering herd (0-500ms)
                                let backoff_ms = rand_u32() % 500;
                                std::thread::sleep(std::time::Duration::from_millis(
                                    backoff_ms as u64,
                                ));
                                // Retry acquisition by recursing
                                return start_or_wait_for_shared_postgres();
                            }
                        }
                    }
                }
            }

            // Wait for env file to appear and PostgreSQL to be ready (up to 30 seconds)
            for _attempt in 0..300 {
                if let Ok(content) = std::fs::read_to_string(env_file) {
                    for line in content.lines() {
                        if let Some(url) = line.strip_prefix("export TEST_DATABASE_URL=\"") {
                            if let Some(url) = url.strip_suffix('"') {
                                // Verify it's actually running
                                use std::net::TcpStream;
                                if let Some(port_str) = url.split(':').next_back() {
                                    if let Ok(port) = port_str.parse::<u16>() {
                                        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
                                            return url.to_string();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            panic!(
                "Timed out waiting for shared PostgreSQL instance to start. \
                Check /tmp/ephemeral_pg_env.sh and lock directory."
            );
        }
        Err(e) => {
            panic!("Failed to create lock directory: {}", e);
        }
    }
}

/// Set up a test database with all migrations applied
///
/// Automatically starts an ephemeral PostgreSQL server if TEST_DATABASE_URL is not set.
/// Each test gets a unique database that is isolated from other tests.
///
/// Performance: Uses PostgreSQL template databases to avoid recreating schema/indexes
/// for every test. First test creates template (~6-10s), subsequent tests clone it (~0.5-1s).
pub async fn setup_test_db() -> Database {
    let base_url = get_postgres_url();

    // Ensure template database exists and is current
    let template_name = ensure_template_db(&base_url).await;

    // Create unique database name for this test
    let test_id = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("test_db_{}_{}", std::process::id(), test_id);

    // Connect to postgres database
    let admin_url = format!("{}/postgres", base_url);
    let admin_pool = PgPool::connect(&admin_url)
        .await
        .expect("Failed to connect to PostgreSQL admin database");

    // Drop if exists (cleanup from previous failed runs)
    sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name))
        .execute(&admin_pool)
        .await
        .expect("Failed to drop existing test database");

    // Clone from template (FAST! ~100ms vs 6-10s for full migration)
    sqlx::query(&format!(
        "CREATE DATABASE {} TEMPLATE {}",
        db_name, template_name
    ))
    .execute(&admin_pool)
    .await
    .expect("Failed to create test database from template");

    admin_pool.close().await;

    // Connect to new test database
    let test_url = format!("{}/{}", base_url, db_name);
    let pool = PgPool::connect(&test_url)
        .await
        .expect("Failed to connect to test database");

    Database { pool }
}
