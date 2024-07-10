use std::collections::BTreeMap;

use crate::{
    charge_fees_to_account_no_bump_reputation, reward_e9s_per_block, slice_to_32_bytes_array,
    zlib_decompress, DccIdentity, LABEL_NP_PROFILE, LABEL_NP_REGISTER,
};
use candid::Principal;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use serde::{Deserialize, Serialize};

pub fn operation_fee_e9s() -> u64 {
    reward_e9s_per_block() / 10000
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct NodeProviderProfile {
    pub name: String,
    pub description: String,
    pub url: String,
    pub logo_url: String,
    pub why_choose_us: String,
    pub locations: BTreeMap<String, String>,
    pub contacts: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct UpdateProfilePayload {
    pub profile_payload: Vec<u8>,
    pub signature: Vec<u8>,
}

// To prevent DOS attacks, the NP is charged 1/100 of the block reward for executing this operation
pub fn do_node_provider_update_profile(
    ledger: &mut LedgerMap,
    caller: Principal,
    np_unique_id: Vec<u8>,
    update_profile_payload: Vec<u8>,
) -> Result<String, String> {
    println!(
        "[do_node_provider_update_profile]: caller {} np_unique_id {}",
        caller,
        String::from_utf8_lossy(&np_unique_id)
    );
    if np_unique_id.len() > 64 {
        return Err("Node provider unique id too long".to_string());
    }
    let payload: UpdateProfilePayload =
        serde_json::from_slice(&update_profile_payload).map_err(|e| e.to_string())?;

    if payload.signature.len() != 64 {
        return Err("Invalid signature".to_string());
    }
    if payload.profile_payload.len() > 1024 {
        return Err("Profile payload too long".to_string());
    }

    match ledger.get(LABEL_NP_REGISTER, &np_unique_id) {
        Ok(np_key) => {
            // Check the signature
            let pub_key_bytes = slice_to_32_bytes_array(&np_key)?;
            let dcc_identity =
                DccIdentity::new_verifying_from_bytes(pub_key_bytes).map_err(|e| e.to_string())?;

            match dcc_identity.verify_bytes(&payload.profile_payload, &payload.signature) {
                Ok(()) => {
                    charge_fees_to_account_no_bump_reputation(
                        ledger,
                        &dcc_identity,
                        operation_fee_e9s(),
                    )?;
                    ledger
                        .upsert(LABEL_NP_PROFILE, &np_unique_id, &update_profile_payload)
                        .map(|_| "Profile updated!".to_string())
                        .map_err(|e| e.to_string())
                }
                Err(e) => Err(format!("Signature is invalid: {:?}", e)),
            }
        }
        Err(ledger_map::LedgerError::EntryNotFound) => Err("Node provider not found".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

pub fn do_node_provider_get_profile(ledger: &LedgerMap, np_unique_id: Vec<u8>) -> Option<String> {
    match ledger.get(LABEL_NP_PROFILE, &np_unique_id) {
        Ok(profile) => {
            let payload: UpdateProfilePayload = serde_json::from_slice(&profile).unwrap();
            zlib_decompress(&payload.profile_payload).ok()
        }
        Err(_) => None,
    }
}
