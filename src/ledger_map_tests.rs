#[cfg(test)]
mod tests {
    use std::vec;

    use crate::info;

    use crate::ledger_entry::LedgerBlockHeader;
    use crate::{partition_table, LedgerBlock, LedgerEntry, LedgerError, LedgerMap, Operation};

    #[cfg(not(target_arch = "wasm32"))]
    fn log_init() {
        // Set log level to info by default
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "info");
        }
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[cfg(target_arch = "wasm32")]
    fn log_init() {
        // No-op for wasm
    }

    fn new_temp_ledger(labels_to_index: Option<Vec<String>>) -> LedgerMap {
        log_init();
        info!("Create temp ledger");
        // Create a temporary directory for the test
        let file_path = tempfile::tempdir()
            .unwrap()
            .into_path()
            .join("test_ledger_store.bin");

        fn mock_get_timestamp_nanos() -> u64 {
            0
        }

        LedgerMap::new_with_path(labels_to_index, Some(file_path))
            .expect("Failed to create a temp ledger for the test")
            .with_timestamp_fn(mock_get_timestamp_nanos)
    }

    #[test]
    fn test_compute_cumulative_hash() {
        let parent_hash = vec![0, 1, 2, 3];
        let key = vec![4, 5, 6, 7];
        let value = vec![8, 9, 10, 11];
        let ledger_block = LedgerBlock::new(
            vec![LedgerEntry::new(
                "Label2",
                key.clone(),
                value.clone(),
                Operation::Upsert,
            )],
            0,
            vec![],
        );
        let cumulative_hash = LedgerMap::_compute_block_chain_hash(
            &parent_hash,
            ledger_block.entries(),
            ledger_block.timestamp(),
        )
        .unwrap();

        // Cumulative hash is a sha256 hash of the parent hash, key, and value
        // Obtained from a reference run
        assert_eq!(
            cumulative_hash,
            vec![
                21, 5, 93, 78, 94, 126, 142, 35, 221, 131, 204, 67, 57, 54, 102, 107, 225, 68, 197,
                244, 204, 60, 238, 250, 126, 8, 240, 137, 84, 55, 3, 91
            ]
        );
    }

    #[test]
    fn test_upsert() {
        let mut ledger_map = new_temp_ledger(None);

        // Test upsert
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        ledger_map
            .upsert("Label2", key.clone(), value.clone())
            .unwrap();
        println!("partition table {}", partition_table::get_partition_table());
        assert_eq!(ledger_map.get("Label2", &key).unwrap(), value);
        assert!(ledger_map.commit_block().is_ok());
        assert_eq!(ledger_map.get("Label2", &key).unwrap(), value);
        let entries = ledger_map.entries.get("Label2").unwrap();
        assert_eq!(
            entries.get(&key),
            Some(&LedgerEntry::new("Label2", key, value, Operation::Upsert,))
        );
        assert_eq!(ledger_map.metadata.borrow().num_blocks(), 1);
        assert!(ledger_map.next_block_entries.is_empty());
    }

    #[test]
    fn test_upsert_with_matching_entry_label() {
        let mut ledger_map = new_temp_ledger(None);

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        ledger_map
            .upsert("Label1", key.clone(), value.clone())
            .unwrap();
        assert_eq!(ledger_map.entries.get("Label1"), None); // value not committed yet
        assert_eq!(ledger_map.get("Label1", &key).unwrap(), value);
        ledger_map.commit_block().unwrap();
        let entries = ledger_map.entries.get("Label1").unwrap();
        assert_eq!(
            entries.get(&key),
            Some(&LedgerEntry::new(
                "Label1",
                key.clone(),
                value.clone(),
                Operation::Upsert,
            ))
        );
    }

    #[test]
    fn test_upsert_with_mismatched_entry_label() {
        let mut ledger_map = new_temp_ledger(None);

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        ledger_map
            .upsert("Label2", key.clone(), value.clone())
            .unwrap();

        // Ensure that the entry is not added to the NodeProvider ledger since the label doesn't match
        assert_eq!(ledger_map.entries.get("Label1"), None);
    }

    #[test]
    fn test_delete_with_matching_entry_label() {
        let mut ledger_map = new_temp_ledger(None);

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        ledger_map
            .upsert("Label1", key.clone(), value.clone())
            .unwrap();
        assert_eq!(ledger_map.get("Label1", &key).unwrap(), value); // Before delete: the value is there
        ledger_map.delete("Label1", key.clone()).unwrap();
        let expected_tombstone = Some(LedgerEntry::new(
            "Label1",
            key.clone(),
            vec![],
            Operation::Delete,
        ));
        assert_eq!(
            ledger_map.get("Label1", &key).unwrap_err(),
            LedgerError::EntryNotFound
        ); // After delete: the value is gone in the public interface
        assert_eq!(
            ledger_map
                .next_block_entries
                .get("Label1")
                .unwrap()
                .get(&key),
            expected_tombstone.as_ref()
        );
        assert_eq!(ledger_map.entries.get("Label1"), None); // (not yet committed)

        // Now commit the block
        assert!(ledger_map.commit_block().is_ok());

        // And recheck: the value is gone in the public interface and deletion is in the ledger
        assert_eq!(
            ledger_map.entries.get("Label1").unwrap().get(&key),
            expected_tombstone.as_ref()
        );
        assert_eq!(ledger_map.next_block_entries.get("Label1"), None);
        assert_eq!(
            ledger_map.get("Label1", &key).unwrap_err(),
            LedgerError::EntryNotFound
        );
    }

    #[test]
    fn test_delete_with_mismatched_entry_label() {
        let mut ledger_map = new_temp_ledger(None);

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        ledger_map
            .upsert("Label1", key.clone(), value.clone())
            .unwrap();
        ledger_map.get("Label1", &key).unwrap();
        assert!(ledger_map.entries.get("Label1").is_none()); // the value is not yet committed
        ledger_map.commit_block().unwrap();
        ledger_map.entries.get("Label1").unwrap();
        ledger_map.delete("Label2", key.clone()).unwrap();

        // Ensure that the entry is not deleted from the ledger since the label doesn't match
        let entries_np = ledger_map.entries.get("Label1").unwrap();
        assert_eq!(
            entries_np.get(&key),
            Some(&LedgerEntry::new(
                "Label1",
                key.clone(),
                value.clone(),
                Operation::Upsert,
            ))
        );
        assert_eq!(ledger_map.entries.get("Label2"), None);
    }

    #[test]
    fn test_labels_to_index() {
        let mut ledger_map = new_temp_ledger(Some(vec!["Label1".to_string()]));

        let key = b"test_key".to_vec();
        let value1 = b"test_value1".to_vec();
        let value2 = b"test_value2".to_vec();
        ledger_map
            .upsert("Label1", key.clone(), value1.clone())
            .unwrap();
        ledger_map
            .upsert("Label2", key.clone(), value2.clone())
            .unwrap();
        assert!(ledger_map.entries.get("Label1").is_none()); // the value is not yet committed
        assert!(ledger_map.entries.get("Label2").is_none()); // the value is not yet committed
        ledger_map.commit_block().unwrap();
        assert_eq!(ledger_map.get("Label1", &key).unwrap(), value1);
        assert_eq!(
            ledger_map.get("Label2", &key).unwrap_err(),
            LedgerError::EntryNotFound
        );
        // Delete the non-indexed entry, ensure that the indexed entry is still there
        ledger_map.delete("Label2", key.clone()).unwrap();
        assert_eq!(ledger_map.get("Label1", &key).unwrap(), value1);
        assert_eq!(
            ledger_map.get("Label2", &key).unwrap_err(),
            LedgerError::EntryNotFound
        );
        // Delete the indexed entry, ensure that it's gone
        ledger_map.delete("Label1", key.clone()).unwrap();
        assert_eq!(
            ledger_map.get("Label1", &key).unwrap_err(),
            LedgerError::EntryNotFound
        );
        assert_eq!(
            ledger_map.get("Label2", &key).unwrap_err(),
            LedgerError::EntryNotFound
        );
    }

    #[test]
    fn test_delete() {
        let mut ledger_map = new_temp_ledger(None);

        // Test delete
        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        ledger_map
            .upsert("Label2", key.clone(), value.clone())
            .unwrap();
        ledger_map.delete("Label2", key.clone()).unwrap();
        assert!(ledger_map.commit_block().is_ok());
        let entries = ledger_map.entries.get("Label2").unwrap();
        assert_eq!(
            entries.get(&key),
            Some(LedgerEntry::new(
                "Label2",
                key.clone(),
                vec![],
                Operation::Delete
            ))
            .as_ref()
        );
        assert_eq!(ledger_map.entries.get("Label1"), None);
        assert_eq!(
            ledger_map.get("Label2", &key).unwrap_err(),
            LedgerError::EntryNotFound
        );
    }

    #[test]
    fn test_refresh_ledger() {
        let mut ledger_map = new_temp_ledger(None);

        info!("New temp ledger created");
        info!("ledger: {:?}", ledger_map);

        let key = b"test_key".to_vec();
        let value = b"test_value".to_vec();
        ledger_map
            .upsert("Label2", key.clone(), value.clone())
            .unwrap();
        assert!(ledger_map.commit_block().is_ok());
        ledger_map.refresh_ledger().unwrap();

        let entry = ledger_map
            .entries
            .get("Label2")
            .unwrap()
            .values()
            .next()
            .unwrap()
            .clone();
        assert_eq!(
            entry,
            LedgerEntry::new("Label2", key.clone(), value.clone(), Operation::Upsert)
        );
        let expected_chain_hash = vec![
            245, 142, 15, 179, 87, 133, 107, 164, 123, 16, 145, 52, 243, 153, 170, 45, 177, 243,
            61, 37, 162, 237, 226, 100, 94, 136, 159, 73, 117, 58, 222, 153,
        ];
        assert_eq!(
            ledger_map.metadata.borrow().tip_block_chain_hash(),
            expected_chain_hash
        );
        assert_eq!(ledger_map.get_latest_block_hash(), expected_chain_hash);
    }

    #[test]
    fn test_ledger_block_offsets() {
        // Create a new ledger
        let mut ledger_map = new_temp_ledger(None);

        // Create some dummy entries
        ledger_map.upsert("label1", b"key1", b"value1").unwrap();
        ledger_map.commit_block().unwrap();
        ledger_map.upsert("label1a", b"key2a", b"value2aa").unwrap();
        ledger_map.commit_block().unwrap();
        ledger_map
            .upsert("label1bb", b"key3bbb", b"value3bbbb")
            .unwrap();
        ledger_map.commit_block().unwrap();

        let (headers, blocks): (Vec<_>, Vec<_>) = ledger_map.iter_raw().map(|x| x.unwrap()).unzip();

        let header_len_bytes = headers
            .iter()
            .map(|x| x.serialize().unwrap().len() as u32)
            .collect::<Vec<_>>();
        let blocks_len_bytes = blocks
            .iter()
            .map(|x| x.serialize().unwrap().len() as u32)
            .collect::<Vec<_>>();
        let blk0_bytes = (header_len_bytes[0] + blocks_len_bytes[0]) as u64;
        let blk1_bytes = (header_len_bytes[1] + blocks_len_bytes[1]) as u64;
        let blk2_bytes = (header_len_bytes[2] + blocks_len_bytes[2]) as u64;
        assert_eq!(headers[0].jump_bytes_prev_block(), 0);
        assert_eq!(headers[0].jump_bytes_next_block(), blk0_bytes as u32);

        assert_eq!(headers[1].jump_bytes_prev_block(), -(blk0_bytes as i32));
        assert_eq!(headers[1].jump_bytes_next_block(), blk1_bytes as u32);
        assert_eq!(headers[2].jump_bytes_prev_block(), -(blk1_bytes as i32));
        assert_eq!(headers[2].jump_bytes_next_block(), blk2_bytes as u32);
    }

    #[test]
    fn test_get_block_at_offset() {
        // Create a new ledger
        let mut ledger_map = new_temp_ledger(None);

        // Create some entries and commit them
        ledger_map.upsert("label1", b"key1", b"value1").unwrap();
        ledger_map.commit_block().unwrap();
        let first_block_pos = ledger_map.metadata.borrow().first_block_start_pos();
        assert!(first_block_pos > 0);

        ledger_map.upsert("label2", b"key2", b"value2").unwrap();
        ledger_map.commit_block().unwrap();
        let second_block_pos = ledger_map.get_latest_block_start_pos();

        // Test getting block at first position
        let (header1, block1) = ledger_map.get_block_at_offset(0).unwrap();
        assert_eq!(block1.entries().len(), 1);
        assert_eq!(block1.entries()[0].label(), "label1");
        assert_eq!(block1.entries()[0].key(), b"key1");
        assert_eq!(block1.entries()[0].value(), b"value1");
        assert_eq!(header1.jump_bytes_prev_block(), 0);
        assert!(header1.jump_bytes_next_block() > 0);

        // Test getting block at second position
        let (header2, block2) = ledger_map.get_block_at_offset(second_block_pos).unwrap();
        assert_eq!(block2.entries().len(), 1);
        assert_eq!(block2.entries()[0].label(), "label2");
        assert_eq!(block2.entries()[0].key(), b"key2");
        assert_eq!(block2.entries()[0].value(), b"value2");
        assert!(header2.jump_bytes_prev_block() < 0); // Should point back to previous block

        // Test getting block at invalid position (before first block)
        let result = ledger_map.get_block_at_offset(0);
        assert!(result.is_ok()); // Should return first block instead of error
        let (header, block) = result.unwrap();
        assert_eq!(block.entries()[0].label(), "label1"); // Should get first block
        assert_eq!(header.jump_bytes_prev_block(), 0);

        // Test getting block at non-existent position
        let invalid_pos = second_block_pos + 1000;
        let result = ledger_map.get_block_at_offset(invalid_pos);
        assert!(result.is_err());
    }
    #[test]
    fn test_get_block_from_slice() {
        // Create a new ledger
        let ledger_map = new_temp_ledger(None);

        // Create test data: a valid block header and block
        let entries = vec![
            LedgerEntry::new("test_label", b"key1", b"value1", Operation::Upsert),
            LedgerEntry::new("test_label", b"key2", b"value2", Operation::Upsert),
        ];
        let timestamp = 12345u64;
        let parent_hash = vec![1, 2, 3, 4];
        let block = LedgerBlock::new(entries, timestamp, parent_hash.clone());

        // Serialize the block
        let block_data = block.serialize().unwrap();
        let block_len = block_data.len() as u32;

        // Create a header
        let header = LedgerBlockHeader::new(0, block_len + LedgerBlockHeader::sizeof() as u32);
        let header_data = header.serialize().unwrap();

        // Combine header and block data
        let mut test_data = header_data;
        test_data.extend_from_slice(&block_data);

        // Test successful parsing
        let result = ledger_map.get_block_from_slice(&test_data);

        let (parsed_header, parsed_block) = result.unwrap();
        assert_eq!(parsed_header.block_version(), 1);
        assert_eq!(
            parsed_header.jump_bytes_next_block(),
            block_len + LedgerBlockHeader::sizeof() as u32
        );
        assert_eq!(parsed_block.entries().len(), 2);
        assert_eq!(parsed_block.timestamp(), timestamp);
        assert_eq!(parsed_block.parent_hash(), parent_hash);

        // Test with insufficient data (truncated block)
        let truncated_data = test_data[..test_data.len() - 10].to_vec();
        let result = ledger_map.get_block_from_slice(&truncated_data);
        assert!(result.is_err());
        match result.unwrap_err() {
            LedgerError::BlockCorrupted(_) => {} // Expected error
            err => panic!("Unexpected error: {:?}", err),
        }

        // Test with corrupted header (invalid version)
        let mut corrupted_data = test_data.clone();
        corrupted_data[0] = 99; // Set invalid version
        let result = ledger_map.get_block_from_slice(&corrupted_data);
        assert!(result.is_err());

        // Test with empty data
        let result = ledger_map.get_block_from_slice(&[]);
        assert!(result.is_err());

        // Test with data too small for header
        let result = ledger_map.get_block_from_slice(&[0, 1, 2]);
        assert!(result.is_err());
    }

    #[test]
    fn test_iter_raw_from_slice() {
        // Create a new ledger
        let ledger_map = new_temp_ledger(None);

        // Create multiple blocks
        let blocks_count = 3;
        let mut test_data = Vec::new();

        let mut offset = 0u32;
        // Create and serialize multiple blocks
        for i in 0..blocks_count {
            let entries = vec![LedgerEntry::new(
                format!("label{}", i),
                format!("key{}", i).as_bytes(),
                format!("value{}", i).as_bytes(),
                Operation::Upsert,
            )];
            let timestamp = (i as u64) * 1000;
            let parent_hash = vec![i as u8; 4];
            let block = LedgerBlock::new(entries, timestamp, parent_hash);

            let block_data = block.serialize().unwrap();
            let block_len = block_data.len() as u32;
            let jump = block_len + LedgerBlockHeader::sizeof() as u32;

            let header = LedgerBlockHeader::new(jump as i32 - offset as i32, jump);
            offset += jump;
            let header_data = header.serialize().unwrap();

            test_data.extend_from_slice(&header_data);
            test_data.extend_from_slice(&block_data);
        }

        // Test iterating through all blocks
        let blocks: Vec<_> = ledger_map
            .iter_raw_from_slice(&test_data)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(blocks.len(), blocks_count);

        // Verify each block's content
        for (i, (header, block)) in blocks.iter().enumerate() {
            assert_eq!(header.block_version(), 1);
            assert_eq!(block.timestamp(), (i as u64) * 1000);
            assert_eq!(block.entries().len(), 1);
            assert_eq!(block.entries()[0].label(), format!("label{}", i));
            assert_eq!(block.entries()[0].key(), format!("key{}", i).as_bytes());
            assert_eq!(block.entries()[0].value(), format!("value{}", i).as_bytes());
        }

        // Test with empty data
        assert_eq!(
            ledger_map
                .iter_raw_from_slice(&[])
                .collect::<Vec<_>>()
                .len(),
            0
        );

        // Test with corrupted data (zero jump length)
        let mut corrupted_data = test_data.clone();
        corrupted_data[8] = 0; // Set jump_bytes_next to 0
        corrupted_data[9] = 0;
        corrupted_data[10] = 0;
        corrupted_data[11] = 0;

        let result = ledger_map
            .iter_raw_from_slice(&corrupted_data)
            .collect::<Result<Vec<_>, _>>();
        assert!(result.is_err());

        // Test with partially valid data (first block valid, second corrupted)
        let mut partially_valid_data = test_data.clone();
        let first_block_size = blocks[0].0.jump_bytes_next_block() as usize;
        // Corrupt the second block's header
        partially_valid_data[first_block_size + 1] = 99; // Invalid version

        let blocks: Vec<_> = ledger_map
            .iter_raw_from_slice(&partially_valid_data)
            .take_while(Result::is_ok)
            .map(Result::unwrap)
            .collect();

        assert_eq!(blocks.len(), 1); // Only the first block should be valid
    }
}
