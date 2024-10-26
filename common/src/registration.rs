use crate::{
    amount_as_string, charge_fees_to_account_and_bump_reputation, info, reward_e9s_per_block,
    AHashMap, Balance, DccIdentity, LABEL_NP_REGISTER, LABEL_USER_REGISTER, MAX_PUBKEY_BYTES,
};
use candid::Principal;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    pub static PRINCIPAL_MAP: RefCell<AHashMap<Principal, Vec<u8>>> = RefCell::new(HashMap::default());
}

pub fn get_pubkey_from_principal(principal: Principal) -> Vec<u8> {
    PRINCIPAL_MAP.with(|principal_map| {
        principal_map
            .borrow()
            .get(&principal)
            .cloned()
            .unwrap_or_default()
    })
}

pub fn np_registration_fee_e9s() -> Balance {
    reward_e9s_per_block() / 100
}

// To prevent DOS attacks, the NP is charged 1/100 of the block reward for executing this operation
pub fn do_node_provider_register(
    ledger: &mut LedgerMap,
    caller: Principal,
    np_pubkey_bytes: Vec<u8>, // both are the same
) -> Result<String, String> {
    if np_pubkey_bytes.len() > MAX_PUBKEY_BYTES {
        return Err("Provided public key too long".to_string());
    }
    let dcc_identity = DccIdentity::new_verifying_from_bytes(&np_pubkey_bytes).unwrap();
    if dcc_identity.to_ic_principal() != caller {
        return Err("Invalid caller".to_string());
    }
    info!("[do_node_provider_register]: {}", dcc_identity);

    match ledger.get(LABEL_NP_REGISTER, &np_pubkey_bytes) {
        Ok(_) => {
            info!("Node provider already registered");
            Err("Node provider already registered".to_string())
        }
        Err(ledger_map::LedgerError::EntryNotFound) => {
            account_register(ledger, LABEL_NP_REGISTER, np_pubkey_bytes)
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn do_user_register(
    ledger: &mut LedgerMap,
    caller: Principal,
    user_pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    info!("[do_user_register]: caller: {}", caller);
    if user_pubkey_bytes.len() > MAX_PUBKEY_BYTES {
        return Err("Provided public key too long".to_string());
    }

    match ledger.get(LABEL_USER_REGISTER, &user_pubkey_bytes) {
        Ok(_) => {
            info!("User already registered");
            Err("User already registered".to_string())
        }
        Err(ledger_map::LedgerError::EntryNotFound) => {
            account_register(ledger, LABEL_USER_REGISTER, user_pubkey_bytes)
        }
        Err(e) => Err(e.to_string()),
    }
}

fn account_register(
    ledger: &mut LedgerMap,
    label: &str,
    pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    let dcc_identity =
        DccIdentity::new_verifying_from_bytes(&pubkey_bytes).map_err(|e| e.to_string())?;

    let fees = if ledger.get_blocks_count() > 0 {
        let amount = np_registration_fee_e9s();
        info!(
            "Charging {} tokens from {} for {} registration",
            amount_as_string(amount),
            dcc_identity.to_ic_principal(),
            label
        );
        charge_fees_to_account_and_bump_reputation(ledger, &dcc_identity, amount as Balance)?;
        amount
    } else {
        0
    };

    // Update the cache of principal -> pubkey
    PRINCIPAL_MAP.with(|p| {
        p.borrow_mut()
            .insert(dcc_identity.to_ic_principal(), pubkey_bytes.clone())
    });

    // Store the pubkey in the ledger
    ledger
        .upsert(label, pubkey_bytes, vec![])
        .map(|_| {
            format!(
                "Registration complete! Thank you. You have been charged {} tokens",
                amount_as_string(fees)
            )
        })
        .map_err(|e| e.to_string())
}
