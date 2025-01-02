use crate::account_transfers::FundsTransfer;
use crate::cache_transactions::RecentCache;
use crate::{
    account_balance_add, account_balance_sub, account_balances_clear, contracts_cache_open_add,
    contracts_cache_open_remove, dcc_identity, error, reputations_apply_aging,
    reputations_apply_changes, reputations_clear, AHashMap, ContractSignRequest,
    ContractSignRequestPayload, ReputationAge, ReputationChange, CACHE_TXS_NUM_COMMITTED,
    LABEL_CONTRACT_SIGN_REPLY, LABEL_CONTRACT_SIGN_REQUEST, LABEL_DC_TOKEN_TRANSFER,
    LABEL_NP_REGISTER, LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE, PRINCIPAL_MAP,
};
use borsh::BorshDeserialize;
use candid::Principal;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::{debug, warn, LedgerMap};
use std::collections::HashMap;

pub fn refresh_caches_from_ledger(ledger: &LedgerMap) -> anyhow::Result<()> {
    if ledger.get_blocks_count() == 0 {
        return Ok(());
    }
    let mut replayed_blocks = 0;
    account_balances_clear();
    reputations_clear();
    let mut num_txs = 0u64;
    let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();
    for block in ledger.iter_raw() {
        let (_blk_head, block) = block?;
        for entry in block.entries() {
            match entry.label() {
                LABEL_REPUTATION_CHANGE => {
                    let reputation_change: ReputationChange =
                        BorshDeserialize::try_from_slice(entry.value()).map_err(|e| {
                            error!(
                                "Failed to deserialize reputation change {:?} ==> {:?}",
                                entry, e
                            );
                            e
                        })?;

                    reputations_apply_changes(&reputation_change);
                }
                LABEL_REPUTATION_AGE => {
                    let reputation_age: ReputationAge =
                        BorshDeserialize::try_from_slice(entry.value()).map_err(|e| {
                            error!(
                                "Failed to deserialize reputation age {:?} ==> {:?}",
                                entry, e
                            );
                            e
                        })?;
                    reputations_apply_aging(&reputation_age);
                }
                LABEL_DC_TOKEN_TRANSFER => {
                    let transfer: FundsTransfer = BorshDeserialize::try_from_slice(entry.value())
                        .map_err(|e| {
                        error!("Failed to deserialize transfer {:?} ==> {:?}", entry, e);
                        e
                    })?;

                    if !transfer.from().is_minting_account() {
                        let amount = transfer.amount() + transfer.fee().unwrap_or_default();
                        account_balance_sub(transfer.from(), amount)?;
                    }

                    if !transfer.to().is_minting_account() {
                        account_balance_add(transfer.to(), transfer.amount())?;
                    }

                    RecentCache::add_entry(num_txs, transfer.into());
                    num_txs += 1;
                }
                LABEL_NP_REGISTER => {
                    if let Ok(dcc_identity) =
                        dcc_identity::DccIdentity::new_verifying_from_bytes(entry.value())
                    {
                        principals.insert(dcc_identity.to_ic_principal(), entry.key().to_vec());
                    }
                }
                LABEL_CONTRACT_SIGN_REQUEST => {
                    let contract_id = entry.key();
                    let payload = match ContractSignRequestPayload::try_from_slice(entry.value()) {
                        Ok(payload) => payload,
                        Err(e) => {
                            warn!("Failed to deserialize contract sign request payload: {}", e);
                            continue;
                        }
                    };

                    let contract_req =
                        match ContractSignRequest::try_from_slice(payload.payload_serialized()) {
                            Ok(contract_req) => contract_req,
                            Err(e) => {
                                warn!("Failed to deserialize contract sign request: {}", e);
                                continue;
                            }
                        };
                    contracts_cache_open_add(contract_id.to_vec(), contract_req);
                }
                LABEL_CONTRACT_SIGN_REPLY => {
                    let contract_id = entry.key();
                    contracts_cache_open_remove(contract_id);
                }
                _ => {}
            }
        }
        replayed_blocks += 1;
    }
    CACHE_TXS_NUM_COMMITTED.with(|n| *n.borrow_mut() = num_txs);
    PRINCIPAL_MAP.with(|p| *p.borrow_mut() = principals);
    debug!("Refreshed caches from {} ledger blocks", replayed_blocks);
    Ok(())
}
