use borsh::{BorshDeserialize, BorshSerialize};
#[cfg(all(target_arch = "wasm32", feature = "ic"))]
use ic_cdk::println;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ContractSignReplyV1 {
    requester_pubkey_bytes: Vec<u8>, // Public key of the original requester
    request_memo: String,            // Memo field of the original request
    contract_id: Vec<u8>,            // Contract ID of the request that we are replying to
    sign_accepted: bool, // True/False to mark whether the signing was accepted or rejected by the provider
    response_text: String, // Thank you note, or similar on success. Reason the request failed on failure.
    response_details: String, // Instructions or a link to the detailed instructions: describing next steps, further information, etc.
}

// Main struct for Offering Request
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractSignReply {
    V1(ContractSignReplyV1),
}

impl ContractSignReply {
    pub fn new<S: ToString>(
        requester_pubkey_bytes: Vec<u8>,
        request_memo: S,
        contract_id: Vec<u8>,
        sign_accepted: bool,
        response_text: S,
        response_details: S,
    ) -> Self {
        ContractSignReply::V1(ContractSignReplyV1 {
            requester_pubkey_bytes,
            request_memo: request_memo.to_string(),
            contract_id,
            sign_accepted,
            response_text: response_text.to_string(),
            response_details: response_details.to_string(),
        })
    }

    pub fn contract_id(&self) -> &[u8] {
        match self {
            ContractSignReply::V1(payload) => payload.contract_id.as_slice(),
        }
    }
    pub fn requester_pubkey_bytes(&self) -> &[u8] {
        match self {
            ContractSignReply::V1(payload) => payload.requester_pubkey_bytes.as_slice(),
        }
    }
    pub fn request_memo(&self) -> &String {
        match self {
            ContractSignReply::V1(payload) => &payload.request_memo,
        }
    }
    pub fn sign_accepted(&self) -> bool {
        match self {
            ContractSignReply::V1(payload) => payload.sign_accepted,
        }
    }
    pub fn response_text(&self) -> &String {
        match self {
            ContractSignReply::V1(payload) => &payload.response_text,
        }
    }
    pub fn response_details(&self) -> &String {
        match self {
            ContractSignReply::V1(payload) => &payload.response_details,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct ContractSignReplyPayloadV1 {
    payload_serialized: Vec<u8>,
    crypto_signature: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ContractSignReplyPayload {
    V1(ContractSignReplyPayloadV1),
}

impl ContractSignReplyPayload {
    pub fn new(payload_serialized: Vec<u8>, crypto_signature: Vec<u8>) -> ContractSignReplyPayload {
        ContractSignReplyPayload::V1(ContractSignReplyPayloadV1 {
            payload_serialized,
            crypto_signature,
        })
    }

    pub fn payload_serialized(&self) -> &[u8] {
        match self {
            ContractSignReplyPayload::V1(payload) => payload.payload_serialized.as_slice(),
        }
    }

    pub fn crypto_signature(&self) -> &[u8] {
        match self {
            ContractSignReplyPayload::V1(payload) => payload.crypto_signature.as_slice(),
        }
    }

    pub fn deserialize_contract_sign_reply(&self) -> Result<ContractSignReply, String> {
        ContractSignReply::try_from_slice(self.payload_serialized()).map_err(|e| e.to_string())
    }
}
