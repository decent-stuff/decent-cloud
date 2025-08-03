use crate::{
    amount_as_string, charge_fees_to_account_no_bump_reputation, fn_info, reward_e9s_per_block,
    warn, AHashMap, DccIdentity, TokenAmountE9s, LABEL_NP_OFFERING, MAX_NP_OFFERING_BYTES,
};
use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
#[allow(unused_imports)]
use ic_cdk::println;
use ledger_map::LedgerMap;
use np_offering::ProviderOfferings;
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
pub struct UpdateOfferingsPayloadV1 {
    pub offerings_payload: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize, Clone, PartialEq, Eq, Hash)]
pub enum UpdateOfferingsPayload {
    V1(UpdateOfferingsPayloadV1),
}

impl UpdateOfferingsPayload {
    pub fn new(offerings_payload: &[u8], crypto_signature_bytes: &[u8]) -> Result<Self, String> {
        if offerings_payload.len() > MAX_NP_OFFERING_BYTES {
            return Err("Offering payload too long".to_string());
        }
        Ok(UpdateOfferingsPayload::V1(UpdateOfferingsPayloadV1 {
            offerings_payload: offerings_payload.to_vec(),
            signature: crypto_signature_bytes.to_vec(),
        }))
    }

    pub fn payload_serialized(&self) -> &[u8] {
        match self {
            UpdateOfferingsPayload::V1(payload) => payload.offerings_payload.as_slice(),
        }
    }

    pub fn deserialize(data: &[u8]) -> Result<UpdateOfferingsPayload, String> {
        Self::try_from_slice(data).map_err(|e| e.to_string())
    }

    pub fn deserialize_offerings(
        &self,
        provider_pubkey: &[u8],
    ) -> Result<ProviderOfferings, String> {
        let csv_data = String::from_utf8(self.payload_serialized().to_vec())
            .map_err(|e| format!("Invalid UTF-8 data: {}", e))?;
        np_offering::ProviderOfferings::new_from_str(provider_pubkey, &csv_data)
            .map_err(|e| format!("CSV parsing error: {}", e))
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

    let payload = UpdateOfferingsPayload::new(&offering_serialized, &crypto_signature_bytes)?;
    let payload_bytes = borsh::to_vec(&payload).unwrap();

    let num_offering_instances = payload
        .deserialize_offerings(dcc_id.to_bytes_verifying().as_slice())
        .map(|o| o.get_all_instance_ids().len())
        .unwrap_or(0);

    set_offering_num_per_provider(pubkey_bytes.clone(), num_offering_instances as u64);

    let fees = np_offering_update_fee_e9s();
    charge_fees_to_account_no_bump_reputation(
        ledger,
        &dcc_id.as_icrc_compatible_account(),
        fees,
        "update-offering",
    )?;
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
) -> Vec<ProviderOfferings> {
    let mut results: Vec<ProviderOfferings> = vec![];

    let search_filter = search_filter.trim();

    for entry in ledger
        .iter(Some(LABEL_NP_OFFERING))
        .chain(ledger.next_block_iter(Some(LABEL_NP_OFFERING)))
    {
        // Only the latest ProviderOfferings entry is returned in the iterator per provider
        let payload_decoded = match UpdateOfferingsPayload::deserialize(entry.value()) {
            Ok(payload) => payload,
            Err(e) => {
                warn!("Error decoding payload: {}", e);
                continue;
            }
        };
        match payload_decoded.deserialize_offerings(entry.key()) {
            Ok(offering) => {
                if search_filter.is_empty() || !offering.matches_search(search_filter).is_empty() {
                    results.push(offering);
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
