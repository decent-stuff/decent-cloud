use crate::{
    amount_as_string, charge_fees_to_account_and_bump_reputation, fn_info, info,
    reward_e9s_per_block, AHashMap, DccIdentity, TokenAmount,
};
use candid::Principal;
use function_name::named;
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

pub fn account_registration_fee_e9s() -> TokenAmount {
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
        charge_fees_to_account_and_bump_reputation(
            ledger,
            &dcc_id,
            &dcc_id,
            amount as TokenAmount,
        )?;
        amount
    } else {
        0
    };

    // Update the cache of principal -> pubkey, for quick search
    PRINCIPAL_MAP.with(|p| {
        p.borrow_mut()
            .insert(dcc_id.to_ic_principal(), pubkey_bytes.clone())
    });

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
