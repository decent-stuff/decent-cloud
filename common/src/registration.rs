use crate::{
    amount_as_string_u64, charge_fees_to_account_and_bump_reputation, info, reward_e9s_per_block,
    slice_to_32_bytes_array, AHashMap, DccIdentity, LABEL_NP_REGISTER, LABEL_USER_REGISTER,
};
use candid::Principal;
use ed25519_dalek::VerifyingKey;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    pub static PRINCIPAL_MAP: RefCell<AHashMap<Principal, Vec<u8>>> = RefCell::new(HashMap::default());
}

pub fn get_uid_from_principal(principal: Principal) -> Vec<u8> {
    PRINCIPAL_MAP.with(|principal_map| {
        principal_map
            .borrow()
            .get(&principal)
            .cloned()
            .unwrap_or_default()
    })
}

pub fn np_registration_fee_e9s() -> u64 {
    reward_e9s_per_block() / 100
}

// To prevent DOS attacks, the NP is charged 1/100 of the block reward for executing this operation
pub fn do_node_provider_register(
    ledger: &mut LedgerMap,
    caller: Principal,
    np_uid_bytes: Vec<u8>, // both are the same
    np_pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    if np_uid_bytes.len() > 64 {
        return Err("Node provider uid too long".to_string());
    }
    if np_pubkey_bytes.len() > 64 {
        return Err("Node provider public key too long".to_string());
    }
    let dcc_identity = DccIdentity::new_verifying_from_bytes(&np_pubkey_bytes).unwrap();
    if dcc_identity.to_ic_principal() != caller {
        return Err("Invalid caller".to_string());
    }
    println!("[do_node_provider_register]: {}", dcc_identity);

    match ledger.get(LABEL_NP_REGISTER, &np_uid_bytes) {
        Ok(_) => {
            info!("Node provider already registered");
            Err("Node provider already registered".to_string())
        }
        Err(ledger_map::LedgerError::EntryNotFound) => {
            account_register(ledger, LABEL_NP_REGISTER, np_uid_bytes, np_pubkey_bytes)
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn do_user_register(
    ledger: &mut LedgerMap,
    caller: Principal,
    user_uid: Vec<u8>,
    user_pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    println!("[do_user_register]: caller: {}", caller);
    match ledger.get(LABEL_USER_REGISTER, &user_uid) {
        Ok(_) => {
            info!("User already registered");
            Err("User already registered".to_string())
        }
        Err(ledger_map::LedgerError::EntryNotFound) => {
            account_register(ledger, LABEL_USER_REGISTER, user_uid, user_pubkey_bytes)
        }
        Err(e) => Err(e.to_string()),
    }
}

fn account_register(
    ledger: &mut LedgerMap,
    label: &str,
    identity_uid: Vec<u8>,
    identity_pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    let identity_pubkey_bytes = slice_to_32_bytes_array(&identity_pubkey_bytes)?;
    let verifying_key =
        VerifyingKey::from_bytes(identity_pubkey_bytes).map_err(|e| e.to_string())?;
    let dcc_identity = DccIdentity::new_verifying(&verifying_key).map_err(|e| e.to_string())?;

    if ledger.get_blocks_count() > 0 {
        let amount = np_registration_fee_e9s();
        info!(
            "Charging {} tokens from {} for {} registration",
            amount_as_string_u64(amount),
            dcc_identity.to_ic_principal(),
            label
        );
        charge_fees_to_account_and_bump_reputation(ledger, &dcc_identity, amount)?;
    }

    PRINCIPAL_MAP.with(|p| {
        p.borrow_mut()
            .insert(dcc_identity.to_ic_principal(), identity_uid.clone())
    });

    ledger
        .upsert(label, &identity_uid, identity_pubkey_bytes)
        .map(|_| "ok".to_string())
        .map_err(|e| e.to_string())
}
