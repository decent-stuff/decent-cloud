use anyhow::Result;
use sqlx::{Row, SqlitePool};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        sqlx::migrate!().run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn get_last_sync_position(&self) -> Result<u64> {
        let row = sqlx::query("SELECT last_position FROM sync_state WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("last_position") as u64)
    }

    pub async fn update_sync_position(&self, position: u64) -> Result<()> {
        sqlx::query("UPDATE sync_state SET last_position = ?, last_sync_at = CURRENT_TIMESTAMP WHERE id = 1")
            .bind(position as i64)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Test helper method to access the underlying pool for test assertions
    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn insert_entries(&self, entries: Vec<LedgerEntryData>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        for entry in entries {
            sqlx::query(
                "INSERT OR REPLACE INTO ledger_entries (label, key, value) VALUES (?, ?, ?)",
            )
            .bind(&entry.label)
            .bind(&entry.key)
            .bind(&entry.value)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct LedgerEntryData {
    pub label: String,
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}
