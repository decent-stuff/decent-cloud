use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, info, reward_e9s_per_block, warn,
    Balance, DccIdentity, ED25519_SIGNATURE_LENGTH, LABEL_NP_OFFERING, MAX_NP_OFFERING_BYTES,
    MAX_PUBKEY_BYTES,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use candid::Principal;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use np_offering::Offering;

fn np_offering_update_fee_e9s() -> Balance {
    reward_e9s_per_block() / 10000
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub struct UpdateOfferingPayloadV1 {
    pub offering_payload: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub enum UpdateOfferingPayload {
    V1(UpdateOfferingPayloadV1),
}

impl UpdateOfferingPayload {
    pub fn new_signed(offering: &Offering, dcc_id: &DccIdentity) -> Self {
        let enc_bytes = borsh::to_vec(&offering).unwrap();
        let signature = dcc_id.sign(&enc_bytes).unwrap();
        UpdateOfferingPayload::V1(UpdateOfferingPayloadV1 {
            offering_payload: enc_bytes,
            signature: signature.to_vec(),
        })
    }

    pub fn verify_signature(&self, dcc_id: &DccIdentity) -> Result<(), String> {
        match self {
            UpdateOfferingPayload::V1(payload) => {
                if payload.signature.len() != ED25519_SIGNATURE_LENGTH {
                    return Err("Invalid signature".to_string());
                }
                if payload.offering_payload.len() > MAX_NP_OFFERING_BYTES {
                    return Err("Offering payload too long".to_string());
                }
                dcc_id
                    .verify_bytes(&payload.offering_payload, &payload.signature)
                    .map_err(|e| e.to_string())
            }
        }
    }

    pub fn deserialize_unchecked(data: &[u8]) -> Result<UpdateOfferingPayload, String> {
        UpdateOfferingPayload::try_from_slice(data).map_err(|e| e.to_string())
    }

    pub fn deserialize_checked(
        dcc_id: &DccIdentity,
        data: &[u8],
    ) -> Result<UpdateOfferingPayload, String> {
        let result = Self::deserialize_unchecked(data)?;
        result.verify_signature(dcc_id).map_err(|e| e.to_string())?;
        Ok(result)
    }

    pub fn offering(&self) -> Result<Offering, String> {
        match self {
            UpdateOfferingPayload::V1(payload) => {
                Offering::try_from_slice(&payload.offering_payload)
                    .map(|v| v.compute_json_value())
                    .map_err(|e| e.to_string())
            }
        }
    }
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
    info!(
        "[do_node_provider_update_offering]: {} => {} bytes",
        dcc_identity,
        update_offering_payload.len()
    );

    UpdateOfferingPayload::deserialize_checked(&dcc_identity, update_offering_payload)?;

    let fees = np_offering_update_fee_e9s();
    charge_fees_to_account_no_bump_reputation(ledger, &dcc_identity, fees)?;
    // Store the original signed payload in the ledger
    ledger
        .upsert(LABEL_NP_OFFERING, &pubkey_bytes, update_offering_payload)
        .map(|_| {
            format!(
                "Offering updated! Thank you. You have been charged {} tokens",
                amount_as_string(fees)
            )
        })
        .map_err(|e| e.to_string())
}

/// Search for offerings that match the given filter
/// If the filter is empty, return all offerings
pub fn do_get_matching_offerings(
    ledger: &LedgerMap,
    search_filter: &str,
) -> Vec<(DccIdentity, Offering)> {
    let mut results = vec![];

    let search_filter = search_filter.trim();

    for entry in ledger
        .iter(Some(LABEL_NP_OFFERING))
        .chain(ledger.next_block_iter(Some(LABEL_NP_OFFERING)))
    {
        let dcc_id = match DccIdentity::new_verifying_from_bytes(entry.key()) {
            Ok(dcc_id) => dcc_id,
            Err(e) => {
                warn!(
                    "Error decoding public key {}: {}",
                    BASE64.encode(entry.key()),
                    e
                );
                continue;
            }
        };
        let payload_decoded =
            UpdateOfferingPayload::deserialize_checked(&dcc_id, entry.value()).unwrap();
        match payload_decoded.offering() {
            Ok(offering) => {
                if search_filter.is_empty() || offering.matches_search(search_filter) {
                    results.push((dcc_id, offering));
                }
            }
            Err(e) => {
                warn!("Error decoding offering: {}", e);
                continue;
            }
        }
    }

    results
}
