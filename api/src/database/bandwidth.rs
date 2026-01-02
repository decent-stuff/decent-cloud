//! Bandwidth history database operations.

use super::Database;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A bandwidth history record (raw from database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthRecord {
    pub id: i64,
    pub contract_id: String,
    pub gateway_slug: String,
    pub provider_pubkey: String,
    pub bytes_in: i64,
    pub bytes_out: i64,
    pub recorded_at_ns: i64,
}

/// Aggregated bandwidth stats for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthStats {
    pub contract_id: String,
    pub gateway_slug: String,
    /// Latest bytes_in value
    pub bytes_in: u64,
    /// Latest bytes_out value
    pub bytes_out: u64,
    /// When stats were last updated
    pub last_updated_ns: i64,
}

impl Database {
    /// Record bandwidth stats from an agent heartbeat
    pub async fn record_bandwidth(
        &self,
        contract_id: &str,
        gateway_slug: &str,
        provider_pubkey: &str,
        bytes_in: u64,
        bytes_out: u64,
    ) -> Result<()> {
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as i64;

        // SQLite stores as i64
        let bytes_in_i64 = bytes_in as i64;
        let bytes_out_i64 = bytes_out as i64;

        sqlx::query!(
            r#"INSERT INTO bandwidth_history (contract_id, gateway_slug, provider_pubkey, bytes_in, bytes_out, recorded_at_ns)
               VALUES ($1, $2, $3, $4, $5, $6)"#,
            contract_id,
            gateway_slug,
            provider_pubkey,
            bytes_in_i64,
            bytes_out_i64,
            now_ns
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get latest bandwidth stats for a contract
    pub async fn get_contract_bandwidth(
        &self,
        contract_id: &str,
    ) -> Result<Option<BandwidthStats>> {
        let record = sqlx::query!(
            r#"SELECT
                id as "id!: i64",
                contract_id as "contract_id!: String",
                gateway_slug as "gateway_slug!: String",
                provider_pubkey as "provider_pubkey!: String",
                bytes_in as "bytes_in!: i64",
                bytes_out as "bytes_out!: i64",
                recorded_at_ns as "recorded_at_ns!: i64"
               FROM bandwidth_history
               WHERE contract_id = $1
               ORDER BY recorded_at_ns DESC
               LIMIT 1"#,
            contract_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|r| BandwidthStats {
            contract_id: r.contract_id,
            gateway_slug: r.gateway_slug,
            bytes_in: r.bytes_in as u64,
            bytes_out: r.bytes_out as u64,
            last_updated_ns: r.recorded_at_ns,
        }))
    }

    /// Get bandwidth stats for all contracts of a provider
    pub async fn get_provider_bandwidth_stats(
        &self,
        provider_pubkey: &str,
    ) -> Result<Vec<BandwidthStats>> {
        // Get latest stats for each contract using a subquery
        let records = sqlx::query!(
            r#"SELECT
                bh.id as "id!: i64",
                bh.contract_id as "contract_id!: String",
                bh.gateway_slug as "gateway_slug!: String",
                bh.provider_pubkey as "provider_pubkey!: String",
                bh.bytes_in as "bytes_in!: i64",
                bh.bytes_out as "bytes_out!: i64",
                bh.recorded_at_ns as "recorded_at_ns!: i64"
               FROM bandwidth_history bh
               INNER JOIN (
                   SELECT contract_id, MAX(recorded_at_ns) as max_ts
                   FROM bandwidth_history
                   WHERE provider_pubkey = $1
                   GROUP BY contract_id
               ) latest ON bh.contract_id = latest.contract_id AND bh.recorded_at_ns = latest.max_ts
               WHERE bh.provider_pubkey = $2"#,
            provider_pubkey,
            provider_pubkey
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| BandwidthStats {
                contract_id: r.contract_id,
                gateway_slug: r.gateway_slug,
                bytes_in: r.bytes_in as u64,
                bytes_out: r.bytes_out as u64,
                last_updated_ns: r.recorded_at_ns,
            })
            .collect())
    }

    /// Get bandwidth history for a contract (for graphing)
    pub async fn get_bandwidth_history(
        &self,
        contract_id: &str,
        limit: i64,
    ) -> Result<Vec<BandwidthRecord>> {
        let records = sqlx::query!(
            r#"SELECT
                id as "id!: i64",
                contract_id as "contract_id!: String",
                gateway_slug as "gateway_slug!: String",
                provider_pubkey as "provider_pubkey!: String",
                bytes_in as "bytes_in!: i64",
                bytes_out as "bytes_out!: i64",
                recorded_at_ns as "recorded_at_ns!: i64"
               FROM bandwidth_history
               WHERE contract_id = $1
               ORDER BY recorded_at_ns DESC
               LIMIT $2"#,
            contract_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(records
            .into_iter()
            .map(|r| BandwidthRecord {
                id: r.id,
                contract_id: r.contract_id,
                gateway_slug: r.gateway_slug,
                provider_pubkey: r.provider_pubkey,
                bytes_in: r.bytes_in,
                bytes_out: r.bytes_out,
                recorded_at_ns: r.recorded_at_ns,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;

    #[tokio::test]
    async fn test_record_and_get_bandwidth() {
        let db = setup_test_db().await;

        // Record some bandwidth
        db.record_bandwidth("contract-1", "abc123", "provider-pub", 1000, 2000)
            .await
            .unwrap();

        // Get it back
        let stats = db.get_contract_bandwidth("contract-1").await.unwrap();
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.contract_id, "contract-1");
        assert_eq!(stats.gateway_slug, "abc123");
        assert_eq!(stats.bytes_in, 1000);
        assert_eq!(stats.bytes_out, 2000);
    }

    #[tokio::test]
    async fn test_get_latest_bandwidth() {
        let db = setup_test_db().await;

        // Record multiple entries
        db.record_bandwidth("contract-1", "abc123", "provider-pub", 1000, 2000)
            .await
            .unwrap();

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        db.record_bandwidth("contract-1", "abc123", "provider-pub", 5000, 10000)
            .await
            .unwrap();

        // Should get the latest
        let stats = db
            .get_contract_bandwidth("contract-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(stats.bytes_in, 5000);
        assert_eq!(stats.bytes_out, 10000);
    }

    #[tokio::test]
    async fn test_get_provider_bandwidth_stats() {
        let db = setup_test_db().await;

        // Record bandwidth for multiple contracts
        db.record_bandwidth("contract-1", "abc123", "provider-1", 1000, 2000)
            .await
            .unwrap();
        db.record_bandwidth("contract-2", "def456", "provider-1", 3000, 4000)
            .await
            .unwrap();
        db.record_bandwidth("contract-3", "ghi789", "provider-2", 5000, 6000)
            .await
            .unwrap();

        // Get stats for provider-1
        let stats = db.get_provider_bandwidth_stats("provider-1").await.unwrap();
        assert_eq!(stats.len(), 2);
    }

    #[tokio::test]
    async fn test_bandwidth_history() {
        let db = setup_test_db().await;

        // Record multiple entries
        for i in 0..5 {
            db.record_bandwidth("contract-1", "abc123", "provider-pub", i * 1000, i * 2000)
                .await
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        // Get history (limited to 3)
        let history = db.get_bandwidth_history("contract-1", 3).await.unwrap();
        assert_eq!(history.len(), 3);
        // Should be in descending order (newest first)
        assert!(history[0].bytes_in > history[1].bytes_in);
    }
}
