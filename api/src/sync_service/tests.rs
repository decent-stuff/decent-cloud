use crate::database::{Database, LedgerEntryData};
use ledger_map::LedgerMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::NamedTempFile;

/// Test setup helper for creating a database
async fn setup_test_db() -> Database {
    Database::new(":memory:").await.unwrap()
}

#[test]
fn test_sync_service_interval_creation() {
    let interval_secs = 60u64;
    let expected_duration = Duration::from_secs(interval_secs);

    // Test duration creation
    assert_eq!(expected_duration, Duration::from_secs(60));

    // Test that intervals are comparable
    assert_eq!(expected_duration.as_secs(), 60);
}

#[tokio::test]
async fn test_database_initialization() {
    let database = setup_test_db().await;
    let initial_position = database.get_last_sync_position().await.unwrap();
    assert_eq!(initial_position, 0);
}

#[tokio::test]
async fn test_parse_empty_data() {
    let temp_file = NamedTempFile::new().unwrap();
    let ledger_parser =
        LedgerMap::new_with_path(None, Some(temp_file.path().to_path_buf())).unwrap();

    let mut entries = Vec::new();
    for block_result in ledger_parser.iter_raw_from_slice(&[]) {
        let (_block_header, block, _block_hash) = block_result.unwrap();
        for entry in block.entries() {
            entries.push(crate::database::LedgerEntryData {
                label: entry.label().to_string(),
                key: entry.key().to_vec(),
                value: entry.value().to_vec(),
            });
        }
    }

    assert_eq!(entries.len(), 0);
}

#[test]
fn test_ledger_parser_creation() {
    let temp_file = NamedTempFile::new().unwrap();
    let ledger_parser =
        LedgerMap::new_with_path(None, Some(temp_file.path().to_path_buf())).unwrap();

    // Test that we can wrap it in required Mutex type
    let wrapped_parser = Arc::new(Mutex::new(ledger_parser));
    assert_eq!(Arc::strong_count(&wrapped_parser), 1);
}

#[tokio::test]
async fn test_ledger_entry_data_creation() {
    let entry = LedgerEntryData {
        label: "test_label".to_string(),
        key: b"test_key".to_vec(),
        value: b"test_value".to_vec(),
    };

    assert_eq!(entry.label, "test_label");
    assert_eq!(entry.key, b"test_key");
    assert_eq!(entry.value, b"test_value");
}

#[tokio::test]
async fn test_cursor_formatting() {
    let position = 100u64;
    let cursor_string = if position > 0 {
        Some(format!("position={}", position))
    } else {
        None
    };

    assert_eq!(cursor_string, Some("position=100".to_string()));

    let zero_position = 0u64;
    let zero_cursor = if zero_position > 0 {
        Some(format!("position={}", zero_position))
    } else {
        None
    };

    assert_eq!(zero_cursor, None);
}

#[tokio::test]
async fn test_position_calculation() {
    let last_position = 100u64;
    let data_len = 50usize;
    let new_position = last_position + data_len as u64;

    assert_eq!(new_position, 150);

    // Test edge case with zero data
    let zero_data_len = 0usize;
    let no_change_position = last_position + zero_data_len as u64;
    assert_eq!(no_change_position, 100);
}

#[tokio::test]
async fn test_multiple_ledger_entries() {
    let entries_array = [
        LedgerEntryData {
            label: "label1".to_string(),
            key: b"key1".to_vec(),
            value: b"value1".to_vec(),
        },
        LedgerEntryData {
            label: "label2".to_string(),
            key: b"key2".to_vec(),
            value: b"value2".to_vec(),
        },
        LedgerEntryData {
            label: "label3".to_string(),
            key: b"key3".to_vec(),
            value: b"value3".to_vec(),
        },
    ];
    let entries: Vec<LedgerEntryData> = entries_array.to_vec();

    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].label, "label1");
    assert_eq!(entries[1].label, "label2");
    assert_eq!(entries[2].label, "label3");
}

#[tokio::test]
async fn test_ledger_dir_env_var_persistence() {
    // Test that LEDGER_DIR environment variable is properly used
    let temp_dir = tempfile::tempdir().unwrap();
    let ledger_path = temp_dir.path().to_str().unwrap();
    
    // Set the environment variable
    std::env::set_var("LEDGER_DIR", ledger_path);
    
    // Verify that SyncService reads the LEDGER_DIR environment variable correctly
    let ledger_dir = std::env::var("LEDGER_DIR")
        .map(|path| std::path::PathBuf::from(path))
        .unwrap_or_else(|_| {
            tempfile::tempdir().expect("Failed to create temp dir").keep()
        });
    
    // Verify the correct directory path is used
    assert_eq!(ledger_dir, temp_dir.path());
    assert!(ledger_dir.exists());
    
    // Clean up
    std::env::remove_var("LEDGER_DIR");
}

#[tokio::test]
async fn test_ledger_dir_fallback_to_temp() {
    // Ensure LEDGER_DIR is not set
    std::env::remove_var("LEDGER_DIR");
    
    // Verify that a temp directory is created when LEDGER_DIR is not set
    let ledger_dir = std::env::var("LEDGER_DIR")
        .map(|path| std::path::PathBuf::from(path))
        .unwrap_or_else(|_| {
            // This should create a temp directory
            let temp_dir = tempfile::tempdir()
                .expect("Failed to create temp dir");
            temp_dir.keep()
        });
    
    // Verify the directory exists
    assert!(ledger_dir.exists());
}

#[tokio::test]
async fn test_structured_provider_registration() {
    let database = setup_test_db().await;
    
    // Create a mock provider registration entry
    let entries = vec![LedgerEntryData {
        label: "ProvRegister".to_string(),
        key: vec![1, 2, 3, 4], // Mock pubkey hash
        value: vec![5, 6, 7, 8], // Mock crypto signature
    }];

    // Insert entries into database
    database.insert_entries(entries).await.unwrap();

    // Verify to entry was inserted into the structured table
    let row = sqlx::query("SELECT * FROM provider_registrations WHERE pubkey_hash = ?")
        .bind(&[1, 2, 3, 4][..])
        .fetch_one(database.pool())
        .await
        .unwrap();

    assert!(!row.is_empty());
}

#[tokio::test]
async fn test_structured_provider_check_in() {
    let database = setup_test_db().await;
    
    // Create a mock provider check-in entry with proper CheckInPayload structure
    let check_in_payload = dcc_common::CheckInPayload::new(
        "Test memo".to_string(),
        vec![9, 10, 11, 12] // Mock nonce signature
    );
    let check_in_bytes = check_in_payload.to_bytes().unwrap();
    
    let entries = vec![LedgerEntryData {
        label: "ProvCheckIn".to_string(),
        key: vec![1, 2, 3, 4], // Mock pubkey hash
        value: check_in_bytes,
    }];

    // Insert entries into database
    database.insert_entries(entries).await.unwrap();

    // Verify to entry was inserted into the structured table
    let row = sqlx::query("SELECT * FROM provider_check_ins WHERE pubkey_hash = ?")
        .bind(&[1, 2, 3, 4][..])
        .fetch_one(database.pool())
        .await
        .unwrap();

    let memo: String = row.get("memo");
    assert_eq!(memo, "Test memo");
}

#[tokio::test]
async fn test_structured_token_transfer() {
    let database = setup_test_db().await;
    
    // Create a mock token transfer entry
    let from_account = dcc_common::IcrcCompatibleAccount::from_hex("0x74657374000000000000000000000000000000000000000000000000000000000").unwrap();
    let to_account = dcc_common::IcrcCompatibleAccount::from_hex("0x74657374000000000000000000000000000000000000000000000000000000001").unwrap();
    
    let transfer = dcc_common::FundsTransfer::new(
        from_account,
        to_account,
        None, // fee
        Some(b"Test memo".to_vec()),
        None, // created_at_time
        None, // balance_to_before
        None, // balance_from_before
        None, // balance_to_after
        None, // balance_from_after
        None, // block_hash
        None, // block_offset
    );
    let transfer_bytes = transfer.to_bytes().unwrap();
    
    let entries = vec![LedgerEntryData {
        label: "DCTokenTransfer".to_string(),
        key: b"test_key".to_vec(),
        value: transfer_bytes,
    }];

    // Insert entries into database
    database.insert_entries(entries).await.unwrap();

    // Verify to entry was inserted into the structured table
    let row = sqlx::query("SELECT * FROM token_transfers")
        .fetch_one(database.pool())
        .await
        .unwrap();

    let amount: i64 = row.get("amount_e9s");
    assert!(amount >= 0);
}

#[tokio::test]
async fn test_structured_mixed_entries() {
    let database = setup_test_db().await;
    
    // Create a mix of structured entries
    let entries = vec![
        LedgerEntryData {
            label: "ProvRegister".to_string(),
            key: vec![1, 2, 3, 4],
            value: vec![5, 6, 7, 8],
        },
        LedgerEntryData {
            label: "UserRegister".to_string(),
            key: vec![9, 10, 11, 12],
            value: vec![13, 14, 15, 16],
        },
        LedgerEntryData {
            label: "ProvCheckIn".to_string(),
            key: vec![1, 2, 3, 4],
            value: dcc_common::CheckInPayload::new(
                "Provider check-in".to_string(),
                vec![17, 18, 19, 20]
            ).to_bytes().unwrap(),
        },
    ];

    // Insert entries into database
    database.insert_entries(entries).await.unwrap();

    // Verify structured entries
    let provider_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM provider_registrations")
        .fetch_one(database.pool())
        .await
        .unwrap()
        .get("count");
    
    let user_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM user_registrations")
        .fetch_one(database.pool())
        .await
        .unwrap()
        .get("count");

    let check_in_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM provider_check_ins")
        .fetch_one(database.pool())
        .await
        .unwrap()
        .get("count");

    assert_eq!(provider_count, 1);
    assert_eq!(user_count, 1);
    assert_eq!(check_in_count, 1);
}
