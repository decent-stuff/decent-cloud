use super::types::Database;
use anyhow::Result;
use sqlx::SqlitePool;

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn get_last_sync_position(&self) -> Result<u64> {
        let position = sqlx::query_scalar!("SELECT last_position FROM sync_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(position as u64)
    }

    pub async fn update_sync_position(&self, position: u64) -> Result<()> {
        let position_i64 = position as i64;
        sqlx::query!(
            "UPDATE sync_state SET last_position = ? WHERE id = 1",
            position_i64
        )
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
