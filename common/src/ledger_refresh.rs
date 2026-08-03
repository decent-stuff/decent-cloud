use crate::account_transfer_approvals::{approval_update, FundsTransferApproval};
use crate::account_transfers::FundsTransfer;
use crate::cache_transactions::RecentCache;
use crate::{
    account_balance_add, account_balance_sub, account_balances_clear, dcc_identity, error,
    reputations_apply_aging, reputations_apply_changes, reputations_clear, set_num_providers,
    set_num_users, AHashMap, ReputationAge, ReputationChange,
    LABEL_DC_TOKEN_APPROVAL, LABEL_DC_TOKEN_TRANSFER,
    LABEL_PROV_REGISTER, LABEL_REPUTATION_AGE, LABEL_REPUTATION_CHANGE,
    LABEL_USER_REGISTER, PRINCIPAL_MAP,
};
use borsh::BorshDeserialize;
use candid::Principal;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::{debug, LedgerEntry, LedgerMap};
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        account_balance_get, reputations_clear, DccIdentity, IcrcCompatibleAccount, MINTING_ACCOUNT,
    };
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
            crate::IcrcCompatibleAccount::from(to),
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
            crate::IcrcCompatibleAccount::from(to),
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
            crate::IcrcCompatibleAccount::from(to1),
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
            crate::IcrcCompatibleAccount::from(to2),
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

    #[test]
    fn test_process_entry_for_caches_provider_register_increments_counter() {
        crate::account_balances_clear();
        reputations_clear();

        let mut num_providers = 0u64;
        let mut num_users = 0u64;
        let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

        let dcc_id = DccIdentity::new_from_seed(b"test-provider").unwrap();
        let pubkey_bytes = dcc_id.to_bytes_verifying();
        let entry = LedgerEntry::new(
            LABEL_PROV_REGISTER,
            &pubkey_bytes,
            b"signature_data",
            Operation::Upsert,
        );

        let result =
            process_entry_for_caches(&entry, &mut num_providers, &mut num_users, &mut principals);

        assert!(result.is_ok());
        assert_eq!(num_providers, 1);
        assert_eq!(num_users, 0);
        assert_eq!(principals.len(), 1);
    }

    #[test]
    fn test_process_entry_for_caches_user_register_increments_counter() {
        crate::account_balances_clear();
        reputations_clear();

        let mut num_providers = 0u64;
        let mut num_users = 0u64;
        let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

        let dcc_id = DccIdentity::new_from_seed(b"test-user").unwrap();
        let pubkey_bytes = dcc_id.to_bytes_verifying();
        let entry = LedgerEntry::new(
            LABEL_USER_REGISTER,
            &pubkey_bytes,
            b"signature_data",
            Operation::Upsert,
        );

        let result =
            process_entry_for_caches(&entry, &mut num_providers, &mut num_users, &mut principals);

        assert!(result.is_ok());
        assert_eq!(num_providers, 0);
        assert_eq!(num_users, 1);
        assert_eq!(principals.len(), 1);
    }

    #[test]
    fn test_process_entry_for_caches_register_bad_key_skipped() {
        crate::account_balances_clear();
        reputations_clear();

        let mut num_providers = 0u64;
        let mut num_users = 0u64;
        let mut principals: AHashMap<Principal, Vec<u8>> = HashMap::default();

        // Key too short — not a valid ed25519 public key
        let entry = LedgerEntry::new(
            LABEL_PROV_REGISTER,
            b"short",
            b"signature_data",
            Operation::Upsert,
        );

        let result =
            process_entry_for_caches(&entry, &mut num_providers, &mut num_users, &mut principals);

        // Should succeed but skip the entry (debug log, not error)
        assert!(result.is_ok());
        assert_eq!(num_providers, 0);
        assert!(principals.is_empty());
    }

    #[test]
    fn test_refresh_ledger_and_caches_multi_block() {
        crate::account_balances_clear();
        reputations_clear();

        let mut ledger = new_temp_ledger();

        // Block 1: mint to account A
        let to_a = Account {
            owner: Principal::from_slice(&[1u8; 29]),
            subaccount: None,
        };
        let transfer_a = crate::FundsTransfer::new(
            MINTING_ACCOUNT,
            crate::IcrcCompatibleAccount::from(to_a),
            None,
            None,
            Some(0),
            vec![],
            1000,
            0,
            1000,
        );
        ledger
            .upsert(
                LABEL_DC_TOKEN_TRANSFER,
                transfer_a.to_tx_id(),
                borsh::to_vec(&transfer_a).unwrap(),
            )
            .unwrap();
        ledger.commit_block().unwrap();

        // Block 2: mint to account B
        let to_b = Account {
            owner: Principal::from_slice(&[2u8; 29]),
            subaccount: None,
        };
        let transfer_b = crate::FundsTransfer::new(
            MINTING_ACCOUNT,
            crate::IcrcCompatibleAccount::from(to_b),
            None,
            None,
            Some(1),
            vec![],
            2000,
            0,
            2000,
        );
        ledger
            .upsert(
                LABEL_DC_TOKEN_TRANSFER,
                transfer_b.to_tx_id(),
                borsh::to_vec(&transfer_b).unwrap(),
            )
            .unwrap();
        ledger.commit_block().unwrap();

        assert_eq!(ledger.get_blocks_count(), 2);

        let result = refresh_ledger_and_caches(&mut ledger);

        assert!(result.is_ok());
        assert_eq!(
            account_balance_get(&IcrcCompatibleAccount::from(to_a)),
            1000
        );
        assert_eq!(
            account_balance_get(&IcrcCompatibleAccount::from(to_b)),
            2000
        );
    }
}
