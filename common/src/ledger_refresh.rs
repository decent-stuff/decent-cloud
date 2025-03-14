use crate::account_transfer_approvals::{approval_update, FundsTransferApproval};
use crate::account_transfers::FundsTransfer;
use crate::cache_transactions::RecentCache;
use crate::{
    account_balance_add, account_balance_sub, account_balances_clear, contracts_cache_open_add,
    contracts_cache_open_remove, dcc_identity, error, reputations_apply_aging,
    reputations_apply_changes, reputations_clear, set_num_providers, set_num_users,
    set_offering_num_per_provider, AHashMap, CheckInPayload, ContractSignReplyPayload,
    ContractSignRequest, ContractSignRequestPayload, DccIdentity, ReputationAge, ReputationChange,
    UpdateOfferingPayload, UpdateProfilePayload, LABEL_CONTRACT_SIGN_REPLY,
    LABEL_CONTRACT_SIGN_REQUEST, LABEL_DC_TOKEN_APPROVAL, LABEL_DC_TOKEN_TRANSFER,
    LABEL_NP_CHECK_IN, LABEL_NP_OFFERING, LABEL_NP_PROFILE, LABEL_NP_REGISTER,
    LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE, LABEL_REWARD_DISTRIBUTION, LABEL_USER_REGISTER,
    PRINCIPAL_MAP,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::BorshDeserialize;
use candid::Principal;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::{debug, warn, LedgerBlock, LedgerEntry, LedgerMap};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

pub fn refresh_caches_from_ledger(ledger: &LedgerMap) -> anyhow::Result<()> {
    if ledger.get_blocks_count() == 0 {
        return Ok(());
    }
    let mut replayed_blocks = 0;
    account_balances_clear();
    reputations_clear();
    let mut num_providers = 0u64;
    let mut num_users = 0u64;
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
    PRINCIPAL_MAP.with(|p| *p.borrow_mut() = principals);
    set_num_providers(num_providers);
    set_num_users(num_users);
    debug!(
        "Refreshed caches from {} ledger blocks, found {} transactions",
        replayed_blocks,
        RecentCache::get_max_tx_num().unwrap_or_default()
    );
    Ok(())
}

#[derive(Serialize)]
pub struct WasmLedgerEntry {
    pub label: String,
    pub key: Value,
    pub value: Value,
    pub description: String,
}

impl WasmLedgerEntry {
    fn from_dc_token_approval(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_DC_TOKEN_APPROVAL.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: serde_json::to_value(
                FundsTransferApproval::try_from_slice(entry.value()).unwrap(),
            )
            .unwrap(),
            description: "ICRC2 FundsTransferApproval".to_string(),
        }
    }

    fn from_dc_token_transfer(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_DC_TOKEN_TRANSFER.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: serde_json::to_value(FundsTransfer::try_from_slice(entry.value()).unwrap())
                .unwrap(),
            description: "ICRC1 FundsTransfer".to_string(),
        }
    }

    fn from_np_check_in(entry: &LedgerEntry, parent_hash: &[u8]) -> Self {
        let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
        WasmLedgerEntry {
            label: LABEL_NP_CHECK_IN.to_string(),
            key: Value::String(dcc_id.to_string()),
            value: match CheckInPayload::try_from_slice(entry.value()) {
                Ok(payload) => serde_json::json!({
                    "parent_hash": BASE64.encode(parent_hash),
                    "signature": BASE64.encode(payload.nonce_signature()),
                    "verified": match dcc_id.verify_bytes(parent_hash, payload.nonce_signature()) {
                        Ok(()) => "true".into(),
                        Err(e) => {
                            format!("Signature verification failed: {}", e)
                        }
                    },
                    "memo": payload.memo(),
                }),
                Err(e) => {
                    serde_json::json!(format!(
                        "Failed to deserialize check in payload: {} ({})",
                        BASE64.encode(entry.value()),
                        e
                    ))
                }
            },
            description: "Provider CheckIn".to_string(),
        }
    }

    fn from_np_offering(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_NP_OFFERING.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: match UpdateOfferingPayload::try_from_slice(entry.value()) {
                Ok(payload) => match payload.deserialize_update_offering() {
                    Ok(offering) => serde_json::to_value(&offering).unwrap(),
                    Err(e) => {
                        serde_json::json!(format!(
                            "Failed to deserialize update offering payload: {} ({})",
                            BASE64.encode(entry.value()),
                            e
                        ))
                    }
                },
                Err(e) => {
                    serde_json::json!(format!(
                        "Failed to deserialize update offering payload: {} ({})",
                        BASE64.encode(entry.value()),
                        e
                    ))
                }
            },
            description: "Provider Offering Update".to_string(),
        }
    }

    fn from_np_profile(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_NP_PROFILE.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: match UpdateProfilePayload::try_from_slice(entry.value()) {
                Ok(payload) => match payload.deserialize_update_profile() {
                    Ok(profile) => serde_json::to_value(&profile).unwrap(),
                    Err(e) => {
                        serde_json::json!(format!(
                            "Failed to deserialize update profile payload: {} ({})",
                            BASE64.encode(entry.value()),
                            e
                        ))
                    }
                },
                Err(e) => {
                    serde_json::json!(format!(
                        "Failed to deserialize update profile payload: {} ({})",
                        BASE64.encode(entry.value()),
                        e
                    ))
                }
            },
            description: "Provider Profile Update".to_string(),
        }
    }

    fn from_account_register(entry: &LedgerEntry, parent_hash: &[u8]) -> Self {
        let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
        WasmLedgerEntry {
            label: entry.label().to_string(),
            key: Value::String(dcc_id.to_string()),
            value: serde_json::json!({
                "parent_hash": BASE64.encode(parent_hash),
                "signature": BASE64.encode(entry.value()),
                "verified": match dcc_id.verify_bytes(parent_hash, entry.value()) {
                    Ok(()) => "true".into(),
                    Err(e) => {
                        format!("Signature verification failed: {}", e)
                    }
                },
            }),
            description: "Account Register".to_string(),
        }
    }

    fn from_reputation_age(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_REPUTATION_AGE.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: serde_json::to_value(ReputationAge::try_from_slice(entry.value()).unwrap())
                .unwrap(),
            description: "ReputationAge".to_string(),
        }
    }

    fn from_reputation_change(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_REPUTATION_CHANGE.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: serde_json::to_value(ReputationChange::try_from_slice(entry.value()).unwrap())
                .unwrap(),
            description: "ReputationChange".to_string(),
        }
    }

    fn from_reward_distribution(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_REWARD_DISTRIBUTION.to_string(),
            key: Value::String(match std::str::from_utf8(entry.key()) {
                Ok(s) => s.to_string(),
                Err(_) => BASE64.encode(entry.key()),
            }),
            value: serde_json::to_value(u64::from_le_bytes(
                <[u8; 8]>::try_from_slice(entry.value())
                    .map_err(|e| {
                        format!(
                            "Failed to deserialize reward distribution value: {} ({})",
                            BASE64.encode(entry.value()),
                            e
                        )
                    })
                    .unwrap(),
            ))
            .unwrap(),
            description: "RewardDistribution".to_string(),
        }
    }

    fn from_generic(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: entry.label().to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: serde_json::to_value(BASE64.encode(entry.value())).unwrap(),
            description: "Generic".to_string(),
        }
    }

    fn from_contract_sign_request(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_CONTRACT_SIGN_REQUEST.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: match ContractSignRequestPayload::try_from_slice(entry.value()) {
                Ok(payload) => match payload.deserialize_contract_sign_request() {
                    Ok(contract_req) => serde_json::to_value(&contract_req).unwrap(),
                    Err(e) => {
                        serde_json::json!(format!(
                            "Failed to deserialize contract sign request: {} ({})",
                            BASE64.encode(entry.value()),
                            e
                        ))
                    }
                },
                Err(e) => {
                    serde_json::json!(format!(
                        "Failed to deserialize contract sign request payload: {} ({})",
                        BASE64.encode(entry.value()),
                        e
                    ))
                }
            },
            description: "Contract Sign Request".to_string(),
        }
    }

    fn from_contract_sign_reply(entry: &LedgerEntry) -> Self {
        WasmLedgerEntry {
            label: LABEL_CONTRACT_SIGN_REPLY.to_string(),
            key: Value::String(BASE64.encode(entry.key())),
            value: match ContractSignReplyPayload::try_from_slice(entry.value()) {
                Ok(payload) => match payload.deserialize_contract_sign_reply() {
                    Ok(contract_reply) => serde_json::to_value(&contract_reply).unwrap(),
                    Err(e) => {
                        serde_json::json!(format!(
                            "Failed to deserialize contract sign reply: {} ({})",
                            BASE64.encode(entry.value()),
                            e
                        ))
                    }
                },
                Err(e) => {
                    serde_json::json!(format!(
                        "Failed to deserialize contract sign reply payload: {} ({})",
                        BASE64.encode(entry.value()),
                        e
                    ))
                }
            },
            description: "Contract Sign Reply".to_string(),
        }
    }
}

pub fn ledger_block_parse_entries(block: &LedgerBlock) -> Vec<WasmLedgerEntry> {
    let mut entries = vec![];
    for entry in block.entries() {
        entries.push(match entry.label() {
            LABEL_DC_TOKEN_APPROVAL => WasmLedgerEntry::from_dc_token_approval(entry),
            LABEL_DC_TOKEN_TRANSFER => WasmLedgerEntry::from_dc_token_transfer(entry),
            LABEL_NP_CHECK_IN => WasmLedgerEntry::from_np_check_in(entry, block.parent_hash()),
            LABEL_NP_OFFERING => WasmLedgerEntry::from_np_offering(entry),
            LABEL_NP_PROFILE => WasmLedgerEntry::from_np_profile(entry),
            LABEL_NP_REGISTER => WasmLedgerEntry::from_account_register(entry, block.parent_hash()),
            LABEL_REPUTATION_AGE => WasmLedgerEntry::from_reputation_age(entry),
            LABEL_REPUTATION_CHANGE => WasmLedgerEntry::from_reputation_change(entry),
            LABEL_REWARD_DISTRIBUTION => WasmLedgerEntry::from_reward_distribution(entry),
            LABEL_USER_REGISTER => {
                WasmLedgerEntry::from_account_register(entry, block.parent_hash())
            }
            LABEL_CONTRACT_SIGN_REQUEST => WasmLedgerEntry::from_contract_sign_request(entry),
            LABEL_CONTRACT_SIGN_REPLY => WasmLedgerEntry::from_contract_sign_reply(entry),
            _ => WasmLedgerEntry::from_generic(entry),
        })
    }
    entries
}
