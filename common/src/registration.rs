use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, fn_info, info,
    reward_e9s_per_block, AHashMap, DccIdentity, TokenAmountE9s, LABEL_NP_REGISTER,
    LABEL_USER_REGISTER,
};
use candid::Principal;
use function_name::named;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use once_cell::sync::OnceCell;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

static PRINCIPAL_MAP: OnceCell<Arc<Mutex<AHashMap<Principal, Vec<u8>>>>> = OnceCell::new();
pub static NUM_PROVIDERS: OnceCell<Arc<Mutex<u64>>> = OnceCell::new();
pub static NUM_USERS: OnceCell<Arc<Mutex<u64>>> = OnceCell::new();

pub fn registrations_cache_init() {
    if PRINCIPAL_MAP.get().is_none() {
        PRINCIPAL_MAP
            .set(Arc::new(Mutex::new(HashMap::default())))
            .ok();
    }
    if NUM_PROVIDERS.get().is_none() {
        NUM_PROVIDERS.set(Arc::new(Mutex::new(0))).ok();
    }
    if NUM_USERS.get().is_none() {
        NUM_USERS.set(Arc::new(Mutex::new(0))).ok();
    }
}

pub fn principal_map_lock() -> tokio::sync::MutexGuard<'static, AHashMap<Principal, Vec<u8>>> {
    PRINCIPAL_MAP
        .get()
        .expect("PRINCIPAL_MAP not initialized")
        .blocking_lock()
}

pub fn num_providers_lock() -> tokio::sync::MutexGuard<'static, u64> {
    NUM_PROVIDERS
        .get()
        .expect("NUM_PROVIDERS not initialized")
        .blocking_lock()
}

pub fn num_users_lock() -> tokio::sync::MutexGuard<'static, u64> {
    NUM_USERS
        .get()
        .expect("NUM_USERS not initialized")
        .blocking_lock()
}

pub fn set_num_users(num_users: u64) {
    *num_users_lock() = num_users;
}

pub fn get_num_providers() -> u64 {
    *num_providers_lock()
}

pub fn num_providers_set(num_providers: u64) {
    *num_providers_lock() = num_providers;
}

pub fn inc_num_providers() {
    *num_providers_lock() += 1;
}

pub fn inc_num_users() {
    *num_users_lock() += 1;
}

pub fn get_pubkey_from_principal(principal: Principal) -> Vec<u8> {
    principal_map_lock()
        .get(&principal)
        .cloned()
        .unwrap_or_default()
}

pub fn set_pubkey_for_principal(principal: Principal, pubkey_bytes: Vec<u8>) {
    principal_map_lock().insert(principal, pubkey_bytes);
}

pub fn account_registration_fee_e9s() -> TokenAmountE9s {
    reward_e9s_per_block() / 100
}

#[named]
pub fn do_account_register(
    ledger: &mut LedgerMap,
    label: &str,
    pubkey_bytes: Vec<u8>,
    crypto_signature_bytes: Vec<u8>,
) -> Result<String, String> {
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes).unwrap();
    dcc_id.verify_bytes(&pubkey_bytes, &crypto_signature_bytes)?;
    fn_info!("{}", dcc_id);

    let fees = if ledger.get_blocks_count() > 0 {
        let amount = account_registration_fee_e9s();
        info!(
            "Charging {} tokens from {} for account {} registration",
            amount_as_string(amount),
            dcc_id.to_ic_principal(),
            label
        );
        charge_fees_to_account_no_bump_reputation(
            ledger,
            &dcc_id,
            amount as TokenAmountE9s,
            format!(
                "register-{}",
                dcc_id
                    .to_ic_principal()
                    .to_string()
                    .split_once('-')
                    .expect("Invalid principal")
                    .0
            )
            .as_str(),
        )?;
        amount
    } else {
        0
    };

    // Update the cache of principal -> pubkey, for quick search
    set_pubkey_for_principal(dcc_id.to_ic_principal(), pubkey_bytes.clone());

    if label == LABEL_USER_REGISTER {
        inc_num_users();
    } else if label == LABEL_NP_REGISTER {
        inc_num_providers();
    }

    // Store the pubkey in the ledger
    ledger
        .upsert(label, pubkey_bytes, crypto_signature_bytes)
        .map(|_| {
            format!(
                "Registration complete! Thank you. You have been charged {} tokens",
                amount_as_string(fees)
            )
        })
        .map_err(|e| e.to_string())
}
