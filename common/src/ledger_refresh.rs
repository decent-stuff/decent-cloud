use crate::account_transfer_approvals::{approval_update, FundsTransferApproval};
use crate::account_transfers::FundsTransfer;
use crate::cache_transactions::RecentCache;
use crate::{
    account_balance_add, account_balance_sub, account_balances_clear, dcc_identity, error,
    reputations_apply_aging, reputations_apply_changes, reputations_clear, set_num_providers,
    set_num_users, AHashMap, CheckInPayload, DccIdentity, ReputationAge, ReputationChange,
    LABEL_DC_TOKEN_APPROVAL, LABEL_DC_TOKEN_TRANSFER, LABEL_NP_CHECK_IN, LABEL_NP_REGISTER,
    LABEL_PROV_CHECK_IN, LABEL_PROV_REGISTER, LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE,
    LABEL_REWARD_DISTRIBUTION, LABEL_USER_REGISTER, PRINCIPAL_MAP,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::BorshDeserialize;
use candid::Principal;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::{debug, LedgerBlock, LedgerEntry, LedgerMap};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

fn process_entry_for_caches(
    entry: &LedgerEntry,
    num_providers: &mut u64,
    num_users: &mut u64,
    principals: &mut AHashMap<Principal, Vec<u8>>,
) -> anyhow::Result<()> {
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
            let reputation_age: ReputationAge = BorshDeserialize::try_from_slice(entry.value())
                .map_err(|e| {
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
            let approval = FundsTransferApproval::deserialize(entry.value()).map_err(|e| {
                error!("Failed to deserialize approval {:?} ==> {:?}", entry, e);
                e
            })?;
            approval_update(
                approval.approver().into(),
                approval.spender().into(),
                approval.allowance(),
            );
        }
        LABEL_PROV_REGISTER | LABEL_USER_REGISTER => {
            match dcc_identity::DccIdentity::new_verifying_from_bytes(entry.key())
                .and_then(|id| id.to_ic_principal().map(|p| (id, p)))
            {
                Ok((_, principal)) => {
                    if entry.label() == LABEL_PROV_REGISTER {
                        *num_providers += 1;
                    } else if entry.label() == LABEL_USER_REGISTER {
                        *num_users += 1;
                    }
                    principals.insert(principal, entry.key().to_vec());
                }
                Err(e) => {
                    debug!("Skipping entry with bad key during replay: {e}");
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn refresh_ledger_and_caches(ledger: &mut LedgerMap) -> anyhow::Result<()> {
    if ledger.get_blocks_count() == 0 {
        return Ok(());
    }

    account_balances_clear();
    reputations_clear();

    let mut num_providers = 0u64;
    let mut num_users = 0u64;
    let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

    ledger.refresh_ledger_with_callback(|entry| {
        process_entry_for_caches(entry, &mut num_providers, &mut num_users, &mut principals)
    })?;

    PRINCIPAL_MAP.with(|p| *p.borrow_mut() = principals);
    set_num_providers(num_providers);
    set_num_users(num_users);
    debug!(
        "Refreshed ledger and caches, found {} transactions",
        RecentCache::get_max_tx_num().unwrap_or_default()
    );
    Ok(())
}

pub fn refresh_caches_from_ledger(ledger: &LedgerMap) -> anyhow::Result<()> {
    if ledger.get_blocks_count() == 0 {
        return Ok(());
    }

    account_balances_clear();
    reputations_clear();

    let mut num_providers = 0u64;
    let mut num_users = 0u64;
    let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

    for block in ledger.iter_raw(0) {
        let (_blk_head, block) = block?;
        for entry in block.entries() {
            process_entry_for_caches(entry, &mut num_providers, &mut num_users, &mut principals)?;
        }
    }

    PRINCIPAL_MAP.with(|p| *p.borrow_mut() = principals);
    set_num_providers(num_providers);
    set_num_users(num_users);
    debug!(
        "Refreshed caches from ledger blocks, found {} transactions",
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

    fn from_provider_check_in(entry: &LedgerEntry, parent_hash: &[u8]) -> Self {
        let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
        WasmLedgerEntry {
            label: LABEL_PROV_CHECK_IN.to_string(),
            key: Value::String(dcc_id.to_string()),
            value: match CheckInPayload::try_from_slice(entry.value()) {
                Ok(payload) => serde_json::json!({
                    "parent_hash": BASE64.encode(parent_hash),
                    "signature": BASE64.encode(payload.nonce_signature()),
                    "verified": match dcc_id.verify_bytes(parent_hash, payload.nonce_signature()) {
                        Ok(()) => "verified".into(),
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

    fn from_account_register(entry: &LedgerEntry, parent_hash: &[u8]) -> Self {
        let dcc_id = DccIdentity::new_verifying_from_bytes(entry.key()).unwrap();
        WasmLedgerEntry {
            label: entry.label().to_string(),
            key: Value::String(dcc_id.to_string()),
            value: serde_json::json!({
                "parent_hash": BASE64.encode(parent_hash),
                "signature": BASE64.encode(entry.value()),
                "verified": match dcc_id.verify_bytes(entry.key(), entry.value()) {
                    Ok(()) => "verified".into(),
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
}

pub fn ledger_block_parse_entries(block: &LedgerBlock) -> Vec<WasmLedgerEntry> {
    let mut entries = vec![];
    for entry in block.entries() {
        entries.push(match entry.label() {
            LABEL_DC_TOKEN_APPROVAL => WasmLedgerEntry::from_dc_token_approval(entry),
            LABEL_DC_TOKEN_TRANSFER => WasmLedgerEntry::from_dc_token_transfer(entry),
            LABEL_PROV_CHECK_IN | LABEL_NP_CHECK_IN => {
                WasmLedgerEntry::from_provider_check_in(entry, block.parent_hash())
            }
            LABEL_PROV_REGISTER | LABEL_NP_REGISTER => {
                WasmLedgerEntry::from_account_register(entry, block.parent_hash())
            }
            LABEL_REPUTATION_AGE => WasmLedgerEntry::from_reputation_age(entry),
            LABEL_REPUTATION_CHANGE => WasmLedgerEntry::from_reputation_change(entry),
            LABEL_REWARD_DISTRIBUTION => WasmLedgerEntry::from_reward_distribution(entry),
            LABEL_USER_REGISTER => {
                WasmLedgerEntry::from_account_register(entry, block.parent_hash())
            }
            _ => WasmLedgerEntry::from_generic(entry),
        })
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{account_balance_get, reputations_clear, IcrcCompatibleAccount, MINTING_ACCOUNT};
    use candid::Principal;
    use icrc_ledger_types::icrc1::account::Account;
    use ledger_map::{LedgerEntry, LedgerMap, Operation};

    fn new_temp_ledger() -> LedgerMap {
        let file_path = tempfile::tempdir()
            .unwrap()
            .path()
            .join("test_ledger_store.bin");
        LedgerMap::new_with_path(None, Some(file_path)).expect("Failed to create temp ledger")
    }

    #[test]
    fn test_process_entry_for_caches_unknown_label_is_noop() {
        let mut num_providers = 0u64;
        let mut num_users = 0u64;
        let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

        let entry = LedgerEntry::new("UnknownLabel", b"key", b"value", Operation::Upsert);
        let result =
            process_entry_for_caches(&entry, &mut num_providers, &mut num_users, &mut principals);

        assert!(result.is_ok());
        assert_eq!(num_providers, 0);
        assert_eq!(num_users, 0);
        assert!(principals.is_empty());
    }

    #[test]
    fn test_process_entry_for_caches_malformed_reputation_change_fails() {
        reputations_clear();
        let mut num_providers = 0u64;
        let mut num_users = 0u64;
        let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

        let entry = LedgerEntry::new(
            LABEL_REPUTATION_CHANGE,
            b"key",
            b"malformed_data",
            Operation::Upsert,
        );
        let result =
            process_entry_for_caches(&entry, &mut num_providers, &mut num_users, &mut principals);

        assert!(result.is_err());
    }

    #[test]
    fn test_process_entry_for_caches_malformed_transfer_fails() {
        crate::account_balances_clear();
        let mut num_providers = 0u64;
        let mut num_users = 0u64;
        let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

        let entry = LedgerEntry::new(
            LABEL_DC_TOKEN_TRANSFER,
            b"key",
            b"malformed_data",
            Operation::Upsert,
        );
        let result =
            process_entry_for_caches(&entry, &mut num_providers, &mut num_users, &mut principals);

        assert!(result.is_err());
    }

    #[test]
    fn test_process_entry_for_caches_valid_transfer_updates_balance() {
        crate::account_balances_clear();
        let mut num_providers = 0u64;
        let mut num_users = 0u64;
        let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

        let to = Account {
            owner: Principal::from_slice(&[1u8; 29]),
            subaccount: None,
        };
        let transfer = crate::FundsTransfer::new(
            MINTING_ACCOUNT,
            crate::IcrcCompatibleAccount::from(to.clone()),
            None,
            None,
            Some(0),
            vec![],
            1000,
            0,
            1000,
        );
        let entry = LedgerEntry::new(
            LABEL_DC_TOKEN_TRANSFER,
            transfer.to_tx_id(),
            borsh::to_vec(&transfer).unwrap(),
            Operation::Upsert,
        );

        let result =
            process_entry_for_caches(&entry, &mut num_providers, &mut num_users, &mut principals);

        assert!(result.is_ok());
        assert_eq!(account_balance_get(&IcrcCompatibleAccount::from(to)), 1000);
    }

    #[test]
    fn test_refresh_ledger_and_caches_empty_ledger() {
        let mut ledger = new_temp_ledger();
        assert_eq!(ledger.get_blocks_count(), 0);

        let result = refresh_ledger_and_caches(&mut ledger);
        assert!(result.is_ok());
    }

    #[test]
    fn test_refresh_ledger_and_caches_with_valid_entries() {
        crate::account_balances_clear();
        reputations_clear();

        let mut ledger = new_temp_ledger();

        let to = Account {
            owner: Principal::from_slice(&[1u8; 29]),
            subaccount: None,
        };
        let transfer = crate::FundsTransfer::new(
            MINTING_ACCOUNT,
            crate::IcrcCompatibleAccount::from(to.clone()),
            None,
            None,
            Some(0),
            vec![],
            500,
            0,
            500,
        );
        ledger
            .upsert(
                LABEL_DC_TOKEN_TRANSFER,
                transfer.to_tx_id(),
                borsh::to_vec(&transfer).unwrap(),
            )
            .unwrap();
        ledger.commit_block().unwrap();

        let result = refresh_ledger_and_caches(&mut ledger);

        assert!(result.is_ok());
        assert_eq!(account_balance_get(&IcrcCompatibleAccount::from(to)), 500);
    }

    #[test]
    fn test_refresh_ledger_and_caches_short_circuits_on_malformed_entry() {
        crate::account_balances_clear();
        reputations_clear();

        let mut ledger = new_temp_ledger();

        let to1 = Account {
            owner: Principal::from_slice(&[1u8; 29]),
            subaccount: None,
        };
        let transfer1 = crate::FundsTransfer::new(
            MINTING_ACCOUNT,
            crate::IcrcCompatibleAccount::from(to1.clone()),
            None,
            None,
            Some(0),
            vec![],
            100,
            0,
            100,
        );
        ledger
            .upsert(
                LABEL_DC_TOKEN_TRANSFER,
                transfer1.to_tx_id(),
                borsh::to_vec(&transfer1).unwrap(),
            )
            .unwrap();

        ledger
            .upsert(
                LABEL_REPUTATION_CHANGE,
                b"key",
                b"malformed_reputation_data",
            )
            .unwrap();

        let to2 = Account {
            owner: Principal::from_slice(&[2u8; 29]),
            subaccount: None,
        };
        let transfer2 = crate::FundsTransfer::new(
            MINTING_ACCOUNT,
            crate::IcrcCompatibleAccount::from(to2.clone()),
            None,
            None,
            Some(0),
            vec![],
            200,
            0,
            200,
        );
        ledger
            .upsert(
                LABEL_DC_TOKEN_TRANSFER,
                transfer2.to_tx_id(),
                borsh::to_vec(&transfer2).unwrap(),
            )
            .unwrap();

        ledger.commit_block().unwrap();

        let result = refresh_ledger_and_caches(&mut ledger);

        assert!(result.is_err());
    }
}
