use crate::ledger_path::ledger_file_path;
use anyhow::{Context, Result};
use dcc_common::{
    blocks_until_next_halving, refresh_caches_from_ledger, reward_e9s_per_block,
    reward_e9s_per_block_recalculate, rewards_current_block_checked_in, rewards_pending_e9s,
};
use ledger_map::LedgerMap;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct LedgerMetrics {
    pub total_blocks: u64,
    pub latest_block_timestamp_ns: u64,
    pub blocks_until_next_halving: u64,
    pub current_block_validators: u64,
    pub current_block_rewards_e9s: u64,
    pub reward_per_block_e9s: u64,
}

#[allow(dead_code)]
pub fn load_ledger_metrics() -> Result<LedgerMetrics> {
    let ledger_file = ledger_file_path().context("Ledger directory unavailable")?;
    load_metrics_from_file(&ledger_file)
}

fn load_metrics_from_file(path: &Path) -> Result<LedgerMetrics> {
    if !path.exists() {
        return Err(anyhow::format_err!(
            "Ledger file {} does not exist",
            path.display()
        ));
    }

    let ledger =
        LedgerMap::new_with_path(None, Some(PathBuf::from(path))).context("Ledger open failed")?;

    refresh_caches_from_ledger(&ledger).context("Failed to refresh ledger caches")?;
    reward_e9s_per_block_recalculate();

    Ok(LedgerMetrics {
        total_blocks: ledger.get_blocks_count() as u64,
        latest_block_timestamp_ns: ledger.get_latest_block_timestamp_ns(),
        blocks_until_next_halving: blocks_until_next_halving(),
        current_block_validators: rewards_current_block_checked_in(&ledger) as u64,
        current_block_rewards_e9s: rewards_pending_e9s(&ledger),
        reward_per_block_e9s: reward_e9s_per_block(),
    })
}

#[cfg(test)]
mod tests {
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
}
