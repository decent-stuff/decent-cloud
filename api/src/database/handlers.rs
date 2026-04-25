use super::types::{Database, LedgerEntryData};
use anyhow::Result;
use dcc_common::{
    LABEL_CONTRACT_SIGN_REPLY_LEGACY, LABEL_CONTRACT_SIGN_REQUEST_LEGACY, LABEL_DC_TOKEN_APPROVAL,
    LABEL_DC_TOKEN_TRANSFER, LABEL_NP_CHECK_IN, LABEL_NP_OFFERING_LEGACY, LABEL_NP_PROFILE_LEGACY,
    LABEL_NP_REGISTER, LABEL_PROV_CHECK_IN, LABEL_PROV_OFFERING_LEGACY, LABEL_PROV_PROFILE_LEGACY,
    LABEL_PROV_REGISTER, LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE, LABEL_REWARD_DISTRIBUTION,
    LABEL_USER_REGISTER,
};
use std::collections::HashMap;

impl Database {
    pub async fn insert_entries(&self, entries: Vec<LedgerEntryData>) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Group entries by label for batch processing
        let mut grouped_entries: HashMap<String, Vec<LedgerEntryData>> = HashMap::new();
        for entry in entries {
            grouped_entries
                .entry(entry.label.clone())
                .or_default()
                .push(entry);
        }

        // Process known labels in the defined order, to ensure foreign key constraints are met
        let known_labels = [
            LABEL_REPUTATION_CHANGE,
            LABEL_REPUTATION_AGE,
            LABEL_DC_TOKEN_TRANSFER,
            LABEL_DC_TOKEN_APPROVAL,
            LABEL_PROV_REGISTER,
            LABEL_NP_REGISTER,
            LABEL_PROV_CHECK_IN,
            LABEL_NP_CHECK_IN,
            LABEL_PROV_PROFILE_LEGACY,
            LABEL_NP_PROFILE_LEGACY,
            LABEL_USER_REGISTER,
            LABEL_PROV_OFFERING_LEGACY,
            LABEL_NP_OFFERING_LEGACY,
            LABEL_REWARD_DISTRIBUTION,
            LABEL_CONTRACT_SIGN_REQUEST_LEGACY,
            LABEL_CONTRACT_SIGN_REPLY_LEGACY,
        ];

        for label in known_labels {
            if let Some(entries) = grouped_entries.remove(label) {
                match label {
                    LABEL_REPUTATION_CHANGE => {
                        self.insert_reputation_changes(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert reputation changes: {}", e)
                            })?;
                    }
                    LABEL_REPUTATION_AGE => {
                        self.insert_reputation_aging(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert reputation aging: {}", e)
                            })?;
                    }
                    LABEL_DC_TOKEN_TRANSFER => {
                        self.insert_token_transfers(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert token transfers: {}", e)
                            })?;
                    }
                    LABEL_DC_TOKEN_APPROVAL => {
                        self.insert_token_approvals(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert token approvals: {}", e)
                            })?;
                    }
                    LABEL_PROV_REGISTER | LABEL_NP_REGISTER => {
                        self.insert_provider_registrations(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert provider registrations: {}", e)
                            })?;
                    }
                    LABEL_PROV_CHECK_IN | LABEL_NP_CHECK_IN => {
                        self.insert_provider_check_ins(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert provider check-ins: {}", e)
                            })?;
                    }
                    LABEL_USER_REGISTER => {
                        self.insert_user_registrations(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert user registrations: {}", e)
                            })?;
                    }
                    LABEL_PROV_OFFERING_LEGACY
                    | LABEL_NP_OFFERING_LEGACY
                    | LABEL_PROV_PROFILE_LEGACY
                    | LABEL_NP_PROFILE_LEGACY
                    | LABEL_CONTRACT_SIGN_REQUEST_LEGACY
                    | LABEL_CONTRACT_SIGN_REPLY_LEGACY => {
                        tracing::debug!(
                            "Skipping ledger {} entries - now handled directly in DB",
                            label
                        );
                    }
                    LABEL_REWARD_DISTRIBUTION => {
                        self.insert_reward_distributions(&mut tx, &entries)
                            .await
                            .map_err(|e| {
                                anyhow::anyhow!("Failed to insert reward distributions: {}", e)
                            })?;
                    }
                    _ => {
                        tracing::error!("Unexpected label in known_labels: {}", label);
                        anyhow::bail!("Unexpected label in known_labels: {}", label);
                    }
                }
            }
        }

        for (label, entries) in grouped_entries {
            tracing::error!(
                "Unknown and unhandled ledger entry label: {} with {} entries",
                label,
                entries.len()
            );
        }

        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::database::test_helpers::setup_test_db;
    use crate::database::Database;
    use crate::database::LedgerEntryData;
    use dcc_common::{
        LABEL_CONTRACT_SIGN_REPLY_LEGACY, LABEL_CONTRACT_SIGN_REQUEST_LEGACY, LABEL_NP_CHECK_IN,
        LABEL_NP_OFFERING_LEGACY, LABEL_NP_PROFILE_LEGACY, LABEL_NP_REGISTER, LABEL_PROV_CHECK_IN,
        LABEL_PROV_OFFERING_LEGACY, LABEL_PROV_PROFILE_LEGACY, LABEL_PROV_REGISTER,
        LABEL_REPUTATION_CHANGE, LABEL_USER_REGISTER,
    };

    fn make_entry(
        label: &str,
        key: &[u8],
        value: &[u8],
        timestamp: u64,
        offset: u64,
    ) -> LedgerEntryData {
        LedgerEntryData {
            label: label.to_string(),
            key: key.to_vec(),
            value: value.to_vec(),
            block_timestamp_ns: timestamp,
            block_hash: vec![offset as u8; 3],
            block_offset: offset,
        }
    }

    async fn count_rows(db: &Database, table: &str) -> i64 {
        use sqlx::Row;
        if table.starts_with("provider_") {
            let example_pubkey = Database::example_provider_pubkey();
            let sql = format!("SELECT COUNT(*) as count FROM {} WHERE pubkey != $1", table);
            let row = sqlx::query(&sql)
                .bind(example_pubkey)
                .fetch_one(&db.pool)
                .await
                .unwrap();
            row.get("count")
        } else {
            let sql = format!("SELECT COUNT(*) as count FROM {}", table);
            let row = sqlx::query(&sql).fetch_one(&db.pool).await.unwrap();
            row.get("count")
        }
    }

    #[tokio::test]
    async fn test_insert_entries_legacy_labels_skipped() {
        let db = setup_test_db().await;

        let legacy_labels = [
            LABEL_PROV_PROFILE_LEGACY,
            LABEL_NP_PROFILE_LEGACY,
            LABEL_PROV_OFFERING_LEGACY,
            LABEL_NP_OFFERING_LEGACY,
            LABEL_CONTRACT_SIGN_REQUEST_LEGACY,
            LABEL_CONTRACT_SIGN_REPLY_LEGACY,
        ];

        let entries: Vec<LedgerEntryData> = legacy_labels
            .iter()
            .enumerate()
            .map(|(i, label)| {
                make_entry(
                    label,
                    b"legacy_key",
                    b"legacy_value",
                    1_000_000_000 + i as u64,
                    i as u64,
                )
            })
            .collect();

        let result = db.insert_entries(entries).await;
        assert!(
            result.is_ok(),
            "Legacy labels should be accepted without error"
        );

        let tables = [
            "provider_registrations",
            "provider_check_ins",
            "user_registrations",
            "token_transfers",
            "token_approvals",
            "reputation_changes",
            "reputation_aging",
            "reward_distributions",
        ];
        for table in &tables {
            assert_eq!(
                count_rows(&db, table).await,
                0,
                "Legacy label entries should not insert into {}",
                table
            );
        }
    }

    #[tokio::test]
    async fn test_insert_entries_np_register() {
        let db = setup_test_db().await;

        let entries = vec![make_entry(
            LABEL_NP_REGISTER,
            b"np_provider_key",
            b"np_signature",
            1_000_000_000,
            1,
        )];
        db.insert_entries(entries).await.unwrap();

        assert_eq!(
            count_rows(&db, "provider_registrations").await,
            1,
            "NP_REGISTER should dispatch to provider_registrations"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_np_check_in() {
        let db = setup_test_db().await;

        let payload = dcc_common::CheckInPayload::new("np_check_in".to_string(), vec![1, 2, 3, 4]);
        let entries = vec![make_entry(
            LABEL_NP_CHECK_IN,
            b"np_provider_key",
            &payload.to_bytes().unwrap(),
            2_000_000_000,
            1,
        )];
        db.insert_entries(entries).await.unwrap();

        assert_eq!(
            count_rows(&db, "provider_check_ins").await,
            1,
            "NP_CHECK_IN should dispatch to provider_check_ins"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_mixed_labels_in_single_batch() {
        let db = setup_test_db().await;

        let provider_key = b"mixed_batch_provider";
        let user_key = b"mixed_batch_user";
        let check_in_payload =
            dcc_common::CheckInPayload::new("mixed_check_in".to_string(), vec![1, 2, 3, 4]);

        let entries = vec![
            make_entry(
                LABEL_PROV_REGISTER,
                provider_key,
                b"sig_prov",
                1_000_000_000,
                1,
            ),
            make_entry(LABEL_USER_REGISTER, user_key, b"sig_user", 1_100_000_000, 2),
            make_entry(
                LABEL_PROV_CHECK_IN,
                provider_key,
                &check_in_payload.to_bytes().unwrap(),
                1_200_000_000,
                3,
            ),
        ];
        db.insert_entries(entries).await.unwrap();

        assert_eq!(count_rows(&db, "provider_registrations").await, 1);
        assert_eq!(count_rows(&db, "user_registrations").await, 1);
        assert_eq!(count_rows(&db, "provider_check_ins").await, 1);
    }

    #[tokio::test]
    async fn test_insert_entries_label_grouping_same_batch() {
        let db = setup_test_db().await;

        let provider_key = b"group_test_provider";
        let entries: Vec<LedgerEntryData> = (0..3)
            .map(|i| {
                let payload =
                    dcc_common::CheckInPayload::new(format!("check_{}", i), vec![i as u8; 4]);
                make_entry(
                    LABEL_PROV_CHECK_IN,
                    provider_key,
                    &payload.to_bytes().unwrap(),
                    1_000_000_000 + i * 1_000_000,
                    i,
                )
            })
            .collect();

        db.insert_entries(entries).await.unwrap();

        assert_eq!(
            count_rows(&db, "provider_check_ins").await,
            3,
            "Multiple entries with same label should all be inserted"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_unknown_labels_commit_without_error() {
        let db = setup_test_db().await;

        let entries = vec![
            make_entry("TotallyUnknownLabel", b"k1", b"v1", 1_000_000_000, 1),
            make_entry("AlsoUnknown", b"k2", b"v2", 2_000_000_000, 2),
        ];

        let result = db.insert_entries(entries).await;
        assert!(
            result.is_ok(),
            "Unknown labels should not cause insert_entries to fail; transaction should still commit"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_malformed_reputation_change_fails_transaction() {
        let db = setup_test_db().await;

        let provider_key = b"tx_fail_provider";
        let check_in_payload =
            dcc_common::CheckInPayload::new("valid_check_in".to_string(), vec![1, 2, 3, 4]);

        let entries = vec![
            make_entry(LABEL_PROV_REGISTER, provider_key, b"sig", 1_000_000_000, 1),
            make_entry(
                LABEL_PROV_CHECK_IN,
                provider_key,
                &check_in_payload.to_bytes().unwrap(),
                1_100_000_000,
                2,
            ),
            make_entry(
                LABEL_REPUTATION_CHANGE,
                b"rep_key",
                b"invalid_borsh_data",
                1_200_000_000,
                3,
            ),
        ];

        let result = db.insert_entries(entries).await;
        assert!(result.is_err(), "Malformed borsh data should cause failure");

        assert_eq!(
            count_rows(&db, "provider_registrations").await,
            0,
            "Failed transaction should roll back all inserts"
        );
        assert_eq!(
            count_rows(&db, "provider_check_ins").await,
            0,
            "Failed transaction should roll back all inserts"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_empty_vec_returns_ok() {
        let db = setup_test_db().await;
        let result = db.insert_entries(vec![]).await;
        assert!(result.is_ok(), "Empty entries should return Ok immediately");
    }

    #[tokio::test]
    async fn test_insert_entries_prov_register() {
        let db = setup_test_db().await;

        let entries = vec![make_entry(
            LABEL_PROV_REGISTER,
            b"prov_provider_key",
            b"prov_signature",
            1_000_000_000,
            1,
        )];
        db.insert_entries(entries).await.unwrap();

        assert_eq!(
            count_rows(&db, "provider_registrations").await,
            1,
            "PROV_REGISTER should dispatch to provider_registrations"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_legacy_64byte_check_in_fallback() {
        let db = setup_test_db().await;

        let provider_key = b"legacy_checkin_provider";
        let legacy_nonce = vec![0xABu8; 64];

        let entries = vec![
            make_entry(LABEL_PROV_REGISTER, provider_key, b"sig", 1_000_000_000, 1),
            make_entry(
                LABEL_PROV_CHECK_IN,
                provider_key,
                &legacy_nonce,
                1_100_000_000,
                2,
            ),
        ];
        db.insert_entries(entries).await.unwrap();

        assert_eq!(
            count_rows(&db, "provider_check_ins").await,
            1,
            "Legacy 64-byte nonce should be accepted as check-in"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_provider_registration_upsert() {
        let db = setup_test_db().await;

        let provider_key = b"upsert_provider";

        let entries_v1 = vec![make_entry(
            LABEL_PROV_REGISTER,
            provider_key,
            b"signature_v1",
            1_000_000_000,
            1,
        )];
        db.insert_entries(entries_v1).await.unwrap();

        assert_eq!(count_rows(&db, "provider_registrations").await, 1);

        let entries_v2 = vec![make_entry(
            LABEL_PROV_REGISTER,
            provider_key,
            b"signature_v2",
            2_000_000_000,
            2,
        )];
        db.insert_entries(entries_v2).await.unwrap();

        assert_eq!(
            count_rows(&db, "provider_registrations").await,
            1,
            "Upsert should not create duplicate rows"
        );

        use sqlx::Row;
        let row = sqlx::query("SELECT signature FROM provider_registrations WHERE pubkey != $1")
            .bind(Database::example_provider_pubkey())
            .fetch_one(&db.pool)
            .await
            .unwrap();
        let sig: Vec<u8> = row.get("signature");
        assert_eq!(sig, b"signature_v2", "Upsert should update the signature");
    }

    #[tokio::test]
    async fn test_insert_entries_mixed_prov_and_np_register_same_batch() {
        let db = setup_test_db().await;

        let entries = vec![
            make_entry(
                LABEL_PROV_REGISTER,
                b"prov_key",
                b"prov_sig",
                1_000_000_000,
                1,
            ),
            make_entry(LABEL_NP_REGISTER, b"np_key", b"np_sig", 1_100_000_000, 2),
        ];
        db.insert_entries(entries).await.unwrap();

        assert_eq!(
            count_rows(&db, "provider_registrations").await,
            2,
            "Both PROV_REGISTER and NP_REGISTER should insert into provider_registrations"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_user_register() {
        let db = setup_test_db().await;

        let entries = vec![make_entry(
            LABEL_USER_REGISTER,
            b"user_pubkey",
            b"user_signature",
            1_000_000_000,
            1,
        )];
        db.insert_entries(entries).await.unwrap();

        assert_eq!(
            count_rows(&db, "user_registrations").await,
            1,
            "USER_REGISTER should dispatch to user_registrations"
        );
    }

    #[tokio::test]
    async fn test_insert_entries_provides_clear_error_context_on_failure() {
        let db = setup_test_db().await;

        let entries = vec![make_entry(
            LABEL_REPUTATION_CHANGE,
            b"some_key",
            b"not_valid_borsh",
            1_000_000_000,
            1,
        )];

        let result = db.insert_entries(entries).await;
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("reputation change"),
            "Error should contain context about which handler failed, got: {}",
            err_msg
        );
    }
}
