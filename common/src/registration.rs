use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, fn_info, info,
    reward_e9s_per_block, AHashMap, DccIdentity, TokenAmountE9s, LABEL_PROV_REGISTER,
    LABEL_USER_REGISTER,
};
use candid::Principal;
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    pub static PRINCIPAL_MAP: RefCell<AHashMap<Principal, Vec<u8>>> = RefCell::new(HashMap::default());
    pub static NUM_PROVIDERS: RefCell<u64> = const { RefCell::new(0) };
    pub static NUM_USERS: RefCell<u64> = const { RefCell::new(0) };
}

pub fn get_num_users() -> u64 {
    NUM_USERS.with(|n| *n.borrow())
}

pub fn set_num_users(num_users: u64) {
    NUM_USERS.with(|n| *n.borrow_mut() = num_users);
}

pub fn get_num_providers() -> u64 {
    NUM_PROVIDERS.with(|n| *n.borrow())
}

pub fn set_num_providers(num_providers: u64) {
    NUM_PROVIDERS.with(|n| *n.borrow_mut() = num_providers);
}

pub fn inc_num_providers() {
    NUM_PROVIDERS.with(|n| *n.borrow_mut() += 1);
}

pub fn inc_num_users() {
    NUM_USERS.with(|n| *n.borrow_mut() += 1);
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

pub fn set_pubkey_for_principal(principal: Principal, pubkey_bytes: Vec<u8>) {
    PRINCIPAL_MAP.with(|principal_map| {
        principal_map.borrow_mut().insert(principal, pubkey_bytes);
    })
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
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes)?;
    dcc_id.verify_bytes(&pubkey_bytes, &crypto_signature_bytes)?;
    fn_info!("{}", dcc_id);

    let principal = dcc_id.to_ic_principal()?;
    let fees = if ledger.get_blocks_count() > 0 {
        let amount = account_registration_fee_e9s();
        info!(
            "Charging {} tokens from {} for account {} registration",
            amount_as_string(amount),
            principal,
            label
        );
        let principal_short = principal
            .to_string()
            .split_once('-')
            .map(|(first, _)| first.to_string())
            .ok_or_else(|| format!("Invalid principal: {principal}"))?;
        charge_fees_to_account_no_bump_reputation(
            ledger,
            &dcc_id.as_icrc_compatible_account()?,
            amount as TokenAmountE9s,
            &format!("register-{principal_short}"),
        )?;
        amount
    } else {
        0
    };

    // Update the cache of principal -> pubkey, for quick search
    set_pubkey_for_principal(principal, pubkey_bytes.clone());

    if label == LABEL_USER_REGISTER {
        inc_num_users();
    } else if label == LABEL_PROV_REGISTER {
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
