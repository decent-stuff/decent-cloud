use crate::account_transfer_approvals::{approval_update, FundsTransferApproval};
use crate::account_transfers::FundsTransfer;
use crate::cache_transactions::RecentCache;
use crate::{
    account_balance_add, account_balance_sub, account_balances_clear, contracts_cache_open_add,
    contracts_cache_open_remove, dcc_identity, error, num_providers_set, principal_map_lock,
    reputations_apply_aging, reputations_apply_changes, reputations_clear, set_num_users,
    set_offering_num_per_provider, AHashMap, ContractSignRequest, ContractSignRequestPayload,
    ReputationAge, ReputationChange, UpdateOfferingPayload, LABEL_CONTRACT_SIGN_REPLY,
    LABEL_CONTRACT_SIGN_REQUEST, LABEL_DC_TOKEN_APPROVAL, LABEL_DC_TOKEN_TRANSFER,
    LABEL_NP_OFFERING, LABEL_NP_REGISTER, LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE,
    LABEL_USER_REGISTER,
};
use borsh::BorshDeserialize;
use candid::Principal;
use futures::{pin_mut, stream::TryStreamExt};
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::{debug, warn, LedgerMap};
use std::collections::HashMap;

pub async fn refresh_caches_from_ledger(ledger: &LedgerMap) -> anyhow::Result<()> {
    if ledger.get_blocks_count() == 0 {
        return Ok(());
    }
    let mut replayed_blocks = 0;
    account_balances_clear();
    reputations_clear();
    let mut num_providers = 0u64;
    let mut num_users = 0u64;
    let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();
    {
        let stream = ledger.iter_raw();
        pin_mut!(stream);

        while let Ok(block) = stream.try_next().await {
            let (_blk_head, block) = block.expect("Failed to get block");
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
                        let transfer: FundsTransfer =
                            BorshDeserialize::try_from_slice(entry.value()).map_err(|e| {
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

                        RecentCache::append_entry(transfer.into());
                    }
                    LABEL_DC_TOKEN_APPROVAL => {
                        let approval =
                            FundsTransferApproval::deserialize(entry.value()).map_err(|e| {
                                error!("Failed to deserialize approval {:?} ==> {:?}", entry, e);
                                e
                            })?;
                        approval_update(
                            approval.approver().into(),
                            approval.spender().into(),
                            approval.allowance(),
                        );
                    }
                    LABEL_NP_REGISTER | LABEL_USER_REGISTER => {
                        if let Ok(dcc_identity) =
                            dcc_identity::DccIdentity::new_verifying_from_bytes(entry.key())
                        {
                            if entry.label() == LABEL_NP_REGISTER {
                                num_providers += 1;
                            } else if entry.label() == LABEL_USER_REGISTER {
                                num_users += 1;
                            }
                            principals.insert(dcc_identity.to_ic_principal(), entry.key().to_vec());
                        }
                    }
                    LABEL_NP_OFFERING => {
                        if let Ok(dcc_identity) =
                            dcc_identity::DccIdentity::new_verifying_from_bytes(entry.key())
                        {
                            match UpdateOfferingPayload::deserialize_unchecked(entry.value()) {
                                Ok(payload) => {
                                    set_offering_num_per_provider(
                                        entry.key().to_vec(),
                                        payload
                                            .offering()
                                            .map(|o| o.get_all_instance_ids().len() as u64)
                                            .unwrap_or_default(),
                                    );
                                }
                                Err(e) => {
                                    debug!("Failed to deserialize offering payload: {}", e);
                                    continue;
                                }
                            }
                            principals.insert(dcc_identity.to_ic_principal(), entry.key().to_vec());
                        }
                    }
                    LABEL_CONTRACT_SIGN_REQUEST => {
                        let contract_id = entry.key();
                        let payload =
                            match ContractSignRequestPayload::try_from_slice(entry.value()) {
                                Ok(payload) => payload,
                                Err(e) => {
                                    warn!(
                                        "Failed to deserialize contract sign request payload: {}",
                                        e
                                    );
                                    continue;
                                }
                            };

                        let contract_req =
                            match ContractSignRequest::try_from_slice(payload.payload_serialized())
                            {
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
    }
    *principal_map_lock() = principals;
    num_providers_set(num_providers);
    set_num_users(num_users);
    debug!(
        "Refreshed caches from {} ledger blocks, found {} transactions",
        replayed_blocks,
        RecentCache::get_max_tx_num().unwrap_or_default()
    );
    Ok(())
}
