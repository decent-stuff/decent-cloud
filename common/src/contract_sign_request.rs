use std::io::Error;

use borsh::{BorshDeserialize, BorshSerialize};
use function_name::named;
use ledger_map::LedgerMap;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    amount_as_string, charge_fees_to_account_and_bump_reputation, fn_info, DccIdentity,
    TokenAmount, LABEL_CONTRACT_SIGN_REQUEST,
};

pub fn contract_sign_fee_e9s(contract_value: TokenAmount) -> TokenAmount {
    contract_value / 100
}

// Main struct for Offering Request
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractSignRequest {
    V1(ContractSignRequestV1),
}

// Struct for requesting a contract signature, version 1. Future versions can be added below
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ContractSignRequestV1 {
    requester_pubkey_bytes: Vec<u8>, // Who is making this request?
    requester_ssh_pubkey: String, // The ssh key that will be given access to the instance, preferably in ed25519 key format https://en.wikipedia.org/wiki/Ssh-keygen
    requester_contact: String,    // Where can the requester be contacted by the provider, if needed
    provider_pubkey_bytes: Vec<u8>, // To which provider is this targeted?
    offering_id: String,          // Requester would like to contract this particular offering id
    region_name: Option<String>,  // Optional region name
    instance_id: Option<String>, // Optional instance id that can be provided to alter the particular instance that requester already controls
    instance_config: Option<String>, // Optional configuration for the instance deployment, e.g. cloud-init
    payment_amount: u64,             // How much is the requester offering to pay for the contract
    duration_seconds: u64, // For how many SECONDS would the requester like to sign the contract; 1 hour = 3600 seconds, 1 day = 86400 seconds
    start_timestamp: Option<u64>, // Optionally, only start contract at this unix time (in seconds) UTC. This can be in the past or in the future. Default is now.
    request_memo: String, // Reference to this particular request; arbitrary text. Can be used e.g. for administrative purposes
}

impl ContractSignRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        requester_pubkey_bytes: Vec<u8>,
        requester_ssh_pubkey: String,
        requester_contact: String,
        provider_pubkey_bytes: &[u8],
        offering_id: String,
        region_name: Option<String>,
        instance_id: Option<String>,
        instance_config: Option<String>,
        payment_amount: u64,
        duration_seconds: u64,
        start_timestamp: Option<u64>,
        request_memo: String,
    ) -> Self {
        ContractSignRequest::V1(ContractSignRequestV1 {
            requester_pubkey_bytes,
            requester_ssh_pubkey,
            requester_contact,
            provider_pubkey_bytes: provider_pubkey_bytes.to_vec(),
            offering_id,
            region_name,
            instance_id,
            instance_config,
            payment_amount,
            duration_seconds,
            start_timestamp,
            request_memo,
        })
    }

    pub fn payment_amount(&self) -> u64 {
        match self {
            ContractSignRequest::V1(v1) => v1.payment_amount,
        }
    }

    pub fn requester_pubkey_bytes(&self) -> &[u8] {
        match self {
            ContractSignRequest::V1(v1) => &v1.requester_pubkey_bytes,
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
            ContractSignRequest::V1(v1) => &v1.provider_pubkey_bytes,
        }
    }

    pub fn offering_id(&self) -> &String {
        match self {
            ContractSignRequest::V1(v1) => &v1.offering_id,
        }
    }

    pub fn instance_id(&self) -> Option<&String> {
        match self {
            ContractSignRequest::V1(v1) => v1.instance_id.as_ref(),
        }
    }

    pub fn instance_config(&self) -> Option<&String> {
        match self {
            ContractSignRequest::V1(v1) => v1.instance_config.as_ref(),
        }
    }

    pub fn rent_period_seconds(&self) -> u64 {
        match self {
            ContractSignRequest::V1(v1) => v1.duration_seconds,
        }
    }

    pub fn rent_start_timestamp(&self) -> Option<u64> {
        match self {
            ContractSignRequest::V1(v1) => v1.start_timestamp,
        }
    }

    pub fn request_memo(&self) -> &String {
        match self {
            ContractSignRequest::V1(v1) => &v1.request_memo,
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

#[named]
pub fn do_contract_sign_request(
    ledger: &mut LedgerMap,
    pubkey_bytes: Vec<u8>,
    request_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
) -> Result<String, String> {
    let dcc_id = DccIdentity::new_verifying_from_bytes(&pubkey_bytes).unwrap();
    dcc_id.verify_bytes(&request_serialized, &crypto_signature)?;

    fn_info!("{}", dcc_id);

    let contract_req = ContractSignRequest::try_from_slice(&request_serialized).unwrap();

    let fees = contract_sign_fee_e9s(contract_req.payment_amount());

    let payload = ContractSignRequestPayload::new(&request_serialized, &crypto_signature).unwrap();
    let payload_bytes = borsh::to_vec(&payload).unwrap();

    charge_fees_to_account_and_bump_reputation(ledger, &dcc_id, &dcc_id, fees)?;
    let contract_id = payload.calc_contract_id();

    ledger.upsert(
        LABEL_CONTRACT_SIGN_REQUEST,
        contract_id,
        payload_bytes,
    ).map(|_| {
        format!(
            "Contract signing req 0x{} submitted! Thank you. You have been charged {} tokens as a fee, and your reputation has been bumped accordingly. Please wait for a response.",
            hex::encode(contract_id),
            amount_as_string(fees)
        )
    }).map_err(|e| e.to_string())
}
