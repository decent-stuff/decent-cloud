use crate::{
    account_balance_get, account_transfers::FundsTransfer, amount_as_string,
    charge_fees_to_account_no_bump_reputation, fn_info, get_account_from_pubkey, info,
    ledger_funds_transfer, platform_specific::get_timestamp_ns, DccIdentity, TokenAmountE9s,
    TransferError, BLOCK_INTERVAL_SECS, DC_TOKEN_DECIMALS_DIV, FIRST_BLOCK_TIMESTAMP_NS,
    KEY_LAST_REWARD_DISTRIBUTION_TS, LABEL_PROV_CHECK_IN, LABEL_PROV_REGISTER,
    LABEL_REWARD_DISTRIBUTION, MINTING_ACCOUNT, REWARD_HALVING_AFTER_BLOCKS,
    TRANSFER_MEMO_BYTES_MAX, VALIDATION_MEMO_BYTES_MAX,
};
use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use serde::Serialize;
use std::cell::RefCell;

pub fn check_in_fee_e9s() -> TokenAmountE9s {
    reward_e9s_per_block() / 100
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
pub struct CheckInPayloadV1 {
    memo: String, // Memo can for example be shown on a dashboard, as an arbitrary personal message
    nonce_signature: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize)]
pub enum CheckInPayload {
    V1(CheckInPayloadV1),
}

impl CheckInPayload {
    pub fn new(memo: String, nonce_signature: Vec<u8>) -> CheckInPayload {
        CheckInPayload::V1(CheckInPayloadV1 {
            memo,
            nonce_signature,
        })
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(self)
    }

    pub fn memo(&self) -> &str {
        match self {
            CheckInPayload::V1(record) => &record.memo,
        }
    }

    pub fn nonce_signature(&self) -> &[u8] {
        match self {
            CheckInPayload::V1(record) => &record.nonce_signature,
        }
    }
}

fn calc_token_rewards_e9_since_timestamp_ns(last_reward_distribution_ts_ns: u64) -> TokenAmountE9s {
    let elapsed_secs_since_reward_distribution =
        (get_timestamp_ns().saturating_sub(last_reward_distribution_ts_ns)) / 1_000_000_000;
    let reward_amount_e9 = reward_e9s_per_block();

    info!(
        "Elapsed {} seconds since the last reward distribution at timestamp {}",
        elapsed_secs_since_reward_distribution, last_reward_distribution_ts_ns
    );

    reward_amount_e9 * elapsed_secs_since_reward_distribution as TokenAmountE9s
        / BLOCK_INTERVAL_SECS as TokenAmountE9s
}

thread_local! {
    static REWARD_E9S_PER_BLOCK: RefCell<TokenAmountE9s> = const { RefCell::new(0) };
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
            amount_as_string(reward_amount_e9),
            reward_rounds
        );

        reward.replace(reward_amount_e9);
    })
}

pub fn reward_e9s_per_block() -> TokenAmountE9s {
    REWARD_E9S_PER_BLOCK.with(|reward| *reward.borrow())
}

pub fn blocks_until_next_halving() -> u64 {
    let elapsed_secs_since_reward_distribution =
        (get_timestamp_ns().saturating_sub(FIRST_BLOCK_TIMESTAMP_NS)) / 1_000_000_000;
    let reward_rounds = elapsed_secs_since_reward_distribution / BLOCK_INTERVAL_SECS;

    REWARD_HALVING_AFTER_BLOCKS - reward_rounds % REWARD_HALVING_AFTER_BLOCKS
}

pub fn get_last_rewards_distribution_ts(ledger: &LedgerMap) -> Result<u64, String> {
    match ledger.get(LABEL_REWARD_DISTRIBUTION, KEY_LAST_REWARD_DISTRIBUTION_TS) {
        Ok(value_bytes) => {
            let bytes: [u8; 8] = value_bytes
                .as_slice()[..8]
                .try_into()
                .map_err(|_| "Stored timestamp is not 8 bytes".to_string())?;
            Ok(u64::from_le_bytes(bytes))
        }
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

pub fn rewards_pending_e9s(ledger: &LedgerMap) -> TokenAmountE9s {
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
    let eligible_providers = ledger
        .next_block_iter(Some(LABEL_PROV_CHECK_IN))
        .cloned()
        .collect::<Vec<_>>();

    if eligible_providers.is_empty() {
        let msg = format!(
            "Distributing reward of {} tokens: no eligible Providers",
            amount_as_string(rewards_e9_to_distribute)
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

    let token_rewards_per_provider =
        rewards_e9_to_distribute / (eligible_providers.len() as TokenAmountE9s);
    response_text.push(format!(
        "Distributing reward of {} tokens to {} Providers = {} tokens per Provider",
        amount_as_string(rewards_e9_to_distribute),
        eligible_providers.len(),
        amount_as_string(token_rewards_per_provider)
    ));
    if let Some(msg) = response_text.last() {
        info!("{}", msg);
    }

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

    for provider in eligible_providers {
        let provider_acct = get_account_from_pubkey(provider.key());

        let balance_to_after =
            account_balance_get(&provider_acct) + token_rewards_per_provider as TokenAmountE9s;
        ledger_funds_transfer(
            ledger,
            FundsTransfer::new(
                MINTING_ACCOUNT,
                provider_acct,
                None,
                None,
                Some(get_timestamp_ns()),
                vec![],
                token_rewards_per_provider,
                0,
                balance_to_after,
            ),
        )?;
    }
    if let Some(msg) = response_text.last() {
        info!("rewards distributed: {}", msg);
    }
    serde_json::to_string_pretty(&response_text).map_err(|e| TransferError::GenericError {
        error_code: 10112u64.into(),
        message: e.to_string(),
    })
}

#[named]
pub fn do_provider_check_in(
    ledger: &mut LedgerMap,
    pubkey_bytes: Vec<u8>,
    memo: String,
    nonce_signature: Vec<u8>,
) -> Result<String, String> {
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes)?;
    fn_info!("{}", dcc_id);

    // Check the max length of the memo
    if memo.len() > VALIDATION_MEMO_BYTES_MAX {
        return Err(format!(
            "Memo too long, max length is {} bytes",
            VALIDATION_MEMO_BYTES_MAX
        ));
    }

    // Check that the Provider is already registered
    ledger
        .get(LABEL_PROV_REGISTER, &pubkey_bytes)
        .map_err(|e| format!("Provider not yet registered in the Ledger: {}", e))?;

    // Verify the signature
    dcc_id.verify_bytes(&ledger.get_latest_block_hash(), &nonce_signature)?;

    let fees = if ledger.get_blocks_count() > 0 {
        let amount = check_in_fee_e9s();
        info!(
            "Charging {} tokens {} for the check in",
            amount_as_string(amount as TokenAmountE9s),
            dcc_id.to_ic_principal()
        );
        let dcc_id_text = dcc_id.to_ic_principal().to_text();
        let dcc_id_short = dcc_id_text
            .split_once('-')
            .map(|(first, _)| first)
            .unwrap_or(&dcc_id_text);
        let mut fee_memo = format!(
            "check-in-{}-{}-{}",
            dcc_id_short,
            ledger.get_blocks_count(),
            memo
        );
        fee_memo.truncate(TRANSFER_MEMO_BYTES_MAX);
        charge_fees_to_account_no_bump_reputation(
            ledger,
            &dcc_id.as_icrc_compatible_account(),
            amount as TokenAmountE9s,
            &fee_memo,
        )?;
        amount
    } else {
        0
    };

    let payload = CheckInPayload::new(memo, nonce_signature);
    let payload_bytes = payload.to_bytes().map_err(|e| {
        format!(
            "Failed to serialize check-in payload: {}. {}",
            e,
            "This is an internal error and should not happen"
        )
    })?;

    Ok(ledger
        .upsert(LABEL_PROV_CHECK_IN, pubkey_bytes, &payload_bytes)
        .map(|_| {
            format!(
                "Signature verified, check in successful. You have been charged {} tokens",
                amount_as_string(fees)
            )
        })?)
}

pub fn rewards_current_block_checked_in(ledger: &LedgerMap) -> usize {
    ledger.next_block_iter(Some(LABEL_PROV_CHECK_IN)).count()
}

#[cfg(test)]
mod tests_rewards;
