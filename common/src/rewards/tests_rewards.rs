use super::*;
use crate::platform_specific::set_timestamp_ns;
use serde_json::Value;

#[test]
fn test_zero_reward_period_elapsed() {
    set_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS);
    reward_e9s_per_block_recalculate();
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS),
        0
    );
}

#[test]
fn test_single_reward_period() {
    let base_ts = FIRST_BLOCK_TIMESTAMP_NS;
    let after_one_period = base_ts + BLOCK_INTERVAL_SECS * 1_000_000_000;
    set_timestamp_ns(after_one_period);
    reward_e9s_per_block_recalculate();
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(base_ts),
        50 * DC_TOKEN_DECIMALS_DIV
    );
}

#[test]
fn test_partial_reward_period() {
    let base_ts = FIRST_BLOCK_TIMESTAMP_NS;
    let partway_period = base_ts + BLOCK_INTERVAL_SECS / 2 * 1_000_000_000;
    set_timestamp_ns(partway_period);
    reward_e9s_per_block_recalculate();
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(base_ts),
        25 * DC_TOKEN_DECIMALS_DIV
    );
}

#[test]
fn test_immediate_before_halving() {
    let base_ts = FIRST_BLOCK_TIMESTAMP_NS;
    let before_halving =
        base_ts + (REWARD_HALVING_AFTER_BLOCKS * BLOCK_INTERVAL_SECS - 1) * 1_000_000_000; // Just before halving
    set_timestamp_ns(before_halving);
    reward_e9s_per_block_recalculate();
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(base_ts),
        (50 * DC_TOKEN_DECIMALS_DIV) * 210000 - 83333334
    );
}

#[test]
fn test_at_halving_point() {
    let base_ts = FIRST_BLOCK_TIMESTAMP_NS;
    let at_halving = base_ts + 210000 * BLOCK_INTERVAL_SECS * 1_000_000_000; // At halving
    set_timestamp_ns(at_halving);
    reward_e9s_per_block_recalculate();
    assert_eq!(reward_e9s_per_block(), 25 * DC_TOKEN_DECIMALS_DIV);
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(base_ts),
        25 * DC_TOKEN_DECIMALS_DIV * 210000
    );
}

#[test]
fn test_after_several_halvings() {
    let base_ts = FIRST_BLOCK_TIMESTAMP_NS;
    let long_after = base_ts + 5 * 210000 * BLOCK_INTERVAL_SECS * 1_000_000_000; // After several halvings
    set_timestamp_ns(long_after);
    reward_e9s_per_block_recalculate();
    let expected_reward = ((50 * DC_TOKEN_DECIMALS_DIV) >> 5) * 5 * 210000; // Should have halved 5 times
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(base_ts),
        expected_reward
    );
}

#[test]
fn test_rewards_per_time_period() {
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS),
        0
    );
    set_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS + BLOCK_INTERVAL_SECS * 1_000_000_000 / 2);
    reward_e9s_per_block_recalculate();
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS),
        25 * DC_TOKEN_DECIMALS_DIV
    );
    set_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS + BLOCK_INTERVAL_SECS * 1_000_000_000);
    reward_e9s_per_block_recalculate();
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS),
        50 * DC_TOKEN_DECIMALS_DIV
    );
    set_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS + BLOCK_INTERVAL_SECS * 3 / 2 * 1_000_000_000);
    reward_e9s_per_block_recalculate();
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS),
        75 * DC_TOKEN_DECIMALS_DIV
    );

    // However, if rewards are distributed after 1 block only, it should be only 25 tokens
    assert_eq!(
        calc_token_rewards_e9_since_timestamp_ns(
            FIRST_BLOCK_TIMESTAMP_NS + BLOCK_INTERVAL_SECS * 1_000_000_000
        ),
        25 * DC_TOKEN_DECIMALS_DIV
    );
}

fn log_init() {
    // Set log level to info by default
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    let _ = env_logger::builder().is_test(true).try_init();
}

fn new_temp_ledger(labels_to_index: Option<Vec<String>>) -> LedgerMap {
    log_init();
    info!("Create temp ledger");
    // Create a temporary directory for the test
    let file_path = tempfile::tempdir()
        .unwrap()
        .into_path()
        .join("test_ledger_store.bin");

    LedgerMap::new_with_path(labels_to_index, Some(file_path))
        .expect("Failed to create a test temp ledger")
}

// fn new_test_account(desc: &[u8]) -> Account {
//     Account::from(Principal::from_slice(desc))
// }

#[test]
fn test_get_last_rewards_distribution_ts() {
    let mut test_ledger = new_temp_ledger(None);

    let result = get_last_rewards_distribution_ts(&test_ledger);
    assert_eq!(result.unwrap(), FIRST_BLOCK_TIMESTAMP_NS);

    // Insert initial data
    test_ledger
        .upsert(
            LABEL_REWARD_DISTRIBUTION,
            KEY_LAST_REWARD_DISTRIBUTION_TS,
            1234567890u64.to_le_bytes(),
        )
        .unwrap();

    // Call the function
    let result = get_last_rewards_distribution_ts(&test_ledger);
    assert_eq!(result.unwrap(), 1234567890);
}

#[test]
fn test_rewards_distribute_no_eligible_nps() {
    let mut test_ledger = new_temp_ledger(None);
    set_timestamp_ns(FIRST_BLOCK_TIMESTAMP_NS + BLOCK_INTERVAL_SECS * 1_000_000_000);

    // Insert initial data
    test_ledger
        .upsert(
            LABEL_REWARD_DISTRIBUTION,
            KEY_LAST_REWARD_DISTRIBUTION_TS,
            FIRST_BLOCK_TIMESTAMP_NS.to_le_bytes(),
        )
        .unwrap();

    // Call the function
    let result = rewards_distribute(&mut test_ledger);
    assert!(result.is_ok());
    let response: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(
        response[0].as_str().unwrap(),
        "Distributing reward of 50.000000000 DC tokens: no eligible NPs"
    );
}

#[test]
fn test_rewards_distribute_with_eligible_nps() {
    let mut test_ledger = new_temp_ledger(None);
    let mut last_ts_ns = FIRST_BLOCK_TIMESTAMP_NS + BLOCK_INTERVAL_SECS * 1_000_000_000;
    set_timestamp_ns(last_ts_ns);

    test_ledger
        .upsert(
            LABEL_REWARD_DISTRIBUTION,
            KEY_LAST_REWARD_DISTRIBUTION_TS,
            FIRST_BLOCK_TIMESTAMP_NS.to_le_bytes(),
        )
        .unwrap();

    let np1 = DccIdentity::new_from_seed(b"np1_seed").unwrap();
    test_ledger
        .upsert(
            LABEL_NP_REGISTER,
            np1.to_bytes_verifying(),
            np1.to_bytes_verifying(),
        )
        .unwrap();
    test_ledger
        .upsert(
            LABEL_NP_CHECK_IN,
            np1.to_bytes_verifying(),
            np1.to_bytes_verifying(),
        )
        .unwrap();

    let result = rewards_distribute(&mut test_ledger).unwrap();
    // Response is a JSON array of strings
    let result: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        result[0].as_str().unwrap(),
        "Distributing reward of 50.000000000 DC tokens to 1 NPs = 50.000000000 DC tokens per NP"
    );

    // Fast forward 42 blocks, there are rewards for 42 blocks that should be distributed
    last_ts_ns += 42 * BLOCK_INTERVAL_SECS * 1_000_000_000;
    set_timestamp_ns(last_ts_ns);

    test_ledger
        .upsert(
            LABEL_NP_CHECK_IN,
            np1.to_bytes_verifying(),
            np1.to_bytes_verifying(),
        )
        .unwrap();
    let result = rewards_distribute(&mut test_ledger).unwrap();
    let result: Value = serde_json::from_str(&result).unwrap();
    // 50 tokens * 42 = 2100
    assert_eq!(
        result[0].as_str().unwrap(),
        "Distributing reward of 2100.000000000 DC tokens to 1 NPs = 2100.000000000 DC tokens per NP"
    );

    // Later on, both np1 and np2 should be eligible for rewards. Each should get 25 tokens
    last_ts_ns += 7 * BLOCK_INTERVAL_SECS * 1_000_000_000;
    set_timestamp_ns(last_ts_ns);
    let np2 = DccIdentity::new_from_seed(b"np2_seed").unwrap();
    test_ledger
        .upsert(
            LABEL_NP_REGISTER,
            np2.to_bytes_verifying(),
            np2.to_bytes_verifying(),
        )
        .unwrap();
    test_ledger
        .upsert(
            LABEL_NP_CHECK_IN,
            np1.to_bytes_verifying(),
            np1.to_bytes_verifying(),
        )
        .unwrap();
    test_ledger
        .upsert(
            LABEL_NP_CHECK_IN,
            np2.to_bytes_verifying(),
            np2.to_bytes_verifying(),
        )
        .unwrap();

    let result = rewards_distribute(&mut test_ledger).unwrap();
    let result: Value = serde_json::from_str(&result).unwrap();
    // 50 tokens * 7 = 350
    assert_eq!(
        result[0].as_str().unwrap(),
        "Distributing reward of 350.000000000 DC tokens to 2 NPs = 175.000000000 DC tokens per NP"
    );
}
