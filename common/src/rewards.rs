use crate::platform_specific::get_timestamp_ns;
use crate::MAX_PUBKEY_BYTES;
use crate::{
    account_transfers::FundsTransfer, amount_as_string_u64,
    charge_fees_to_account_no_bump_reputation, get_account_from_pubkey, info,
    ledger_funds_transfer, np_registration_fee_e9s, DccIdentity, TransferError,
    BLOCK_INTERVAL_SECS, DC_TOKEN_DECIMALS_DIV, FIRST_BLOCK_TIMESTAMP_NS,
    KEY_LAST_REWARD_DISTRIBUTION_TS, LABEL_NP_CHECK_IN, LABEL_NP_REGISTER,
    LABEL_REWARD_DISTRIBUTION, MINTING_ACCOUNT, REWARD_HALVING_AFTER_BLOCKS,
};
use candid::Principal;
use ed25519_dalek::Signature;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use std::cell::RefCell;

fn calc_token_rewards_e9_since_timestamp_ns(last_reward_distribution_ts_ns: u64) -> u64 {
    let elapsed_secs_since_reward_distribution =
        (get_timestamp_ns().saturating_sub(last_reward_distribution_ts_ns)) / 1_000_000_000;
    let reward_amount_e9 = reward_e9s_per_block();

    info!(
        "Elapsed {} seconds since the last reward distribution at timestamp {}",
        elapsed_secs_since_reward_distribution, last_reward_distribution_ts_ns
    );

    reward_amount_e9 * elapsed_secs_since_reward_distribution / BLOCK_INTERVAL_SECS
}

thread_local! {
    static REWARD_E9S_PER_BLOCK: RefCell<u64> = const { RefCell::new(0) };
}

pub fn reward_e9s_per_block_recalculate() {
    REWARD_E9S_PER_BLOCK.with(|reward| {
        let elapsed_secs_since_reward_distribution =
            (get_timestamp_ns().saturating_sub(FIRST_BLOCK_TIMESTAMP_NS)) / 1_000_000_000;
        let reward_rounds = elapsed_secs_since_reward_distribution / BLOCK_INTERVAL_SECS;
        let mut reward_amount_e9 = 50 * DC_TOKEN_DECIMALS_DIV;
        for _ in 0..reward_rounds / REWARD_HALVING_AFTER_BLOCKS {
            reward_amount_e9 /= 2;
        }

        info!(
            "Reward per block set to: {} tokens ({} reward rounds)",
            amount_as_string_u64(reward_amount_e9),
            reward_rounds
        );

        reward.replace(reward_amount_e9);
    })
}

pub fn reward_e9s_per_block() -> u64 {
    REWARD_E9S_PER_BLOCK.with(|reward| *reward.borrow())
}

pub fn get_last_rewards_distribution_ts(ledger: &LedgerMap) -> Result<u64, String> {
    match ledger.get(LABEL_REWARD_DISTRIBUTION, KEY_LAST_REWARD_DISTRIBUTION_TS) {
        Ok(value_bytes) => Ok(u64::from_le_bytes(
            value_bytes.as_slice()[..8]
                .try_into()
                .expect("slice with incorrect length"),
        )),
        Err(_) => {
            let latest_block_ts = ledger.get_latest_block_timestamp_ns();
            if latest_block_ts > 0 {
                info!(
                    "Get last distribution ts: using latest block ts {}",
                    latest_block_ts
                );
                Ok(latest_block_ts)
            } else {
                info!(
                    "Get last distribution ts: using first block ts {} ns",
                    FIRST_BLOCK_TIMESTAMP_NS
                );
                Ok(FIRST_BLOCK_TIMESTAMP_NS)
            }
        }
    }
}

pub fn rewards_pending_e9s(ledger: &LedgerMap) -> u64 {
    let since_ts = match get_last_rewards_distribution_ts(ledger) {
        Ok(ts) => ts,
        Err(_) => return 0,
    };
    calc_token_rewards_e9_since_timestamp_ns(since_ts)
}

pub fn rewards_distribute(ledger: &mut LedgerMap) -> Result<String, TransferError> {
    let since_ts =
        get_last_rewards_distribution_ts(ledger).map_err(|e| TransferError::GenericError {
            error_code: 10046u64.into(),
            message: e,
        })?;

    reward_e9s_per_block_recalculate();
    let rewards_e9_to_distribute = calc_token_rewards_e9_since_timestamp_ns(since_ts);
    let mut response_text = Vec::new();
    let eligible_nps = ledger
        .next_block_iter(Some(LABEL_NP_CHECK_IN))
        .cloned()
        .collect::<Vec<_>>();

    if eligible_nps.is_empty() {
        let msg = format!(
            "Distributing reward of {} tokens: no eligible NPs",
            amount_as_string_u64(rewards_e9_to_distribute)
        );
        info!("{}", msg);
        response_text.push(msg.to_string());
        return serde_json::to_string_pretty(&response_text).map_err(|e| {
            TransferError::GenericError {
                error_code: 10064u64.into(),
                message: e.to_string(),
            }
        });
    }

    let token_rewards_per_np = rewards_e9_to_distribute / (eligible_nps.len() as u64);
    response_text.push(format!(
        "Distributing reward of {} tokens to {} NPs = {} tokens per NP",
        amount_as_string_u64(rewards_e9_to_distribute),
        eligible_nps.len(),
        amount_as_string_u64(token_rewards_per_np)
    ));
    info!("{}", response_text.iter().last().unwrap());

    ledger
        .upsert(
            LABEL_REWARD_DISTRIBUTION,
            KEY_LAST_REWARD_DISTRIBUTION_TS,
            get_timestamp_ns().to_le_bytes(),
        )
        .map_err(|e| TransferError::GenericError {
            error_code: 10086u64.into(),
            message: e.to_string(),
        })?;

    for np in eligible_nps {
        let np_acct = get_account_from_pubkey(np.key());

        ledger_funds_transfer(
            ledger,
            FundsTransfer::new(
                MINTING_ACCOUNT,
                np_acct,
                None,
                None,
                Some(get_timestamp_ns()),
                vec![],
                token_rewards_per_np.into(),
            ),
        )?;
    }
    info!("rewards distributed: {}", response_text.last().unwrap());
    serde_json::to_string_pretty(&response_text).map_err(|e| TransferError::GenericError {
        error_code: 10112u64.into(),
        message: e.to_string(),
    })
}

pub fn do_node_provider_check_in(
    ledger: &mut LedgerMap,
    caller: Principal,
    pubkey_bytes: Vec<u8>,
    nonce_signature: Vec<u8>,
) -> Result<String, String> {
    info!("[do_node_provider_check_in]: caller: {}", caller);

    if pubkey_bytes.len() > MAX_PUBKEY_BYTES {
        return Err("Node provider unique id too long".to_string());
    }
    if nonce_signature.len() != 64 {
        return Err("Invalid signature".to_string());
    }
    // Ensure the NP is registered
    ledger
        .get(LABEL_NP_REGISTER, &pubkey_bytes)
        .map_err(|e| e.to_string())?;
    let dcc_identity =
        DccIdentity::new_verifying_from_bytes(&pubkey_bytes).map_err(|e| e.to_string())?;
    info!(
        "Check-in of {}, account: {}",
        dcc_identity,
        get_account_from_pubkey(&pubkey_bytes)
    );
    let latest_nonce = ledger.get_latest_block_hash();
    let signature = Signature::from_slice(&nonce_signature).map_err(|e| e.to_string())?;
    info!(
        "Checking signature {} against latest nonce: {}",
        signature,
        hex::encode(&latest_nonce)
    );
    dcc_identity
        .verify(&latest_nonce, &signature)
        .expect("Signature didn't verify");

    if ledger.get_blocks_count() > 0 {
        let amount = np_registration_fee_e9s();
        info!(
            "Charging {} tokens {} for NP check in",
            amount_as_string_u64(amount),
            dcc_identity.to_ic_principal()
        );
        charge_fees_to_account_no_bump_reputation(ledger, &dcc_identity, amount)?;
    }

    ledger
        .upsert(LABEL_NP_CHECK_IN, pubkey_bytes, nonce_signature)
        .map(|_| "ok".to_string())
        .map_err(|e| format!("{:?}", e))?;

    Ok("Signature verified, check in successful.".to_string())
}

pub fn rewards_applied_np_count(ledger: &LedgerMap) -> usize {
    ledger.next_block_iter(Some(LABEL_NP_CHECK_IN)).count()
}

#[cfg(test)]
mod tests_rewards;
