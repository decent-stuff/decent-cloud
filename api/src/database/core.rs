use super::types::Database;
use anyhow::Result;
use sqlx::PgPool;

impl Database {
    /// Create a new Database connection and run migrations
    ///
    /// The `sqlx::migrate!()` macro uses a relative path "./migrations_pg" which is
    /// resolved relative to the crate root (CARGO_MANIFEST_DIR = api/ directory).
    /// This works correctly in both api-server runtime and cargo build/test contexts.
    ///
    /// Migrations are tracked in the __sqlx_migrations table to ensure each runs only once.
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;
        sqlx::migrate!("./migrations_pg").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn get_last_sync_position(&self) -> Result<u64> {
        let position: i64 =
            sqlx::query_scalar!("SELECT last_position FROM sync_state WHERE id = 1")
                .fetch_one(&self.pool)
                .await?;
        Ok(position as u64)
    }

    pub async fn update_sync_position(&self, position: u64) -> Result<()> {
        let position_i64 = position as i64;
        sqlx::query!(
            "UPDATE sync_state SET last_position = $1 WHERE id = 1",
            position_i64
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Access the underlying pool (needed for session store and testing)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
