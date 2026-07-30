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
use sqlx::postgres::PgPoolOptions;
use sqlx::{Connection, PgConnection, PgPool};
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
    /// Data directory (path persists for stale cleanup by future test runs)
    data_dir: PathBuf,
    /// pg_ctl child process (reaped on drop to avoid zombies)
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
# Allow enough connections for concurrent test processes (8 threads x ~4 connections each)
max_connections = 200
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
            _process: process,
        })
    }
}

impl Drop for EphemeralPostgres {
    fn drop(&mut self) {
        // Reap the pg_ctl child process (it exits immediately after starting PostgreSQL).
        // Do NOT stop PostgreSQL here: with nextest, each test runs in its own process and the
        // first process to exit would kill the shared instance used by all parallel tests.
        // PostgreSQL is stopped during stale detection in start_or_wait_for_shared_postgres
        // when the TCP connection fails on a subsequent test run.
        self._process.wait().ok();
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

/// Calculate migration hash for versioning.
///
/// Derived from the compile-time `sqlx::migrate!` migrator (which auto-discovers
/// every `.sql` file in `migrations_pg/`), so this stays in sync with the
/// migrations directory automatically — no hand-maintained list of files. The
/// hash changes whenever a migration is added, removed, or its content (and thus
/// its checksum) changes, forcing the template database to be rebuilt.
fn migration_hash() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    let migrator = sqlx::migrate!("./migrations_pg");
    for migration in migrator.iter() {
        migration.version.hash(&mut hasher);
        migration.checksum.hash(&mut hasher);
    }
    format!("{:x}", hasher.finish())
}

/// Fixed advisory-lock key that serializes test-template setup across concurrent test
/// processes. Each `cargo nextest` test runs in its own process; without coordination they
/// all race to build the shared template DB. The lock guarantees exactly one process performs
/// the (slow) migration while the others block on the lock and then reuse the finished
/// template. A session advisory lock is auto-released on disconnect, so a crashed builder can
/// never wedge the lock (unlike the old fixed-timeout poll loop, which wedged forever).
const TEMPLATE_SETUP_ADVISORY_KEY: i64 = 0x4443_5F54_454D_504C; // "DC_TMPL"

/// Forcefully drop a database: terminate backends, clear the template flag, then DROP.
/// Used for stale/incomplete template DBs (same migration hash, never marked as template) and
/// for templates left behind by previous migration hashes. Errors are logged loudly but
/// non-fatal: a failed cleanup of an *old* template must not abort setup of the current one.
async fn drop_database_force(conn: &mut PgConnection, db_name: &str) {
    // Terminate any open connections (required before DROP can succeed).
    if let Err(e) = sqlx::query(&format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
        db_name
    ))
    .execute(&mut *conn)
    .await
    {
        eprintln!(
            "Warning: Failed to terminate connections to '{}': {:#?}",
            db_name, e
        );
    }

    // Clear the template flag (DROP DATABASE refuses a marked template).
    if let Err(e) = sqlx::query(&format!(
        "UPDATE pg_database SET datistemplate = FALSE WHERE datname = '{}'",
        db_name
    ))
    .execute(&mut *conn)
    .await
    {
        eprintln!(
            "Warning: Failed to clear template flag for '{}': {:#?}",
            db_name, e
        );
        return;
    }

    if let Err(e) = sqlx::query(&format!("DROP DATABASE IF EXISTS {}", db_name))
        .execute(&mut *conn)
        .await
    {
        eprintln!("Warning: Failed to drop database '{}': {:#?}", db_name, e);
    }
}

/// True iff a database named `template_name` exists and is marked `datistemplate = TRUE`.
async fn is_template_ready(conn: &mut PgConnection, template_name: &str) -> bool {
    let ready: Option<bool> =
        sqlx::query_scalar("SELECT datistemplate FROM pg_database WHERE datname = $1")
            .bind(template_name)
            .fetch_optional(&mut *conn)
            .await
            .expect("Failed to check template readiness");
    matches!(ready, Some(true))
}

/// Build (or rebuild) the template DB. The caller MUST hold `TEMPLATE_SETUP_ADVISORY_KEY`.
///
/// Cleans up templates from previous migration hashes, recovers a stale same-name
/// non-template DB (left by a process that crashed between `CREATE DATABASE` and the
/// mark-as-template step), creates the template, runs all migrations, and marks it.
async fn build_template(conn: &mut PgConnection, base_url: &str, template_name: &str) {
    // Clean up templates from previous migration hashes (only the lock-holder does this).
    let old_templates: Vec<String> = sqlx::query_scalar(
        "SELECT datname FROM pg_database WHERE datname LIKE 'template_test_db_%' AND datistemplate = TRUE",
    )
    .fetch_all(&mut *conn)
    .await
    .expect("Failed to query old templates");

    for old_template in old_templates {
        if old_template == template_name {
            continue; // is_template_ready was false, so this branch is unreachable; guard anyway.
        }
        drop_database_force(conn, &old_template).await;
    }

    // Recover from a stale/incomplete DB with our name (datistemplate = FALSE because
    // is_template_ready returned false above). Drop it so we recreate cleanly instead of
    // wedging like the old fixed-timeout poll loop did.
    let row_exists: Option<bool> =
        sqlx::query_scalar("SELECT true FROM pg_database WHERE datname = $1")
            .bind(template_name)
            .fetch_optional(&mut *conn)
            .await
            .expect("Failed to check template existence");
    if row_exists.is_some() {
        eprintln!(
            "Recovering stale/incomplete template DB '{}' (dropping + recreating)",
            template_name
        );
        drop_database_force(conn, template_name).await;
    }

    // Create the fresh template database.
    sqlx::query(&format!("CREATE DATABASE {}", template_name))
        .execute(&mut *conn)
        .await
        .expect("Failed to create template database");

    // Run all migrations on the template (single-connection pool to the template DB).
    // Same macro production uses (core.rs), so the test schema matches the deployed schema and
    // adding a migration no longer requires touching this file.
    let template_url = format!("{}/{}", base_url, template_name);
    let template_pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&template_url)
        .await
        .expect("Failed to connect to template database");

    sqlx::migrate!("./migrations_pg")
        .run(&template_pool)
        .await
        .expect("Template migrations failed");

    template_pool.close().await;

    // Mark as a template so future runs use the fast path and CREATE DATABASE ... TEMPLATE works.
    sqlx::query(&format!(
        "UPDATE pg_database SET datistemplate = TRUE WHERE datname = '{}'",
        template_name
    ))
    .execute(&mut *conn)
    .await
    .expect("Failed to mark database as template");
}

/// Ensure the shared template database exists, is migrated, and is marked as a template.
///
/// Concurrent-safe: a Postgres advisory lock serializes setup so exactly one process builds
/// the template (running all migrations — slow, ~tens of seconds) while the others block on
/// the lock and then reuse the finished template. Recovers from a stale/incomplete non-template
/// DB instead of wedging forever. With a persistent Postgres (CI self-hosted runner), the
/// fast path makes per-test setup ~100ms after the first run.
async fn ensure_template_db(base_url: &str) -> String {
    let template_name = format!("template_test_db_{}", migration_hash());

    // Process-local fast path: template already prepared earlier in this process.
    if let Some(existing) = TEMPLATE_INITIALIZED.get() {
        if existing == &template_name {
            return template_name;
        }
    }

    // Dedicated admin connection (to the `postgres` database). Held for the whole setup so the
    // advisory lock stays on this session until we explicitly release it.
    let admin_url = format!("{}/postgres", base_url);
    let mut admin_conn = PgConnection::connect(&admin_url)
        .await
        .expect("Failed to connect to PostgreSQL admin database");

    // Unlocked fast path: template is already ready — skip lock contention entirely.
    if is_template_ready(&mut admin_conn, &template_name).await {
        TEMPLATE_INITIALIZED.set(template_name.clone()).ok();
        return template_name;
    }

    // Slow path: serialize setup across all concurrent test processes.
    // pg_advisory_lock BLOCKS until acquired; auto-released on disconnect if this process dies.
    sqlx::query(&format!(
        "SELECT pg_advisory_lock({})",
        TEMPLATE_SETUP_ADVISORY_KEY
    ))
    .execute(&mut admin_conn)
    .await
    .expect("Failed to acquire template-setup advisory lock");

    // Re-check under the lock: another process may have finished while we waited.
    if !is_template_ready(&mut admin_conn, &template_name).await {
        build_template(&mut admin_conn, base_url, &template_name).await;
    }

    // Release so the next waiting process proceeds.
    if let Err(e) = sqlx::query(&format!(
        "SELECT pg_advisory_unlock({})",
        TEMPLATE_SETUP_ADVISORY_KEY
    ))
    .execute(&mut admin_conn)
    .await
    {
        panic!("Failed to release template-setup advisory lock: {:#?}", e);
    }

    // Invariant: the template must be ready now regardless of who built it.
    assert!(
        is_template_ready(&mut admin_conn, &template_name).await,
        "Template '{}' was not marked ready after setup",
        template_name
    );

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

            // Store in global static so the Child is reaped on process exit
            // This should succeed since we hold the lock
            match EPHEMERAL_PG.set(pg) {
                Ok(_) => {}
                Err(_) => panic!("Failed to store PostgreSQL instance in global static - already set by another thread"),
            }

            // Write env file for other processes (include data dir for stale cleanup)
            let pg_data_dir = EPHEMERAL_PG.get().unwrap().data_dir.join("data");
            let env_content = format!(
                "export TEST_DATABASE_URL=\"{}\"\nexport EPHEMERAL_PG_DATA_DIR=\"{}\"\n",
                url,
                pg_data_dir.display()
            );
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
                                eprintln!("Warning: Detected stale lock directory with dead PostgreSQL, cleaning up...");
                                // Stop PostgreSQL and remove its data directory
                                if let Ok(env_content) = std::fs::read_to_string(env_file) {
                                    for line in env_content.lines() {
                                        if let Some(data_dir) = line
                                            .strip_prefix("export EPHEMERAL_PG_DATA_DIR=\"")
                                            .and_then(|s| s.strip_suffix('"'))
                                        {
                                            let pg_bin_dir =
                                                find_postgres_bin_dir().unwrap_or_default();
                                            Command::new(EphemeralPostgres::pg_cmd(
                                                &pg_bin_dir,
                                                "pg_ctl",
                                            ))
                                            .args(["-D", data_dir, "stop", "-m", "immediate"])
                                            .stdout(Stdio::null())
                                            .stderr(Stdio::null())
                                            .status()
                                            .ok();
                                            if let Some(base_dir) = Path::new(data_dir).parent() {
                                                std::fs::remove_dir_all(base_dir).ok();
                                            }
                                            break;
                                        }
                                    }
                                }
                                std::fs::remove_dir_all(lock_dir).ok();
                                std::fs::remove_file(env_file).ok();
                                // Random backoff to avoid thundering herd (0-500ms)
                                let backoff_ms = rand_u32() % 500;
                                std::thread::sleep(std::time::Duration::from_millis(
                                    backoff_ms as u64,
                                ));
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

/// Clean up stale test databases from previous runs.
///
/// Drops any `test_db_*` databases that are not templates and not the current process's databases.
/// This prevents unbounded growth of the PostgreSQL data directory.
async fn cleanup_stale_test_dbs(admin_pool: &PgPool) {
    // Find all non-template test databases
    let stale_dbs: Vec<String> = match sqlx::query_scalar(
        "SELECT datname FROM pg_database WHERE datname LIKE 'test_db_%' AND datistemplate = FALSE",
    )
    .fetch_all(admin_pool)
    .await
    {
        Ok(dbs) => dbs,
        Err(e) => {
            eprintln!("Warning: Failed to query stale test databases: {:#?}", e);
            return;
        }
    };

    let current_prefix = format!("test_db_{}_", std::process::id());

    for db_name in stale_dbs {
        // Skip databases belonging to the current process
        if db_name.starts_with(&current_prefix) {
            continue;
        }

        // Check if the owning process is still alive by extracting PID from name
        // Format: test_db_{pid}_{counter}
        let is_orphaned = db_name
            .strip_prefix("test_db_")
            .and_then(|rest| rest.split('_').next())
            .and_then(|pid_str| pid_str.parse::<u32>().ok())
            .map(|pid| !Path::new(&format!("/proc/{}", pid)).exists())
            .unwrap_or(true); // If we can't parse the PID, assume orphaned

        if !is_orphaned {
            continue;
        }

        // Terminate connections to the stale database
        sqlx::query(&format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            db_name
        ))
        .execute(admin_pool)
        .await
        .ok();

        // Drop the stale database
        if let Err(e) = sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\"", db_name))
            .execute(admin_pool)
            .await
        {
            eprintln!(
                "Warning: Failed to drop stale test database '{}': {:#?}",
                db_name, e
            );
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

    // Connect to postgres database (limit to 2 connections to avoid exhausting PostgreSQL)
    let admin_url = format!("{}/postgres", base_url);
    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await
        .expect("Failed to connect to PostgreSQL admin database");

    // Clean up stale test databases once per 30-second window across all parallel processes.
    // CLEANUP_DONE AtomicBool is per-process (useless with nextest's per-test process isolation);
    // the filesystem lock ensures only one of the ~N concurrent test processes runs cleanup,
    // preventing N concurrent DROP DATABASE calls on the same stale databases.
    let epoch_window = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 30;
    if std::fs::create_dir(format!("/tmp/dc_test_cleanup_{}.lock.d", epoch_window)).is_ok() {
        cleanup_stale_test_dbs(&admin_pool).await;
    }

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

    // Connect to new test database (limit to 2 connections - tests are single-threaded)
    let test_url = format!("{}/{}", base_url, db_name);
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&test_url)
        .await
        .expect("Failed to connect to test database");

    Database { pool }
}
