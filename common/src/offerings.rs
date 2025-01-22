use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, fn_info, reward_e9s_per_block,
    warn, AHashMap, DccIdentity, TokenAmountE9s, LABEL_NP_OFFERING, MAX_NP_OFFERING_BYTES,
};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
#[cfg(target_arch = "wasm32")]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use np_offering::Offering;
use std::cell::RefCell;

thread_local! {
    pub static NUM_OFFERINGS_PER_PROVIDER: RefCell<AHashMap<Vec<u8>, u64>> = RefCell::new(AHashMap::default());
    pub static NUM_OFFERINGS_TOTAL: RefCell<u64> = const { RefCell::new(0) };
}

pub fn set_offering_num_per_provider(pubkey_bytes: Vec<u8>, num: u64) {
    NUM_OFFERINGS_PER_PROVIDER.with(|map| {
        map.borrow_mut().insert(pubkey_bytes, num);
        NUM_OFFERINGS_TOTAL.with(|total| *total.borrow_mut() = map.borrow().values().sum());
    });
}

pub fn get_num_offerings() -> u64 {
    NUM_OFFERINGS_TOTAL.with(|n| *n.borrow())
}

fn np_offering_update_fee_e9s() -> TokenAmountE9s {
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
    pub fn new(offering_payload: &[u8], crypto_signature_bytes: &[u8]) -> Result<Self, String> {
        if offering_payload.len() > MAX_NP_OFFERING_BYTES {
            return Err("Offering payload too long".to_string());
        }
        Ok(UpdateOfferingPayload::V1(UpdateOfferingPayloadV1 {
            offering_payload: offering_payload.to_vec(),
            signature: crypto_signature_bytes.to_vec(),
        }))
    }

    pub fn payload_serialized(&self) -> &[u8] {
        match self {
            UpdateOfferingPayload::V1(payload) => payload.offering_payload.as_slice(),
        }
    }

    pub fn deserialize_unchecked(data: &[u8]) -> Result<UpdateOfferingPayload, String> {
        UpdateOfferingPayload::try_from_slice(data).map_err(|e| e.to_string())
    }

    pub fn offering(&self) -> Result<Offering, String> {
        Offering::new_from_bytes(self.payload_serialized(), "json")
    }
}

#[named]
pub fn do_node_provider_update_offering(
    ledger: &mut LedgerMap,
    pubkey_bytes: Vec<u8>,
    offering_serialized: Vec<u8>,
    crypto_signature_bytes: Vec<u8>,
) -> Result<String, String> {
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes).unwrap();
    dcc_id.verify_bytes(&offering_serialized, &crypto_signature_bytes)?;
    fn_info!("{} => {} bytes", dcc_id, offering_serialized.len());

    let payload = UpdateOfferingPayload::new(&offering_serialized, &crypto_signature_bytes)?;
    let payload_bytes = borsh::to_vec(&payload).unwrap();

    let num_offering_instances = payload
        .offering()
        .map(|o| o.get_all_instance_ids().len())
        .unwrap_or(0);

    set_offering_num_per_provider(pubkey_bytes.clone(), num_offering_instances as u64);

    let fees = np_offering_update_fee_e9s();
    charge_fees_to_account_no_bump_reputation(ledger, &dcc_id, fees)?;
    // Store the original signed payload in the ledger
    ledger
        .upsert(LABEL_NP_OFFERING, &pubkey_bytes, payload_bytes)
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
        let payload_decoded = match UpdateOfferingPayload::deserialize_unchecked(entry.value()) {
            Ok(payload) => payload,
            Err(e) => {
                warn!("Error decoding payload: {}", e);
                continue;
            }
        };
        match payload_decoded.offering() {
            Ok(offering) => {
                if search_filter.is_empty() || !offering.matches_search(search_filter).is_empty() {
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
