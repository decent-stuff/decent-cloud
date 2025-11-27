#[cfg(all(target_arch = "wasm32", feature = "ic"))]
use ic_cdk::println;
use std::{cell::RefCell, collections::HashMap, io::Error};

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::{AHashMap, DccIdentity, TokenAmountE9s};

pub type ContractId = Vec<u8>;

thread_local! {
    /// Key is a 32-byte contract id
    /// Value is a ContractSignRequest
    static CONTRACTS_CACHE_OPEN: RefCell<AHashMap<Vec<u8>, ContractSignRequest>> = RefCell::new(HashMap::default());
}

pub fn contracts_cache_get_open_for_provider(
    filter_provider_pubkey_bytes: Option<Vec<u8>>,
) -> Vec<(Vec<u8>, ContractSignRequest)> {
    CONTRACTS_CACHE_OPEN.with(|contracts| {
        contracts
            .borrow()
            .iter()
            .filter(move |(_, req)| match &filter_provider_pubkey_bytes {
                None => true, // No filter ==> include all entries
                Some(filter_provider_pubkey_bytes) => {
                    req.provider_pubkey_bytes() == filter_provider_pubkey_bytes
                }
            })
            .map(|(key, req)| (key.clone(), req.clone()))
            .collect()
    })
}

pub fn contracts_cache_open_add(contract_id: Vec<u8>, req: ContractSignRequest) {
    CONTRACTS_CACHE_OPEN.with(|contracts| {
        contracts.borrow_mut().insert(contract_id, req);
    })
}

pub fn contracts_cache_open_remove(contract_id: &[u8]) {
    CONTRACTS_CACHE_OPEN.with(|contracts| {
        contracts.borrow_mut().remove(contract_id);
    })
}

pub fn contract_sign_fee_e9s(contract_value_e9s: TokenAmountE9s) -> TokenAmountE9s {
    contract_value_e9s / 100
}

// Main struct for Offering Request
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractSignRequest {
    V1(ContractSignRequestV1),
}

// Custom serializer for public keys
fn serialize_pubkey<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let dcc_id = DccIdentity::new_verifying_from_bytes(bytes).unwrap();
    serializer.serialize_str(&dcc_id.verifying_key_as_pem_one_line())
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct PaymentEntry {
    pub pricing_model: String,    // E.g. on_demand, reserved, ...
    pub time_period_unit: String, // E.g. hour, day
    pub quantity: u64,            // number of units
}

impl PaymentEntry {
    pub fn new<S: ToString>(pricing_model: S, period: S, quantity: u64) -> Self {
        PaymentEntry {
            pricing_model: pricing_model.to_string(),
            time_period_unit: period.to_string(),
            quantity,
        }
    }
}

// This struct is added to work around a clap issue.
// Clap needs one value produced by a value_parser to avoid the following mismatch:
// Mismatch between definition and access of `payment_entries_json`. Could not downcast to dcc_common::contract_sign_request::PaymentEntry, need to downcast to alloc::vec::Vec<dcc_common::contract_sign_request::PaymentEntry>
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct PaymentEntries(pub Vec<PaymentEntry>);

// Struct for preparing payment on the CLI, which makes it easier to calculate the total
// amount
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct PaymentEntryWithAmount {
    #[serde(flatten)]
    pub e: PaymentEntry,
    pub amount_e9s: TokenAmountE9s, // total amount
}

// Struct for requesting a contract signature, version 1. Future versions can be added below
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ContractSignRequestV1 {
    #[serde(serialize_with = "serialize_pubkey")]
    requester_pubkey: Vec<u8>, // Who is making this request?
    requester_ssh_pubkey: String, // The ssh key that will be given access to the instance, preferably in ed25519 key format https://en.wikipedia.org/wiki/Ssh-keygen
    requester_contact: String,    // Where can the requester be contacted by the provider, if needed
    #[serde(serialize_with = "serialize_pubkey")]
    provider_pubkey: Vec<u8>, // To which provider is this targeted?
    offering_id: String,          // Requester would like to contract this particular offering id
    region_name: Option<String>,  // Optional region name
    contract_id: Option<String>,  // Optional contract id, if an existing contract is being extended
    instance_config: Option<String>, // Optional configuration for the instance deployment, e.g. cloud-init
    payment_amount_e9s: TokenAmountE9s, // How much is the requester offering to pay for the contract
    payment_entries: Vec<PaymentEntryWithAmount>,
    start_timestamp: Option<u64>, // Optionally, only start contract at this unix time (in seconds) UTC. This can be in the past or in the future. Default is now.
    request_memo: String, // Reference to this particular request; arbitrary text. Can be used e.g. for administrative purposes
}

impl ContractSignRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        requester_pubkey_bytes: &[u8],
        requester_ssh_pubkey: String,
        requester_contact: String,
        provider_pubkey_bytes: &[u8],
        offering_id: String,
        region_name: Option<String>,
        contract_id: Option<String>,
        instance_config: Option<String>,
        payment_amount_e9s: TokenAmountE9s,
        payment_entries: Vec<PaymentEntryWithAmount>,
        start_timestamp: Option<u64>,
        request_memo: String,
    ) -> Self {
        ContractSignRequest::V1(ContractSignRequestV1 {
            requester_pubkey: requester_pubkey_bytes.to_vec(),
            requester_ssh_pubkey,
            requester_contact,
            provider_pubkey: provider_pubkey_bytes.to_vec(),
            offering_id,
            region_name,
            contract_id,
            instance_config,
            payment_amount_e9s,
            payment_entries,
            start_timestamp,
            request_memo,
        })
    }

    pub fn payment_amount_e9s(&self) -> u64 {
        match self {
            ContractSignRequest::V1(v1) => v1.payment_amount_e9s,
        }
    }

    pub fn requester_pubkey_bytes(&self) -> &[u8] {
        match self {
            ContractSignRequest::V1(v1) => &v1.requester_pubkey,
        }
    }

    pub fn requester_ssh_pubkey(&self) -> &String {
        match self {
            ContractSignRequest::V1(v1) => &v1.requester_ssh_pubkey,
        }
    }

    pub fn requester_contact(&self) -> &String {
        match self {
            ContractSignRequest::V1(v1) => &v1.requester_contact,
        }
    }

    pub fn provider_pubkey_bytes(&self) -> &[u8] {
        match self {
            ContractSignRequest::V1(v1) => &v1.provider_pubkey,
        }
    }

    pub fn offering_id(&self) -> &String {
        match self {
            ContractSignRequest::V1(v1) => &v1.offering_id,
        }
    }

    pub fn contract_id(&self) -> Option<&String> {
        match self {
            ContractSignRequest::V1(v1) => v1.contract_id.as_ref(),
        }
    }

    pub fn instance_config(&self) -> Option<&String> {
        match self {
            ContractSignRequest::V1(v1) => v1.instance_config.as_ref(),
        }
    }

    pub fn contract_start_timestamp(&self) -> Option<u64> {
        match self {
            ContractSignRequest::V1(v1) => v1.start_timestamp,
        }
    }

    pub fn request_memo(&self) -> &String {
        match self {
            ContractSignRequest::V1(v1) => &v1.request_memo,
        }
    }

    pub fn region_name(&self) -> Option<&String> {
        match self {
            ContractSignRequest::V1(v1) => v1.region_name.as_ref(),
        }
    }

    pub fn payment_entries(&self) -> &Vec<PaymentEntryWithAmount> {
        match self {
            ContractSignRequest::V1(v1) => &v1.payment_entries,
        }
    }
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub struct ContractSignRequestPayloadV1 {
    payload_serialized: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum ContractSignRequestPayload {
    V1(ContractSignRequestPayloadV1),
}

impl ContractSignRequestPayload {
    pub fn new(payload_serialized: &[u8], crypto_sig: &[u8]) -> Result<Self, String> {
        Ok(ContractSignRequestPayload::V1(
            ContractSignRequestPayloadV1 {
                payload_serialized: payload_serialized.to_vec(),
                signature: crypto_sig.to_vec(),
            },
        ))
    }

    pub fn payload_serialized(&self) -> &[u8] {
        match self {
            ContractSignRequestPayload::V1(v1) => v1.payload_serialized.as_slice(),
        }
    }

    pub fn deserialize_contract_sign_request(&self) -> Result<ContractSignRequest, Error> {
        ContractSignRequest::try_from_slice(self.payload_serialized())
    }

    pub fn calc_contract_id(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.payload_serialized());
        hasher.finalize().into()
    }
}
