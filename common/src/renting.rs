use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

use crate::{
    // amount_as_string, charge_fees_to_account_no_bump_reputation, info, reward_e9s_per_block, warn,
    // Balance, DccIdentity, ED25519_SIGNATURE_LENGTH, LABEL_NP_OFFERING, MAX_NP_OFFERING_BYTES,
    // MAX_PUBKEY_BYTES,
    DccIdentity,
};

// Main struct for Offering Request
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum OfferingRequest {
    V1(OfferingRequestV1),
}

// Struct for Offering Request version 1, other versions can be added below
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct OfferingRequestV1 {
    #[serde(skip, default)]
    #[borsh(skip)]
    requester_dcc_id: DccIdentity, // Who is making this rent request?
    requester_ssh_pubkey: String, // The ssh key that will be given access to the instance, preferably in ed25519 key format https://en.wikipedia.org/wiki/Ssh-keygen
    requester_contact: String,    // Where can the requester be contacted by the provider, if needed
    provider_pubkey_bytes: Vec<u8>, // To which provider is this targeted?
    offering_id: String,          // Requester would like to rent this particular offering id
    instance_id: Option<String>, // Optional instance id that can be provided to alter the particular instance a requester already controls
    instance_config: Option<String>, // Optional configuration for the rented instance, e.g. cloud-init
    payment_amount: u64, // How much is the requester offering to pay for renting the resource
    rent_period_seconds: u64, // For how many SECONDS would the requester like to rent the resource; 1 hour = 3600 seconds, 1 day = 86400 seconds
    rent_start_timestamp: Option<u64>, // Optionally, only start renting at this unix time (in seconds) UTC. This can be in the future.
    request_memo: String, // Reference to this particular request; arbitrary text. Can be used e.g. for administrative purposes
}

impl OfferingRequest {
    pub fn new(
        requester_dcc_id: DccIdentity,
        requester_ssh_pubkey: String,
        requester_contact: String,
        provider_pubkey_bytes: &[u8],
        offering_id: String,
        instance_id: Option<String>,
        instance_config: Option<String>,
        payment_amount: u64,
        rent_period_seconds: u64,
        rent_start_timestamp: Option<u64>,
        request_memo: String,
    ) -> Self {
        OfferingRequest::V1(OfferingRequestV1 {
            requester_dcc_id,
            requester_ssh_pubkey,
            requester_contact,
            provider_pubkey_bytes: provider_pubkey_bytes.to_vec(),
            offering_id,
            instance_id,
            instance_config,
            payment_amount,
            rent_period_seconds,
            rent_start_timestamp,
            request_memo,
        })
    }

    pub fn to_payload_signed(&self) -> OfferingRequestPayload {
        OfferingRequestPayload::new(self)
    }

    pub fn requester_dcc_id(&self) -> &DccIdentity {
        match self {
            OfferingRequest::V1(request) => &request.requester_dcc_id,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OfferingRequestPayloadV1 {
    offering_request_bytes: Vec<u8>,
    signature: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OfferingRequestPayload {
    V1(OfferingRequestPayloadV1),
}

impl OfferingRequestPayload {
    pub fn new(offering_request: &OfferingRequest) -> Self {
        let offering_request_bytes = borsh::to_vec(&offering_request).unwrap();
        OfferingRequestPayload::V1(OfferingRequestPayloadV1 {
            offering_request_bytes: offering_request_bytes.clone(),
            signature: offering_request
                .requester_dcc_id()
                .sign(&offering_request_bytes)
                .unwrap()
                .to_vec(),
        })
    }
}
