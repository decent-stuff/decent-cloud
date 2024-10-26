use crate::{
    charge_fees_to_account_no_bump_reputation, info, reward_e9s_per_block, Balance, DccIdentity,
    ED25519_SIGNATURE_LENGTH, LABEL_NP_OFFERING, MAX_NP_OFFERING_BYTES, MAX_PUBKEY_BYTES,
};
use candid::Principal;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use np_offering::Offering;
use serde::{Deserialize, Serialize};

fn np_offering_update_fee_e9s() -> Balance {
    reward_e9s_per_block() / 10000
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct UpdateOfferingPayload {
    pub offering_payload: Vec<u8>,
    pub signature: Vec<u8>,
}

pub fn do_node_provider_update_offering(
    ledger: &mut LedgerMap,
    caller: Principal,
    pubkey_bytes: Vec<u8>,
    update_offering_payload: &[u8],
) -> Result<String, String> {
    if pubkey_bytes.len() > MAX_PUBKEY_BYTES {
        return Err("Provided public key too long".to_string());
    }

    let dcc_identity =
        DccIdentity::new_verifying_from_bytes(&pubkey_bytes).map_err(|e| e.to_string())?;
    if caller != dcc_identity.to_ic_principal() {
        return Err("Invalid caller".to_string());
    }
    info!("[do_node_provider_update_offering]: {}", dcc_identity);

    let payload: UpdateOfferingPayload =
        serde_json::from_slice(&update_offering_payload).map_err(|e| e.to_string())?;

    if payload.signature.len() != ED25519_SIGNATURE_LENGTH {
        return Err("Invalid signature".to_string());
    }
    if payload.offering_payload.len() > MAX_NP_OFFERING_BYTES {
        return Err("Offering payload too long".to_string());
    }

    match dcc_identity.verify_bytes(&payload.offering_payload, &payload.signature) {
        Ok(()) => {
            charge_fees_to_account_no_bump_reputation(
                ledger,
                &dcc_identity,
                np_offering_update_fee_e9s(),
            )?;
            // Store the original signed payload in the ledger
            ledger
                .upsert(LABEL_NP_OFFERING, &pubkey_bytes, &update_offering_payload)
                .map(|_| "Offering updated! Thank you.".to_string())
                .map_err(|e| e.to_string())
        }
        Err(e) => Err(format!("Signature is invalid: {:?}", e)),
    }
}

/// Search for offerings that match the given filter
pub fn do_get_matching_offerings(ledger: &LedgerMap, filter: String) -> Vec<Offering> {
    let mut results = vec![];

    for entry in ledger
        .iter(Some(LABEL_NP_OFFERING))
        .chain(ledger.next_block_iter(Some(LABEL_NP_OFFERING)))
    {
        let payload: UpdateOfferingPayload =
            serde_json::from_slice(entry.value()).expect("Failed to decode payload");
        let offering: Offering =
            serde_json::from_slice(&payload.offering_payload).expect("Failed to decode offering");

        // FIXME: filter
        results.push(offering);
    }

    results
}
