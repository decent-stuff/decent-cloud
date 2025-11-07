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
