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

pub fn get_unique_id_from_principal(principal: Principal) -> Vec<u8> {
    PRINCIPAL_MAP.with(|principal_map| {
        principal_map
            .borrow()
            .get(&principal)
            .cloned()
            .unwrap_or_default()
    })
}

pub fn registration_fee_e9s() -> u64 {
    reward_e9s_per_block() / 100
}

// To prevent DOS attacks, the NP is charged 1/100 of the block reward for executing this operation
pub fn do_node_provider_register(
    ledger: &mut LedgerMap,
    caller: Principal,
    np_unique_id: Vec<u8>,
    np_pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    println!("[do_node_provider_register]: caller: {}", caller);
    if np_unique_id.len() > 64 {
        return Err("Node provider unique id too long".to_string());
    }
    if np_pubkey_bytes.len() != 32 {
        return Err("Invalid Node provider public key".to_string());
    }
    match ledger.get(LABEL_NP_REGISTER, &np_unique_id) {
        Ok(_) => {
            info!("Node provider already registered");
            Err("Node provider already registered".to_string())
        }
        Err(ledger_map::LedgerError::EntryNotFound) => {
            account_register(ledger, LABEL_NP_REGISTER, np_unique_id, np_pubkey_bytes)
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn do_user_register(
    ledger: &mut LedgerMap,
    caller: Principal,
    user_unique_id: Vec<u8>,
    user_pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    println!("[do_user_register]: caller: {}", caller);
    match ledger.get(LABEL_USER_REGISTER, &user_unique_id) {
        Ok(_) => {
            info!("User already registered");
            Err("User already registered".to_string())
        }
        Err(ledger_map::LedgerError::EntryNotFound) => account_register(
            ledger,
            LABEL_USER_REGISTER,
            user_unique_id,
            user_pubkey_bytes,
        ),
        Err(e) => Err(e.to_string()),
    }
}

fn account_register(
    ledger: &mut LedgerMap,
    label: &str,
    identity_unique_id: Vec<u8>,
    identity_pubkey_bytes: Vec<u8>,
) -> Result<String, String> {
    let identity_pubkey_bytes = slice_to_32_bytes_array(&identity_pubkey_bytes)?;
    let verifying_key =
        VerifyingKey::from_bytes(identity_pubkey_bytes).map_err(|e| e.to_string())?;
    let dcc_identity = DccIdentity::new_verifying(&verifying_key).map_err(|e| e.to_string())?;

    if ledger.get_blocks_count() > 0 {
        let amount = registration_fee_e9s();
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
            .insert(dcc_identity.to_ic_principal(), identity_unique_id.clone())
    });

    ledger
        .upsert(label, &identity_unique_id, identity_pubkey_bytes)
        .map(|_| "ok".to_string())
        .map_err(|e| e.to_string())
}
