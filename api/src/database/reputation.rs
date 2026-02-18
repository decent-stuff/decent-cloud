use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use borsh::BorshDeserialize;
use dcc_common::{cache_reputation::ReputationAge, cache_reputation::ReputationChange};

impl Database {
    // Reputation changes
    pub(crate) async fn insert_reputation_changes(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let change = ReputationChange::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation change: {}", e))?;

            let timestamp_i64 = entry.block_timestamp_ns as i64;
            let delta_amount = change
                .changes()
                .first()
                .map(|(_, delta)| *delta)
                .ok_or_else(|| anyhow::anyhow!("Reputation change has no entries"))?;

            sqlx::query!(
                "INSERT INTO reputation_changes (pubkey, change_amount, reason, block_timestamp_ns) VALUES ($1, $2, $3, $4)",
                entry.key,
                delta_amount,
                "", // Reason is not stored in structure, use empty string
                timestamp_i64 // Use actual block timestamp
            )

            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }

    // Reputation aging
    pub(crate) async fn insert_reputation_aging(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        entries: &[LedgerEntryData],
    ) -> Result<()> {
        for entry in entries {
            let age = ReputationAge::try_from_slice(&entry.value)
                .map_err(|e| anyhow::anyhow!("Failed to parse reputation age: {}", e))?;

            let timestamp_i64 = entry.block_timestamp_ns as i64;
            let aging_factor = age.reductions_ppm() as i64;

            sqlx::query!(
                "INSERT INTO reputation_aging (block_timestamp_ns, aging_factor_ppm) VALUES ($1, $2)",
                timestamp_i64,
                aging_factor
            )
            .execute(&mut **tx)
            .await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::test_helpers::setup_test_db;
    use dcc_common::{
        cache_reputation::ReputationAge, cache_reputation::ReputationChange,
        LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE,
    };
    use sqlx::Row;

    fn make_entry(label: &str, key: &[u8], value: &[u8], timestamp: u64) -> LedgerEntryData {
        LedgerEntryData {
            label: label.to_string(),
            key: key.to_vec(),
            value: value.to_vec(),
            block_timestamp_ns: timestamp,
            block_hash: vec![0; 3],
            block_offset: 0,
        }
    }

    #[tokio::test]
    async fn test_insert_reputation_change_positive() {
        let db = setup_test_db().await;
        let pubkey = b"rep_pos_key_1";
        let delta = 42i64;
        let timestamp = 1_000_000_000u64;

        let change = ReputationChange::new_single(pubkey.to_vec(), delta);
        let value = borsh::to_vec(&change).unwrap();

        let entries = vec![make_entry(LABEL_REPUTATION_CHANGE, pubkey, &value, timestamp)];
        db.insert_entries(entries).await.unwrap();

        let row = sqlx::query(
            "SELECT pubkey, change_amount, reason, block_timestamp_ns FROM reputation_changes",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();

        assert_eq!(row.get::<Vec<u8>, _>("pubkey"), pubkey.to_vec());
        assert_eq!(row.get::<i64, _>("change_amount"), delta);
        assert_eq!(row.get::<String, _>("reason"), "");
        assert_eq!(row.get::<i64, _>("block_timestamp_ns"), timestamp as i64);
    }

    #[tokio::test]
    async fn test_insert_reputation_change_negative() {
        let db = setup_test_db().await;
        let pubkey = b"rep_neg_key_1";
        let delta = -17i64;
        let timestamp = 2_000_000_000u64;

        let change = ReputationChange::new_single(pubkey.to_vec(), delta);
        let value = borsh::to_vec(&change).unwrap();

        let entries = vec![make_entry(LABEL_REPUTATION_CHANGE, pubkey, &value, timestamp)];
        db.insert_entries(entries).await.unwrap();

        let row = sqlx::query("SELECT change_amount FROM reputation_changes")
            .fetch_one(&db.pool)
            .await
            .unwrap();

        assert_eq!(row.get::<i64, _>("change_amount"), delta);
    }

    #[tokio::test]
    async fn test_insert_reputation_changes_batch() {
        let db = setup_test_db().await;

        let entries: Vec<LedgerEntryData> = (0..3)
            .map(|i| {
                let pubkey = format!("batch_key_{}", i);
                let delta = (i + 1) as i64 * 10;
                let change = ReputationChange::new_single(pubkey.as_bytes().to_vec(), delta);
                let value = borsh::to_vec(&change).unwrap();
                make_entry(
                    LABEL_REPUTATION_CHANGE,
                    pubkey.as_bytes(),
                    &value,
                    (i as u64 + 1) * 1_000_000_000,
                )
            })
            .collect();

        db.insert_entries(entries).await.unwrap();

        let rows = sqlx::query(
            "SELECT change_amount, block_timestamp_ns FROM reputation_changes ORDER BY block_timestamp_ns",
        )
        .fetch_all(&db.pool)
        .await
        .unwrap();

        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].get::<i64, _>("change_amount"), 10);
        assert_eq!(rows[1].get::<i64, _>("change_amount"), 20);
        assert_eq!(rows[2].get::<i64, _>("change_amount"), 30);
        assert_eq!(rows[0].get::<i64, _>("block_timestamp_ns"), 1_000_000_000);
        assert_eq!(rows[1].get::<i64, _>("block_timestamp_ns"), 2_000_000_000);
        assert_eq!(rows[2].get::<i64, _>("block_timestamp_ns"), 3_000_000_000);
    }

    #[tokio::test]
    async fn test_insert_reputation_change_malformed_borsh() {
        let db = setup_test_db().await;

        let entries = vec![make_entry(
            LABEL_REPUTATION_CHANGE,
            b"bad_key",
            b"not_valid_borsh",
            1_000_000_000,
        )];

        let result = db.insert_entries(entries).await;
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("reputation change"),
            "Error should mention reputation change, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_insert_reputation_aging_valid() {
        let db = setup_test_db().await;
        let ppm = 5_000u64; // 0.5% reduction
        let timestamp = 3_000_000_000u64;

        let age = ReputationAge::new(ppm);
        let value = borsh::to_vec(&age).unwrap();

        let entries = vec![make_entry(LABEL_REPUTATION_AGE, b"", &value, timestamp)];
        db.insert_entries(entries).await.unwrap();

        let row = sqlx::query(
            "SELECT block_timestamp_ns, aging_factor_ppm FROM reputation_aging",
        )
        .fetch_one(&db.pool)
        .await
        .unwrap();

        assert_eq!(row.get::<i64, _>("block_timestamp_ns"), timestamp as i64);
        assert_eq!(row.get::<i64, _>("aging_factor_ppm"), ppm as i64);
    }

    #[tokio::test]
    async fn test_insert_reputation_aging_malformed_borsh() {
        let db = setup_test_db().await;

        let entries = vec![make_entry(
            LABEL_REPUTATION_AGE,
            b"",
            b"garbage_data",
            1_000_000_000,
        )];

        let result = db.insert_entries(entries).await;
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("reputation aging"),
            "Error should mention reputation aging, got: {}",
            err_msg
        );
    }
}
