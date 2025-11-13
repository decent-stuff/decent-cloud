use super::*;
use dcc_common::{DccIdentity, LABEL_PROV_REGISTER};
use ledger_map::LedgerMap;
use tempfile::tempdir;

fn write_sample_ledger(path: &Path) {
    let mut ledger = LedgerMap::new_with_path(None, Some(path.to_path_buf())).unwrap();
    ledger.begin_block().unwrap();

    let identity = DccIdentity::new_from_seed(&[7u8; 32]).unwrap();
    ledger
        .upsert(
            LABEL_PROV_REGISTER,
            identity.to_bytes_verifying(),
            vec![1u8; 64],
        )
        .unwrap();

    ledger.commit_block().unwrap();
}

#[test]
fn fails_when_file_missing() {
    let temp_dir = tempdir().unwrap();
    let missing_file = temp_dir.path().join("missing.bin");
    let err = load_metrics_from_file(&missing_file).unwrap_err();
    assert!(err.to_string().contains("does not exist"));
}

#[test]
fn reads_metrics_from_populated_ledger() {
    let temp_dir = tempdir().unwrap();
    let ledger_file = temp_dir.path().join("main.bin");
    write_sample_ledger(&ledger_file);

    let metrics = load_metrics_from_file(&ledger_file).unwrap();

    assert!(metrics.total_blocks >= 1);
    assert!(metrics.blocks_until_next_halving > 0);
    assert!(metrics.latest_block_timestamp_ns > 0);
    assert_eq!(metrics.current_block_validators, 0);
}
