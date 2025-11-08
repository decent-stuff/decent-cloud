use anyhow::Result;
use sqlx::{Row, SqlitePool};
use super::types::Database;

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn get_last_sync_position(&self) -> Result<u64> {
        let row: sqlx::sqlite::SqliteRow = sqlx::query("SELECT last_position FROM sync_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("last_position") as u64)
    }

    pub async fn update_sync_position(&self, position: u64) -> Result<()> {
        sqlx::query("UPDATE sync_state SET last_position = ? WHERE id = 1")
            .bind(position as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Test helper method to access the underlying pool
    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
