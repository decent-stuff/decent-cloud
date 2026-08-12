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

/// Forcefully drop a database using PostgreSQL 13+ `WITH (FORCE)` (terminates
/// connections then drops). For templates, clears the template flag first since
/// `DROP DATABASE` refuses a marked template. Errors are logged loudly but
/// non-fatal: a failed cleanup of an *old* DB must not abort setup of the current one.
async fn drop_database_force(conn: &mut PgConnection, db_name: &str) {
    // Clear the template flag if set (DROP DATABASE refuses a marked template).
    let _ = sqlx::query(&format!(
        "UPDATE pg_database SET datistemplate = FALSE WHERE datname = '{}'",
        db_name
    ))
    .execute(&mut *conn)
    .await;

    // DROP DATABASE WITH (FORCE) — PG 13+ terminates backends then drops, atomically.
    if let Err(e) = sqlx::query(&format!(
        "DROP DATABASE IF EXISTS {} WITH (FORCE)",
        db_name
    ))
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

        if is_orphaned {
            // DROP DATABASE WITH (FORCE) terminates backends then drops, atomically.
            if let Err(e) =
                sqlx::query(&format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", db_name))
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
}

// ---------------------------------------------------------------------------
// DB POOL: recycled databases (eliminates ~2600 CREATE/DROP DATABASE per CI run)
// ---------------------------------------------------------------------------
//
// Each `cargo nextest` test runs in its own process. With ~2600 tests that means
// ~2600 CREATE DATABASE + DROP DATABASE operations, all serialized on the
// `pg_database` system catalog lock. This pool eliminates the hot-path DDL:
//
// 1. Pre-create `DB_POOL_SIZE` databases from the template (once per migration hash).
// 2. Each test process claims a slot via an atomic `mkdir` lock (with stale-PID
//    reclamation — same pattern as the ephemeral-PG lock).
// 3. Before each test: `TRUNCATE ... RESTART IDENTITY CASCADE` on all public tables
//    (per-table lock, fully parallel across different pool DBs — no catalog lock).
// 4. When the process exits, the slot becomes stale and the next process reclaims it.
//
// Pool DBs are named `test_pool_{hash}_{slot}` so they are tied to the migration
// hash; old-hash pool DBs are cleaned alongside old templates.

/// Number of pre-created databases in the pool. Covers nextest's default parallelism.
const DB_POOL_SIZE: usize = 16;

/// Claim a pool slot via an atomic `mkdir` lock. Returns the slot number, or `None`
/// if all slots are busy (caller falls back to per-test CREATE/DROP). The lock is
/// reclaimed automatically when the owning process dies (stale-PID detection).
fn claim_pool_slot() -> Option<usize> {
    let pid = std::process::id();
    for slot in 0..DB_POOL_SIZE {
        let lock_dir = format!("/tmp/dc_db_pool_{}.lock.d", slot);
        if std::fs::create_dir(&lock_dir).is_ok() {
            // Won the slot — write our PID inside for stale detection. Log (don't
            // ignore) a write failure: a missing pid file makes the slot look
            // permanently busy to later processes (read_to_string fails → skip),
            // which is a leak worth surfacing.
            if let Err(e) = std::fs::write(format!("{}/pid", lock_dir), pid.to_string()) {
                eprintln!(
                    "Warning: failed to write pid file for pool slot {}: {}",
                    slot, e
                );
            }
            return Some(slot);
        }
        // Slot taken — check if the owner is still alive.
        if let Ok(content) = std::fs::read_to_string(format!("{}/pid", lock_dir)) {
            if let Ok(owner_pid) = content.trim().parse::<u32>() {
                if !Path::new(&format!("/proc/{}", owner_pid)).exists() {
                    // Stale — reclaim atomically.
                    let _ = std::fs::remove_dir_all(&lock_dir);
                    if std::fs::create_dir(&lock_dir).is_ok() {
                        if let Err(e) =
                            std::fs::write(format!("{}/pid", lock_dir), pid.to_string())
                        {
                            eprintln!(
                                "Warning: failed to write pid file for reclaimed pool slot {}: {}",
                                slot, e
                            );
                        }
                        return Some(slot);
                    }
                }
            }
        }
    }
    None
}

/// Pool DB name: `test_pool_{migration_hash}_{slot}`. Tied to the migration hash
/// so old pool DBs are cleaned when the schema changes.
fn pool_db_name(template_name: &str, slot: usize) -> String {
    // Extract the hash suffix from the template name (template_test_db_{hash}).
    let hash = template_name
        .strip_prefix("template_test_db_")
        .unwrap_or("unknown");
    format!("test_pool_{}_{}", hash, slot)
}

/// Base advisory-lock key for exclusive access to a pool DB slot. Each slot adds
/// its index: `POOL_DB_ADVISORY_LOCK_BASE + slot` (slot ∈ 0..DB_POOL_SIZE, so it
/// fits in the low 32 bits). "DBPL" occupies the high 32 bits to avoid colliding
/// with the template-setup advisory key (`TEMPLATE_SETUP_ADVISORY_KEY`).
const POOL_DB_ADVISORY_LOCK_BASE: i64 = 0x4442_504C_0000_0000;

/// Try to take the session advisory lock for `slot` on the pool DB connection.
///
/// Returns `true` if acquired (caller proceeds to TRUNCATE + use the pool DB for
/// the test process's lifetime), or `false` if another live session already holds
/// it (caller releases its filesystem slot and falls back to a fresh per-test DB).
///
/// The lock is **session-scoped**: it is auto-released the moment the connection
/// closes (process exit, crash, or kill), so a process that dies never wedges the
/// slot — and a reclaimer is only allowed in once the prior occupant's connection
/// is truly gone. This is the quiescence guarantee that the filesystem slot lock's
/// `/proc/{pid}` liveness check cannot provide on its own.
///
/// Errors propagate as a panic (test failure) — a broken lock check must never
/// silently permit a race.
async fn try_acquire_pool_db_lock(pool: &PgPool, slot: usize) -> bool {
    let key = POOL_DB_ADVISORY_LOCK_BASE + slot as i64;
    sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
        .bind(key)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|e| panic!("pool DB advisory-lock check failed for slot {slot}: {e:#?}"))
}

/// Migration seed data that must survive a pool-DB TRUNCATE reset.
///
/// These rows are inserted by migration `001_schema.sql`. `reset_db_data()` truncates
/// all public tables (except `_sqlx_migrations`) then re-inserts them so tests that
/// depend on this seed data — e.g. `database::tests::test_database_basic_operations`,
/// which calls `get_last_sync_position()` → `SELECT ... FROM sync_state WHERE id = 1`
/// via `fetch_one` (panics if the row is missing) — keep working.
///
/// Stored as one entry per statement because `sqlx::query()` uses the Postgres extended
/// (prepared) protocol, which rejects multi-statement strings.
///
/// If a future migration adds seed data, update this constant; a failing test will
/// signal the need.
const MIGRATION_SEED_DATA_SQL: &[&str] = &[
    // sync_state: single row consumed by get_last_sync_position() (fetch_one).
    "INSERT INTO sync_state (id, last_position) VALUES (1, 0) ON CONFLICT (id) DO NOTHING",
    // invoice_sequence: current-year invoice counter (migration 039).
    "INSERT INTO invoice_sequence (id, year, next_number) \
     VALUES (1, EXTRACT(YEAR FROM NOW()), 1) ON CONFLICT (id) DO NOTHING",
    // receipt_sequence: receipt counter (migration 038).
    "INSERT INTO receipt_sequence (id, next_number) VALUES (1, 1) \
     ON CONFLICT (id) DO NOTHING",
];

/// Reset all data in the database by truncating every table in the `public` schema.
/// Resets sequences via `RESTART IDENTITY`. Uses `CASCADE` to handle FK constraints.
/// This is a fast data-only reset — no schema/index recreation, no catalog lock.
///
/// `_sqlx_migrations` is excluded (truncating it would break the migrator) and the
/// migration seed rows (see `MIGRATION_SEED_DATA_SQL`) are re-inserted afterwards so
/// tests that rely on them keep passing.
async fn reset_db_data(pool: &PgPool) {
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT tablename FROM pg_tables \
         WHERE schemaname = 'public' AND tablename != '_sqlx_migrations'",
    )
    .fetch_all(pool)
    .await
    .expect("Failed to list tables for TRUNCATE");

    if tables.is_empty() {
        return;
    }

    let table_list = tables
        .iter()
        .map(|t| format!("public.{}", t))
        .collect::<Vec<_>>()
        .join(", ");

    sqlx::query(&format!("TRUNCATE {} RESTART IDENTITY CASCADE", table_list))
        .execute(pool)
        .await
        .expect("Failed to TRUNCATE tables for pool DB reset");

    // Re-insert migration seed data wiped by the TRUNCATE above.
    for stmt in MIGRATION_SEED_DATA_SQL {
        sqlx::query(stmt)
            .execute(pool)
            .await
            .expect("Failed to re-insert migration seed data after TRUNCATE");
    }
}

/// Clean up stale pool databases from old migration hashes.
/// Drops all `test_pool_%` databases whose hash doesn't match the current template.
async fn cleanup_stale_pool_dbs(admin_pool: &PgPool, current_hash_prefix: &str) {
    let pool_dbs: Vec<String> = match sqlx::query_scalar(
        "SELECT datname FROM pg_database WHERE datname LIKE 'test_pool_%' AND datistemplate = FALSE",
    )
    .fetch_all(admin_pool)
    .await
    {
        Ok(dbs) => dbs,
        Err(e) => {
            eprintln!("Warning: Failed to query stale pool databases: {:#?}", e);
            return;
        }
    };

    let current_pattern = format!("test_pool_{}_", current_hash_prefix);
    for db_name in pool_dbs {
        if db_name.starts_with(&current_pattern) {
            continue; // belongs to the current migration hash — keep it
        }
        let mut conn = admin_pool.acquire().await.unwrap_or_else(|e| {
            panic!("Failed to acquire connection for pool cleanup: {:#?}", e)
        });
        drop_database_force(&mut conn, &db_name).await;
    }
}

/// Set up a test database with all migrations applied
///
/// Automatically starts an ephemeral PostgreSQL server if TEST_DATABASE_URL is not set.
/// Each test gets an isolated database.
///
/// Performance strategy (eliminates the ~2600 CREATE/DROP bottleneck):
/// - **Pool path (fast):** Claims a pre-created pool DB (one of `DB_POOL_SIZE` cloned
///   from the template) and resets data via `TRUNCATE`. No CREATE/DROP in the hot path.
/// - **Fallback path:** If all pool slots are busy, creates a fresh DB from the template.
pub async fn setup_test_db() -> Database {
    let base_url = get_postgres_url();

    // Ensure template database exists and is current
    let template_name = ensure_template_db(&base_url).await;

    // Extract the migration hash prefix for pool DB naming + stale cleanup.
    let hash_prefix = template_name
        .strip_prefix("template_test_db_")
        .unwrap_or("unknown");

    // Connect to postgres database (limit to 2 connections to avoid exhausting PostgreSQL)
    let admin_url = format!("{}/postgres", base_url);
    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await
        .expect("Failed to connect to PostgreSQL admin database");

    // Clean up stale databases once per 30-second window across all parallel processes.
    let epoch_window = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 30;
    if std::fs::create_dir(format!("/tmp/dc_test_cleanup_{}.lock.d", epoch_window)).is_ok() {
        cleanup_stale_test_dbs(&admin_pool).await;
        cleanup_stale_pool_dbs(&admin_pool, hash_prefix).await;
    }

    // ── Pool path (fast): claim a pre-created DB and TRUNCATE ──────────────
    if let Some(slot) = claim_pool_slot() {
        let db_name = pool_db_name(&template_name, slot);

        // Ensure the pool DB exists (create from template if missing — one-time per slot).
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)",
        )
        .bind(&db_name)
        .fetch_one(&admin_pool)
        .await
        .unwrap_or(false);

        let create_ok = if !exists {
            match sqlx::query(&format!(
                "CREATE DATABASE {} TEMPLATE {}",
                db_name, template_name
            ))
            .execute(&admin_pool)
            .await
            {
                Ok(_) => true,
                Err(e) => {
                    // CREATE failed — release slot, fall through to fallback.
                    let _ =
                        std::fs::remove_dir_all(format!("/tmp/dc_db_pool_{}.lock.d", slot));
                    eprintln!(
                        "Warning: pool DB creation failed for slot {}, falling back: {:#?}",
                        slot, e
                    );
                    false
                }
            }
        } else {
            true
        };

        admin_pool.close().await;

        if create_ok {
            // Connect to the pool DB, TRUNCATE all data, return.
            let test_url = format!("{}/{}", base_url, db_name);
            match PgPoolOptions::new()
                .max_connections(2)
                .connect(&test_url)
                .await
            {
                Ok(pool) => {
                    // DB-level mutual exclusion: a session advisory lock guarantees
                    // we are the only process using this pool DB. The filesystem slot
                    // lock above only distributes processes across slots — it cannot
                    // guarantee the pool DB's Postgres session is quiesced when a slot
                    // is reclaimed. A process killed mid-test (e.g. nextest slow-timeout
                    // SIGTERM) vanishes from `/proc` instantly while its DB session may
                    // still be tearing down, so a reclaimer could otherwise connect to
                    // the same DB and race the dying session — observed in production as
                    // `duplicate key ... Key (id)=(101) already exists` when two test
                    // processes (the `api` lib + the `api::bin/api-server` binary, which
                    // both compile `offerings::tests`) land on the same pool DB.
                    //
                    // `pg_try_advisory_lock` is session-scoped and auto-released on
                    // disconnect, so it frees *exactly* when the prior occupant's
                    // connection is gone — the quiescence guarantee `/proc` lacks.
                    if !try_acquire_pool_db_lock(&pool, slot).await {
                        // Pool DB is held by a live session — the filesystem slot lock
                        // raced. Release the slot and fall through to the per-test
                        // CREATE/DROP fallback. `pool` had no lock acquired, so dropping
                        // it just closes the connection we opened.
                        let _ = std::fs::remove_dir_all(format!(
                            "/tmp/dc_db_pool_{}.lock.d",
                            slot
                        ));
                        eprintln!(
                            "Warning: pool DB slot {} advisory-lock busy, falling back to per-test DB",
                            slot
                        );
                        drop(pool);
                    } else {
                        reset_db_data(&pool).await;
                        return Database { pool };
                    }
                }
                Err(e) => {
                    // Connection failed — release slot, fall through to fallback.
                    let _ = std::fs::remove_dir_all(format!(
                        "/tmp/dc_db_pool_{}.lock.d",
                        slot
                    ));
                    eprintln!(
                        "Warning: pool DB connection failed for slot {}, falling back: {:#?}",
                        slot, e
                    );
                }
            }
        }
    }

    // ── Fallback path: per-test CREATE/DROP (pool full or unavailable) ──────
    // Reconnect to admin (pool was closed above in the pool path).
    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&admin_url)
        .await
        .expect("Failed to reconnect to PostgreSQL admin database");

    let test_id = TEST_DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("test_db_{}_{}", std::process::id(), test_id);

    // Drop if exists with FORCE (terminates lingering connections then drops).
    sqlx::query(&format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", db_name))
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
